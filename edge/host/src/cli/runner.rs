//! The real `EngineRunner` — invokes the locally-installed `claude`/`codex`
//! CLIs under the engineer's subscription session (FR-005/006), inside the repo
//! so installed skills/agents/plugins are in scope (FR-007).
//!
//! The argv builder is pure and unit-tested (it is the verifiable surface for
//! SC-002: no API-key flag, real CLI invocation). The spawn path is covered by
//! the driver integration test.

use crate::events::{map_claude_line, map_codex_line, CliSignal, Faction};
use crate::orchestrator::engine::{EngineOutcome, EngineRunner, Role};
use async_trait::async_trait;
use std::path::PathBuf;

/// The MCP permission-prompt tool name Claude calls to gate tool use. Matches
/// the server name (`gate`) registered in [`gate_mcp_config`].
pub const GATE_PROMPT_TOOL: &str = "mcp__gate__approve";

/// Wiring for the US2 permission gate: the `--mcp-config` JSON registering the
/// gate MCP server (which POSTs to the loopback permission server) and the
/// `--permission-prompt-tool` name Claude routes gated tool use to.
#[derive(Clone)]
pub struct GateConfig {
    pub mcp_config_json: String,
    pub prompt_tool: String,
}

/// Build the inline `--mcp-config` JSON registering the gate as a stdio MCP
/// server. The gate script reads `CONSTRUCT_GATE_URL` to reach the app's loopback
/// permission server and `CONSTRUCT_GATE_TOKEN` to authenticate to it — the
/// per-run secret the server requires on every request (M2).
pub fn gate_mcp_config(gate_script_path: &str, gate_url: &str, gate_token: &str) -> String {
    serde_json::json!({
        "mcpServers": {
            "gate": {
                "command": "node",
                "args": [gate_script_path],
                "env": {
                    "CONSTRUCT_GATE_URL": gate_url,
                    "CONSTRUCT_GATE_TOKEN": gate_token
                }
            }
        }
    })
    .to_string()
}

pub struct CliEngineRunner {
    program: String,
    faction: Faction,
    cwd: PathBuf,
    mapper: fn(&str) -> CliSignal,
    /// Present only for Claude when US2 gating is active.
    gate: Option<GateConfig>,
    /// The hired agent's standing instructions, prepended to every prompt.
    role_prompt: Option<String>,
}

impl CliEngineRunner {
    /// Architects faction — Claude Code over bidirectional stream-json (RT-1).
    pub fn claude(cwd: impl Into<PathBuf>) -> Self {
        Self {
            program: "claude".into(),
            faction: Faction::Architects,
            cwd: cwd.into(),
            mapper: map_claude_line,
            gate: None,
            role_prompt: None,
        }
    }

    /// Attach the agent's skill/role prompt — prepended to every prompt this
    /// runner is given, so a hired agent carries its standing instructions.
    pub fn with_role_prompt(mut self, prompt: impl Into<String>) -> Self {
        self.role_prompt = Some(prompt.into());
        self
    }

    /// Compose the agent's role prompt with a task prompt.
    fn full_prompt(&self, prompt: &str) -> String {
        match &self.role_prompt {
            Some(rp) if !rp.trim().is_empty() => format!("{rp}\n\n{prompt}"),
            _ => prompt.to_string(),
        }
    }

    /// Attach the US2 permission gate (Claude only). Gated Execute runs route
    /// tool use through `--permission-prompt-tool` instead of auto-accepting.
    pub fn with_gate(mut self, gate: GateConfig) -> Self {
        self.gate = Some(gate);
        self
    }

    /// Forgers faction — Codex `exec --json` (RT-2).
    pub fn codex(cwd: impl Into<PathBuf>) -> Self {
        Self {
            program: "codex".into(),
            faction: Faction::Forgers,
            cwd: cwd.into(),
            mapper: map_codex_line,
            gate: None,
            role_prompt: None,
        }
    }

    pub fn faction(&self) -> Faction {
        self.faction
    }

