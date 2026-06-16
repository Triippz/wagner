//! Transmissions (US2) — human-in-the-loop permission/question round-trips.
//!
//! Per the empirical finding in research.md RT-1b, headless Claude does not emit
//! interactive permission prompts; the host instead exposes an MCP
//! `--permission-prompt-tool`. When Claude wants to use a tool, it calls that MCP
//! tool with `{ tool_name, input }`; the host turns the call into a `Transmission`,
//! emits it to the floor (Gate), waits for the engineer's answer, and returns the
//! documented permission response (`behavior: allow|deny`).
//!
//! This module is the transport-agnostic CORE: the request→transmission→answer→
//! response flow and the pending-request registry. The MCP server wiring that
//! delivers `PermissionRequest`s is the one part still needing a live capture; it
//! is isolated to `parse_permission_request`.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Mutex;
use tokio::sync::oneshot;

/// What the MCP permission tool receives from Claude (the one shape pending a
/// live capture — isolated here so a correction touches only this struct).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PermissionRequest {
    pub tool_name: String,
    #[serde(default)]
    pub input: Value,
    /// Correlation id from Claude's tool call (`tool_use_id` in the live envelope).
    #[serde(default)]
    pub tool_use_id: Option<String>,
}

/// Parse an incoming MCP permission-tool call argument object.
///
/// The live `can_use_tool` envelope (captured 2026-06-13, see
/// `tests/fixtures/can_use_tool.json`) is `{ tool_name, input, tool_use_id }`.
/// We tolerate the documented `toolName`/`tool_input` aliases defensively.
pub fn parse_permission_request(args: &Value) -> Option<PermissionRequest> {
    let tool_name = args
        .get("tool_name")
        .or_else(|| args.get("toolName"))
        .and_then(Value::as_str)?
        .to_string();
    let input = args
        .get("input")
        .or_else(|| args.get("tool_input"))
        .cloned()
        .unwrap_or(Value::Null);
    let tool_use_id = args
        .get("tool_use_id")
        .or_else(|| args.get("toolUseId"))
        .and_then(Value::as_str)
        .map(str::to_string);
    Some(PermissionRequest {
        tool_name,
        input,
        tool_use_id,
    })
}

/// The engineer's decision on a transmission.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Decision {
    Allow,
    Deny,
}

impl Decision {
    /// Map a transmission option id / free-text answer to a decision.
    pub fn from_answer(answer: &str) -> Self {
        match answer.trim().to_lowercase().as_str() {
            "allow" | "yes" | "approve" | "y" | "ok" => Decision::Allow,
            _ => Decision::Deny,
        }
    }
}

/// The documented Claude permission-prompt-tool response payload.
#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(untagged)]
pub enum PermissionResponse {
    Allow {
        behavior: &'static str,
        #[serde(rename = "updatedInput")]
        updated_input: Value,
    },
    Deny {
        behavior: &'static str,
        message: String,
    },
}

/// Build the response payload Claude expects for a decision.
pub fn permission_response(decision: Decision, original_input: &Value) -> PermissionResponse {
    match decision {
        Decision::Allow => PermissionResponse::Allow {
            behavior: "allow",
            updated_input: original_input.clone(),
        },
        Decision::Deny => PermissionResponse::Deny {
            behavior: "deny",
            message: "Denied by the engineer via The Construct.".to_string(),
        },
    }
}

/// Build a Gate transmission JSON from a permission request, for the frontend.
pub fn request_to_transmission_json(
    req: &PermissionRequest,
    id: &str,
    subtask_id: &str,
    raised_at: &str,
) -> Value {
    serde_json::json!({
        "schema": "transmission.v1",
        "id": id,
        "subtask_id": subtask_id,
        "kind": "permission",
        "prompt": format!("Allow {} to run?", req.tool_name),
        "options": [
            {"id": "allow", "label": "Allow"},
            {"id": "deny", "label": "Deny"}
        ],
        "raised_at": raised_at,
        "state": "open"
    })
}

/// Tracks open transmissions awaiting an engineer answer. The MCP tool handler
/// opens one and awaits; `answer` resolves it.
#[derive(Default)]
pub struct TransmissionRegistry {
    pending: Mutex<HashMap<String, oneshot::Sender<Decision>>>,
}

