//! Map Codex `exec --json` JSONL lines to normalized signals.
//!
//! Verified against a real capture (`tests/fixtures/codex-sample.jsonl`):
//! - `{"type":"thread.started","thread_id":..}`                 → operative spawned
//! - `{"type":"turn.started"}`                                   → thinking
//! - `{"type":"item.completed","item":{"type":"agent_message"|"command_execution"|"file_change",..}}` → activity
//! - `{"type":"turn.completed","usage":{...tokens...}}`          → completed (token usage; no USD)
//!
//! Codex reports token counts, not USD, so the Forgers cost dimension is token-based
//! (or wall-clock) rather than dollar-based (FR-015).

use crate::events::{Activity, CliSignal};
use serde_json::Value;

/// Total tokens reported by a Codex `turn.completed` usage block, if present.
pub fn usage_total_tokens(usage: &Value) -> Option<u64> {
    let inp = usage.get("input_tokens").and_then(Value::as_u64);
    let out = usage.get("output_tokens").and_then(Value::as_u64);
    match (inp, out) {
        (None, None) => None,
        _ => Some(inp.unwrap_or(0) + out.unwrap_or(0)),
    }
}

/// Map a Codex item type to a Construct activity.
fn item_to_activity(item_type: &str) -> Activity {
    match item_type {
        "command_execution" => Activity::Shell,
        "file_change" | "patch_apply" => Activity::Edit,
        "file_read" => Activity::Read,
        "reasoning" | "agent_message" => Activity::Think,
        _ => Activity::Think,
    }
}

pub fn map_codex_line(line: &str) -> CliSignal {
    let v: Value = match serde_json::from_str(line) {
        Ok(v) => v,
        Err(_) => return CliSignal::Ignored,
    };
    match v.get("type").and_then(Value::as_str) {
        Some("thread.started") => CliSignal::Spawned,
        Some("turn.started") => CliSignal::Activity {
            activity: Activity::Think,
            message: None,
        },
        Some("item.completed") => map_item(&v),
        Some("turn.completed") => {
            // Codex reports token usage, not USD. Surface it in the distinct
            // `tokens` slot — never in `cost_usd` — so a USD budget is never
            // compared against a token count (M8). Result text is empty; the loop
            // reads usage separately.
            CliSignal::Completed {
                cost_usd: None,
                tokens: v.get("usage").and_then(usage_total_tokens),
                result: String::new(),
            }
        }
        _ => CliSignal::Ignored,
    }
}

fn map_item(v: &Value) -> CliSignal {
    let item = match v.get("item") {
        Some(i) => i,
        None => return CliSignal::Ignored,
    };
    let item_type = item.get("type").and_then(Value::as_str).unwrap_or("");
    let activity = item_to_activity(item_type);
    let message = item
        .get("text")
        .and_then(Value::as_str)
        .or_else(|| item.get("command").and_then(Value::as_str))
        .map(|s| super::truncate(s, super::MESSAGE_PREVIEW_MAX));
    CliSignal::Activity { activity, message }
}

#[cfg(test)]
mod tests {
    use super::*;

    const FIXTURE: &str = include_str!("../../tests/fixtures/codex-sample.jsonl");

    #[test]
    fn maps_real_fixture_lines() {
        let signals: Vec<CliSignal> = FIXTURE.lines().map(map_codex_line).collect();
        assert_eq!(signals[0], CliSignal::Spawned);
        assert!(matches!(
            signals[1],
            CliSignal::Activity {
                activity: Activity::Think,
                ..
            }
        ));
        assert!(matches!(
            &signals[2],
            CliSignal::Activity {
                activity: Activity::Think,
                ..
            }
        ));
        // turn.completed carries token usage in the distinct `tokens` slot — never
        // in the USD `cost_usd` slot (M8).
        match &signals[3] {
            CliSignal::Completed { cost_usd, tokens, .. } => {
                assert_eq!(*cost_usd, None, "Codex tokens must not be written into the USD slot");
                assert_eq!(tokens.unwrap(), 24263 + 52);
            }
            other => panic!("expected Completed, got {:?}", other),
        }
    }

    #[test]
    fn command_execution_is_shell() {
        let line = r#"{"type":"item.completed","item":{"type":"command_execution","command":"cargo test"}}"#;
        assert!(matches!(
            map_codex_line(line),
            CliSignal::Activity {
                activity: Activity::Shell,
                ..
            }
        ));
    }

    #[test]
    fn file_change_is_edit() {
        let line = r#"{"type":"item.completed","item":{"type":"file_change","text":"src/x.rs"}}"#;
        assert!(matches!(
            map_codex_line(line),
            CliSignal::Activity {
                activity: Activity::Edit,
                ..
            }
        ));
    }

    #[test]
    fn usage_total_handles_missing() {
        assert_eq!(usage_total_tokens(&serde_json::json!({})), None);
        assert_eq!(
            usage_total_tokens(&serde_json::json!({"input_tokens": 10, "output_tokens": 5})),
            Some(15)
        );
    }

    #[test]
    fn garbage_ignored() {
        assert_eq!(map_codex_line("xyz"), CliSignal::Ignored);
    }
}