    /// Build the argv for a role. Pure + deterministic — the SC-002 surface.
    /// Note: contains NO `--api-key`/key flag; auth is the CLI's own session.
    pub fn args(&self, role: Role, prompt: &str) -> Vec<String> {
        match self.faction {
            Faction::Architects => {
                // Plan/Judge are read-only reasoning; Execute may act.
                // A gated Execute must use `default` mode so tool use routes to
                // the permission-prompt-tool — `acceptEdits` auto-accepts and
                // bypasses the gate entirely (verified against a live capture).
                let gated_exec = role == Role::Execute && self.gate.is_some();
                let mode = match role {
                    Role::Plan | Role::Judge => "plan",
                    Role::Execute if gated_exec => "default",
                    Role::Execute => "acceptEdits",
                };
                let mut v = vec![
                    "-p".into(),
                    prompt.into(),
                    "--output-format".into(),
                    "stream-json".into(),
                    "--verbose".into(),
                    "--permission-mode".into(),
                    mode.into(),
                    "--add-dir".into(),
                    self.cwd.to_string_lossy().into_owned(),
                ];
                if gated_exec {
                    let gate = self.gate.as_ref().expect("gated_exec implies gate");
                    v.push("--permission-prompt-tool".into());
                    v.push(gate.prompt_tool.clone());
                    v.push("--mcp-config".into());
                    v.push(gate.mcp_config_json.clone());
                    v.push("--strict-mcp-config".into());
                }
                v
            }
            Faction::Forgers => {
                let sandbox = match role {
                    Role::Plan | Role::Judge => "read-only",
                    Role::Execute => "workspace-write",
                };
                vec![
                    "exec".into(),
                    "--json".into(),
                    "--cd".into(),
                    self.cwd.to_string_lossy().into_owned(),
                    "--sandbox".into(),
                    sandbox.into(),
                    "--skip-git-repo-check".into(),
                    prompt.into(),
                ]
            }
        }
    }
}