impl TransmissionRegistry {
    /// Register a transmission id and get the receiver to await the decision on.
    pub fn open(&self, id: String) -> oneshot::Receiver<Decision> {
        let (tx, rx) = oneshot::channel();
        self.pending.lock().unwrap().insert(id, tx);
        rx
    }

    /// Resolve a transmission with the engineer's decision. Returns false if the
    /// id was unknown (already answered / timed out).
    pub fn answer(&self, id: &str, decision: Decision) -> bool {
        if let Some(tx) = self.pending.lock().unwrap().remove(id) {
            tx.send(decision).is_ok()
        } else {
            false
        }
    }

    /// Drop a pending transmission (timeout / abort).
    pub fn cancel(&self, id: &str) {
        self.pending.lock().unwrap().remove(id);
    }

    pub fn open_count(&self) -> usize {
        self.pending.lock().unwrap().len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn parses_permission_request_both_key_styles() {
        let a = parse_permission_request(&json!({"tool_name":"Bash","input":{"command":"ls"}}))
            .unwrap();
        assert_eq!(a.tool_name, "Bash");
        let b = parse_permission_request(&json!({"toolName":"Write","tool_input":{"path":"x"}}))
            .unwrap();
        assert_eq!(b.tool_name, "Write");
        assert!(parse_permission_request(&json!({"nope":1})).is_none());
    }

    #[test]
    fn decision_from_answer() {
        assert_eq!(Decision::from_answer("allow"), Decision::Allow);
        assert_eq!(Decision::from_answer("YES"), Decision::Allow);
        assert_eq!(Decision::from_answer("deny"), Decision::Deny);
        assert_eq!(Decision::from_answer("anything else"), Decision::Deny);
    }

    #[test]
    fn allow_response_echoes_input_deny_carries_message() {
        let input = json!({"command":"ls"});
        let allow = permission_response(Decision::Allow, &input);
        let allow_json = serde_json::to_value(&allow).unwrap();
        assert_eq!(allow_json["behavior"], "allow");
        // Claude's permission-prompt-tool contract requires camelCase `updatedInput`.
        assert_eq!(allow_json["updatedInput"], input);

        let deny = permission_response(Decision::Deny, &input);
        let deny_json = serde_json::to_value(&deny).unwrap();
        assert_eq!(deny_json["behavior"], "deny");
        assert!(deny_json["message"].as_str().unwrap().contains("engineer"));
    }

    /// Ground-truth: parse the LIVE captured `can_use_tool` payload (2026-06-13).
    /// The real envelope uses `input` (not `tool_input`) and carries `tool_use_id`.
    #[test]
    fn parses_real_can_use_tool_capture() {
        let raw = std::fs::read_to_string(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/fixtures/can_use_tool.json"
        ))
        .expect("live capture fixture must be present");
        let v: Value = serde_json::from_str(&raw).unwrap();
        let req = parse_permission_request(&v["arguments"]).expect("real capture must parse");
        assert_eq!(req.tool_name, "WebFetch");
        assert_eq!(req.input["url"], "https://example.com");
        assert_eq!(
            req.tool_use_id.as_deref(),
            Some("toolu_01LTcZSyofe22ZcebZdzpwUm")
        );
    }

    #[test]
    fn transmission_json_validates_against_schema() {
        let req = PermissionRequest {
            tool_name: "Bash".into(),
            input: json!({"command":"rm -rf x"}),
            tool_use_id: None,
        };
        let tj = request_to_transmission_json(&req, "t1", "s1", "2026-06-13T00:00:00Z");
        crate::schema::validate(crate::schema::TRANSMISSION_SCHEMA, &tj)
            .expect("built transmission must validate");
    }

    #[tokio::test]
    async fn registry_round_trips_a_decision() {
        let reg = TransmissionRegistry::default();
        let rx = reg.open("t1".into());
        assert_eq!(reg.open_count(), 1);
        assert!(reg.answer("t1", Decision::Allow));
        assert_eq!(rx.await.unwrap(), Decision::Allow);
        assert_eq!(reg.open_count(), 0);
    }

    #[test]
    fn answering_unknown_id_is_false() {
        let reg = TransmissionRegistry::default();
        assert!(!reg.answer("nope", Decision::Deny));
    }
}
