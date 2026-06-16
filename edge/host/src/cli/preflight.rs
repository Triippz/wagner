//! Preflight — verify both CLIs are present and that execution will use
//! subscription auth, not a metered API key (FR-006, SC-002, EC-004).
//!
//! The detection is parameterized over a PATH lookup and an env lookup so it is
//! deterministically testable; `detect_system` wires the real ones.

use serde::Serialize;

/// API-key env vars that, if present, would route a CLI to metered billing.
/// The Construct never sets these; preflight flags them so the engineer knows
/// their shell would override subscription auth.
pub const API_KEY_VARS: [&str; 2] = ["ANTHROPIC_API_KEY", "OPENAI_API_KEY"];

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct EngineStatus {
    pub present: bool,
    pub path: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CliStatus {
    pub claude: EngineStatus,
    pub codex: EngineStatus,
    /// Which API-key vars are set in the environment (should be empty for pure
    /// subscription use). Non-empty is a warning, not a hard block, but SC-002
    /// requires The Construct itself never to add to this list.
    pub api_keys_in_env: Vec<String>,
    /// Set when the engineer's Claude settings pre-allow gated tools or use a
    /// permissive default mode — those bypass the US2 permission gate, so
    /// transmissions would not fire. Warning, not a block.
    pub gate_bypass_warning: Option<String>,
}

impl CliStatus {
    /// True when both CLIs are available — a run may start (EC-004).
    pub fn ready(&self) -> bool {
        self.claude.present && self.codex.present
    }

    /// Human-readable reason a run cannot start, or None when ready.
    pub fn blocking_reason(&self) -> Option<String> {
        match (self.claude.present, self.codex.present) {
            (true, true) => None,
            (false, false) => Some("neither `claude` nor `codex` found on PATH".into()),
            (false, true) => Some("`claude` not found on PATH".into()),
            (true, false) => Some("`codex` not found on PATH".into()),
        }
    }
}

/// Tools the US2 gate guards. If the engineer's settings pre-allow any of these
/// (or use a permissive default mode), Claude never consults the gate.
const GATED_TOOLS: [&str; 3] = ["Bash", "Edit", "Write"];

/// Permission modes under which Claude auto-approves tool use without consulting
/// the `--permission-prompt-tool`.
const PERMISSIVE_MODES: [&str; 4] = ["auto", "bypassPermissions", "dontAsk", "acceptEdits"];

/// Inspect a Claude `settings.json` value for configuration that would bypass
/// the US2 permission gate. Returns a human-readable warning, or None when clean.
pub fn gate_bypass_warning(settings: &serde_json::Value) -> Option<String> {
    let perms = settings.get("permissions");
    let mut reasons = Vec::new();

    if let Some(allow) = perms
        .and_then(|p| p.get("allow"))
        .and_then(|a| a.as_array())
    {
        let pre_allowed: Vec<&str> = GATED_TOOLS
            .iter()
            .copied()
            .filter(|tool| allow.iter().any(|v| v.as_str() == Some(tool)))
            .collect();
        if !pre_allowed.is_empty() {
            reasons.push(format!(
                "allow rules pre-approve {}",
                pre_allowed.join(", ")
            ));
        }
    }
    if let Some(mode) = perms
        .and_then(|p| p.get("defaultMode"))
        .and_then(|m| m.as_str())
    {
        if PERMISSIVE_MODES.contains(&mode) {
            reasons.push(format!("defaultMode `{mode}` auto-approves tool use"));
        }
    }

    (!reasons.is_empty()).then(|| {
        format!(
            "Claude settings bypass the permission gate ({}); US2 transmissions \
             will not fire for those tools. Remove them from permissions.allow / \
             set defaultMode to `default` to surface permission prompts.",
            reasons.join("; ")
        )
    })
}

/// Detect CLI availability and API-key env presence using injected lookups.
pub fn detect<P, E>(path_lookup: P, env_lookup: E) -> CliStatus
where
    P: Fn(&str) -> Option<String>,
    E: Fn(&str) -> Option<String>,
{
    let engine = |name: &str| {
        let path = path_lookup(name);
        EngineStatus {
            present: path.is_some(),
            path,
        }
    };
    let api_keys_in_env = API_KEY_VARS
        .iter()
        .filter(|k| env_lookup(k).is_some_and(|v| !v.is_empty()))
        .map(|k| k.to_string())
        .collect();

    CliStatus {
        claude: engine("claude"),
        codex: engine("codex"),
        api_keys_in_env,
        gate_bypass_warning: None,
    }
}

/// Real detection: PATH search + process env + Claude settings inspection.
pub fn detect_system() -> CliStatus {
    let mut status = detect(which_on_path, |k| std::env::var(k).ok());
    status.gate_bypass_warning = read_claude_settings().and_then(|s| gate_bypass_warning(&s));
    status
}

/// Read the engineer's Claude `settings.json` (honoring `CLAUDE_CONFIG_DIR`).
fn read_claude_settings() -> Option<serde_json::Value> {
    let dir = std::env::var_os("CLAUDE_CONFIG_DIR")
        .map(std::path::PathBuf::from)
        .or_else(|| {
            std::env::var_os("HOME").map(|h| std::path::PathBuf::from(h).join(".claude"))
        })?;
    let raw = std::fs::read_to_string(dir.join("settings.json")).ok()?;
    serde_json::from_str(&raw).ok()
}

/// Minimal `which`: scan PATH entries for an executable file named `program`.
fn which_on_path(program: &str) -> Option<String> {
    let path = std::env::var_os("PATH")?;
    for dir in std::env::split_paths(&path) {
        let candidate = dir.join(program);
        if candidate.is_file() {
            return Some(candidate.to_string_lossy().to_string());
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ready_when_both_present_and_no_keys() {
        let status = detect(
            |n| match n {
                "claude" => Some("/usr/bin/claude".into()),
                "codex" => Some("/usr/bin/codex".into()),
                _ => None,
            },
            |_| None,
        );
        assert!(status.ready());
        assert!(status.blocking_reason().is_none());
        assert!(status.api_keys_in_env.is_empty());
    }

    #[test]
    fn not_ready_when_codex_missing() {
        let status = detect(
            |n| (n == "claude").then(|| "/usr/bin/claude".into()),
            |_| None,
        );
        assert!(!status.ready());
        assert_eq!(
            status.blocking_reason().unwrap(),
            "`codex` not found on PATH"
        );
    }

    #[test]
    fn flags_api_key_in_env() {
        // SC-002: an API key in the engineer's env is surfaced so they know it
        // would override subscription billing. The Construct itself never sets these.
        let status = detect(
            |_| Some("/usr/bin/x".into()),
            |k| (k == "ANTHROPIC_API_KEY").then(|| "sk-xxx".to_string()),
        );
        assert_eq!(status.api_keys_in_env, vec!["ANTHROPIC_API_KEY"]);
    }

    #[test]
    fn empty_api_key_is_not_flagged() {
        let status = detect(|_| Some("/x".into()), |_| Some(String::new()));
        assert!(status.api_keys_in_env.is_empty());
    }

    #[test]
    fn permissive_allow_rules_bypass_the_gate() {
        let s = serde_json::json!({"permissions":{"allow":["Bash","Edit","Read"]}});
        let w = gate_bypass_warning(&s).expect("Bash/Edit allow must warn");
        assert!(w.contains("Bash"));
        assert!(w.contains("Edit"));
        assert!(!w.contains("Read")); // Read is not a gated tool
    }

    #[test]
    fn permissive_default_mode_bypasses_the_gate() {
        for mode in ["auto", "bypassPermissions", "dontAsk", "acceptEdits"] {
            let s = serde_json::json!({"permissions":{"defaultMode":mode}});
            assert!(
                gate_bypass_warning(&s).is_some(),
                "{mode} should warn about gate bypass"
            );
        }
    }

    #[test]
    fn clean_settings_do_not_warn() {
        let s = serde_json::json!({"permissions":{"allow":["Read"],"defaultMode":"default"}});
        assert!(gate_bypass_warning(&s).is_none());
    }
}