#[async_trait]
impl EngineRunner for CliEngineRunner {
    async fn run(&self, role: Role, prompt: &str) -> EngineOutcome {
        let args = self.args(role, &self.full_prompt(prompt));
        match crate::cli::Driver::spawn(&self.program, &args, &self.cwd, self.mapper) {
            Ok(mut driver) => {
                let success = driver.wait().await.unwrap_or(false);
                let signals = driver.collect_remaining().await;
                EngineOutcome::from_signals(signals, success)
            }
            // CLI missing/unspawnable — surfaced as a failed outcome; preflight
            // should have caught this before the run started (EC-004).
            Err(_) => EngineOutcome {
                signals: vec![],
                success: false,
                cost: 0.0,
                final_text: String::new(),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn claude_args_use_stream_json_and_no_api_key() {
        let r = CliEngineRunner::claude("/repo");
        let args = r.args(Role::Plan, "decompose this");
        assert!(args.contains(&"stream-json".to_string()));
        assert!(args.contains(&"--add-dir".to_string()));
        assert!(args.contains(&"/repo".to_string()));
        // SC-002: never an API-key flag.
        assert!(!args
            .iter()
            .any(|a| a.contains("api-key") || a.contains("api_key")));
    }

    #[test]
    fn claude_plan_is_read_only_execute_can_edit() {
        let r = CliEngineRunner::claude("/repo");
        let plan = r.args(Role::Plan, "x");
        let exec = r.args(Role::Execute, "x");
        let mode_after = |v: &[String]| {
            let i = v.iter().position(|a| a == "--permission-mode").unwrap();
            v[i + 1].clone()
        };
        assert_eq!(mode_after(&plan), "plan");
        assert_eq!(mode_after(&exec), "acceptEdits");
    }

    #[test]
    fn codex_args_use_exec_json_and_sandbox() {
        let r = CliEngineRunner::codex("/repo");
        let plan = r.args(Role::Plan, "judge this");
        assert_eq!(plan[0], "exec");
        assert!(plan.contains(&"--json".to_string()));
        assert!(plan.contains(&"read-only".to_string()));
        let exec = r.args(Role::Execute, "do this");
        assert!(exec.contains(&"workspace-write".to_string()));
    }

    #[test]
    fn factions_are_tagged() {
        assert_eq!(CliEngineRunner::claude(".").faction(), Faction::Architects);
        assert_eq!(CliEngineRunner::codex(".").faction(), Faction::Forgers);
    }

    #[test]
    fn role_prompt_is_prepended_to_the_task_prompt() {
        let r =
            CliEngineRunner::claude("/repo").with_role_prompt("You are Cipher, the test author.");
        let full = r.full_prompt("write a test for slugify");
        assert!(full.starts_with("You are Cipher, the test author."));
        assert!(full.contains("write a test for slugify"));
        // The composed prompt is what reaches argv.
        let args = r.args(Role::Execute, &full);
        assert!(args.iter().any(|a| a.contains("You are Cipher")));
        // No role prompt → prompt passes through unchanged.
        let plain = CliEngineRunner::codex("/repo");
        assert_eq!(plain.full_prompt("do x"), "do x");
    }

    #[test]
    fn gated_execute_routes_tool_use_through_the_permission_prompt_tool() {
        let gate = GateConfig {
            mcp_config_json: gate_mcp_config("/tmp/gate.mjs", "http://127.0.0.1:5599/permission", "test-token"),
            prompt_tool: GATE_PROMPT_TOOL.to_string(),
        };
        let r = CliEngineRunner::claude("/repo").with_gate(gate);
        let exec = r.args(Role::Execute, "do the work");
        // Execute must NOT auto-accept — it must route to the gate.
        let i = exec.iter().position(|a| a == "--permission-mode").unwrap();
        assert_eq!(exec[i + 1], "default");
        assert!(exec.contains(&"--permission-prompt-tool".to_string()));
        assert!(exec.contains(&"mcp__gate__approve".to_string()));
        assert!(exec.contains(&"--mcp-config".to_string()));
        assert!(exec.contains(&"--strict-mcp-config".to_string()));
        // The mcp-config carries the loopback gate URL.
        assert!(exec.iter().any(|a| a.contains("127.0.0.1:5599")));
        // SC-002 still holds: no API key anywhere.
        assert!(!exec
            .iter()
            .any(|a| a.contains("api-key") || a.contains("api_key")));
    }

    #[test]
    fn gated_plan_stays_read_only_and_ungated() {
        let gate = GateConfig {
            mcp_config_json: gate_mcp_config("/tmp/gate.mjs", "http://127.0.0.1:5599/permission", "test-token"),
            prompt_tool: GATE_PROMPT_TOOL.to_string(),
        };
        let r = CliEngineRunner::claude("/repo").with_gate(gate);
        let plan = r.args(Role::Plan, "decompose");
        let i = plan.iter().position(|a| a == "--permission-mode").unwrap();
        assert_eq!(plan[i + 1], "plan");
        // Read-only reasoning needs no gate.
        assert!(!plan.contains(&"--permission-prompt-tool".to_string()));
    }

    #[test]
    fn gate_mcp_config_is_valid_and_carries_the_url() {
        let json = gate_mcp_config("/tmp/gate.mjs", "http://127.0.0.1:5599/permission", "test-token");
        let v: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(v["mcpServers"]["gate"]["command"], "node");
        assert_eq!(v["mcpServers"]["gate"]["args"][0], "/tmp/gate.mjs");
        assert_eq!(
            v["mcpServers"]["gate"]["env"]["CONSTRUCT_GATE_URL"],
            "http://127.0.0.1:5599/permission"
        );
    }

    /// T021 / SC-002 — the verifiable subscription-only surface: across BOTH
    /// engines and EVERY role (gated and ungated), the argv The Construct hands a
    /// CLI never carries an API-key flag. Auth is the CLI's own session; a
    /// metered key would have to come from the engineer's env, never from us.
    #[test]
    fn no_engine_role_injects_an_api_key_flag() {
        let gate = GateConfig {
            mcp_config_json: gate_mcp_config("/tmp/gate.mjs", "http://127.0.0.1:5599/permission", "test-token"),
            prompt_tool: GATE_PROMPT_TOOL.to_string(),
        };
        let runners = [
            CliEngineRunner::claude("/repo"),
            CliEngineRunner::claude("/repo").with_gate(gate),
            CliEngineRunner::codex("/repo"),
        ];
        for r in &runners {
            for role in [Role::Plan, Role::Judge, Role::Execute] {
                for a in &r.args(role, "do work") {
                    let lower = a.to_lowercase();
                    assert!(
                        !lower.contains("api-key") && !lower.contains("api_key"),
                        "argv leaked an API key for {role:?}: {a}"
                    );
                }
            }
        }
    }

    /// SC-002 (env half) — the ONLY child env The Construct authors is the gate
    /// MCP server's `CONSTRUCT_GATE_URL` + the per-run `CONSTRUCT_GATE_TOKEN` (M2).
    /// It must never carry an API key, or a gated Claude run would silently route
    /// to metered billing.
    #[test]
    fn gate_env_carries_only_url_and_token_no_api_key() {
        let json = gate_mcp_config("/tmp/gate.mjs", "http://127.0.0.1:5599/permission", "test-token");
        let v: serde_json::Value = serde_json::from_str(&json).unwrap();
        let env = v["mcpServers"]["gate"]["env"].as_object().unwrap();
        let mut keys: Vec<&str> = env.keys().map(|k| k.as_str()).collect();
        keys.sort_unstable();
        assert_eq!(keys, vec!["CONSTRUCT_GATE_TOKEN", "CONSTRUCT_GATE_URL"]);
        assert_eq!(env["CONSTRUCT_GATE_TOKEN"], "test-token");
        for k in env.keys() {
            let lower = k.to_lowercase();
            assert!(!lower.contains("api-key") && !lower.contains("api_key"));
        }
    }
}
