//! Map Claude Code `--output-format stream-json` lines to normalized signals.
//!
//! Verified against a real capture (`tests/fixtures/claude-sample.jsonl`):
//! - `{"type":"system","subtype":"init",...}`         → operative spawned
//! - `{"type":"assistant","message":{content:[...]}}` → activity (text/tool_use)
//! - `{"type":"result","subtype":"success","total_cost_usd":..,"result":..}` → completed (carries cost)

use crate::events::Activity;
use serde_json::Value;

/// A normalized signal derived from one CLI output line. The loop turns these
/// into `WagnerEvent`s and folds cost into the run's guardrails.
#[derive(Debug, Clone, PartialEq)]
pub enum CliSignal {
    /// Session started — operative appears, idle.
    Spawned,
    /// The operative is doing something; drives district + bubble.
    Activity {
        activity: Activity,
        message: Option<String>,
    },
    /// The CLI paused awaiting a permission/question decision.
    AwaitingInput { prompt: String },
    /// The run finished; `cost_usd` is the CLI-reported USD cost when present
    /// (Claude), and `tokens` is the CLI-reported token total when present
    /// (Codex). They are kept distinct so a USD budget is never compared against a
    /// token count (FR-015).
    Completed {
        cost_usd: Option<f64>,
        tokens: Option<u64>,
        result: String,
    },
    /// A line we intentionally ignore (hooks, rate-limit notices, etc.).
    Ignored,
}

/// Infer the activity from a Claude tool name (+ optional bash command text).
pub fn tool_to_activity(tool: &str, command: Option<&str>) -> Activity {
    match tool {
        "Read" | "Glob" | "Grep" | "NotebookRead" => Activity::Read,
        "Edit" | "Write" | "NotebookEdit" | "MultiEdit" => Activity::Edit,
        "Task" => Activity::Decompose,
        "TodoWrite" => Activity::Think,
        "Bash" | "BashOutput" | "KillBash" => match command.map(classify_command) {
            Some(a) => a,
            None => Activity::Shell,
        },
        _ => Activity::Shell,
    }
}

fn classify_command(cmd: &str) -> Activity {
    let c = cmd.to_lowercase();
    if c.contains("test") || c.contains("vitest") || c.contains("playwright") {
        Activity::Test
    } else if c.contains("build") || c.contains("compile") || c.contains("tsc") {
        Activity::Build
    } else if c.contains("lint") || c.contains("clippy") || c.contains("fmt") {
        Activity::Lint
    } else {
        Activity::Shell
    }
}

/// Map a single stream-json line. Unparseable lines are `Ignored`, never errors —
/// a CLI is allowed to emit lines we don't model.
pub fn map_claude_line(line: &str) -> CliSignal {
    let v: Value = match serde_json::from_str(line) {
        Ok(v) => v,
        Err(_) => return CliSignal::Ignored,
    };
    match v.get("type").and_then(Value::as_str) {
        Some("system") => match v.get("subtype").and_then(Value::as_str) {
            Some("init") => CliSignal::Spawned,
            _ => CliSignal::Ignored, // hooks, progress, etc.
        },
        Some("assistant") => map_assistant(&v),
        Some("result") => CliSignal::Completed {
            cost_usd: v.get("total_cost_usd").and_then(Value::as_f64),
            tokens: None,
            result: v
                .get("result")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string(),
        },
        _ => CliSignal::Ignored,
    }
}

fn map_assistant(v: &Value) -> CliSignal {
    let content = v
        .get("message")
        .and_then(|m| m.get("content"))
        .and_then(Value::as_array);
    let Some(blocks) = content else {
        return CliSignal::Ignored;
    };

    // A tool_use block wins over text (it's the concrete action).
    for block in blocks {
        if block.get("type").and_then(Value::as_str) == Some("tool_use") {
            let tool = block.get("name").and_then(Value::as_str).unwrap_or("");
            let command = block
                .get("input")
                .and_then(|i| i.get("command"))
                .and_then(Value::as_str);
            let activity = tool_to_activity(tool, command);
            let message = Some(match command {
                Some(c) => format!("{}: {}", tool, super::truncate(c, 60)),
                None => tool.to_string(),
            });
            return CliSignal::Activity { activity, message };
        }
    }

    // Otherwise it's reasoning/text → thinking.
    for block in blocks {
        if block.get("type").and_then(Value::as_str) == Some("text") {
            let text = block.get("text").and_then(Value::as_str).unwrap_or("");
            return CliSignal::Activity {
                activity: Activity::Think,
                message: Some(super::truncate(text, super::MESSAGE_PREVIEW_MAX)),
            };
        }
    }
    CliSignal::Ignored
}

#[cfg(test)]
mod tests {
    use super::*;

    const FIXTURE: &str = include_str!("../../tests/fixtures/claude-sample.jsonl");

    #[test]
    fn maps_real_fixture_lines() {
        let signals: Vec<CliSignal> = FIXTURE.lines().map(map_claude_line).collect();
        // init → Spawned
        assert_eq!(signals[0], CliSignal::Spawned);
        // assistant "ok" → thinking
        assert!(matches!(
            &signals[1],
            CliSignal::Activity {
                activity: Activity::Think,
                ..
            }
        ));
        // result → Completed with the real cost present
        match &signals[2] {
            CliSignal::Completed { cost_usd, tokens, result } => {
                assert!(
                    cost_usd.unwrap() > 0.0,
                    "real fixture carries total_cost_usd"
                );
                assert_eq!(*tokens, None, "Claude reports USD, not a token count");
                assert_eq!(result, "ok");
            }
            other => panic!("expected Completed, got {:?}", other),
        }
    }

    #[test]
    fn tool_use_maps_to_district_activity() {
        let line = r#"{"type":"assistant","message":{"content":[{"type":"tool_use","name":"Edit","input":{"file_path":"src/x.rs"}}]}}"#;
        assert!(matches!(
            map_claude_line(line),
            CliSignal::Activity {
                activity: Activity::Edit,
                ..
            }
        ));
    }

    #[test]
    fn bash_test_command_is_test_activity() {
        let line = r#"{"type":"assistant","message":{"content":[{"type":"tool_use","name":"Bash","input":{"command":"cargo test --lib"}}]}}"#;
        assert!(matches!(
            map_claude_line(line),
            CliSignal::Activity {
                activity: Activity::Test,
                ..
            }
        ));
    }

    #[test]
    fn command_classification() {
        assert_eq!(classify_command("pnpm build"), Activity::Build);
        assert_eq!(classify_command("cargo clippy"), Activity::Lint);
        assert_eq!(classify_command("ls -la"), Activity::Shell);
        assert_eq!(tool_to_activity("Read", None), Activity::Read);
        assert_eq!(tool_to_activity("Task", None), Activity::Decompose);
    }

    #[test]
    fn hook_and_garbage_lines_are_ignored() {
        assert_eq!(
            map_claude_line(r#"{"type":"system","subtype":"hook_started"}"#),
            CliSignal::Ignored
        );
        assert_eq!(map_claude_line("not json at all"), CliSignal::Ignored);
    }
}
