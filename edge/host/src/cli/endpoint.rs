//! Local-model harness (Phase 3) — an `EngineRunner` backed by an OpenAI-compatible
//! HTTP chat endpoint (vLLM / Ollama / any self-hosted server).
//!
//! Unlike the Claude/Codex runners (which spawn the engineer's subscription
//! CLIs), this one POSTs to `{base_url}/chat/completions`. Local/self-hosted
//! models are **not metered**, so SC-002 (no metered API, subscription-only)
//! is untouched — no API key is ever attached.
//!
//! The wire mapping (request body, response/​models parsing) is pure and unit
//! tested; the `run`/`ping` methods are the thin reqwest shell over it.

use crate::events::CliSignal;
use crate::orchestrator::engine::{EngineOutcome, EngineRunner, Role};
use async_trait::async_trait;
use serde_json::{json, Value};

/// Bound every model HTTP call so a hung/slow local server can't hang the goal
/// loop forever — no guardrail otherwise catches a stalled socket.
const MODEL_HTTP_TIMEOUT_SECS: u64 = 120;

/// Build a reqwest client with the model-call timeout applied. Falls back to the
/// default client if the (TLS) builder ever fails — never panics.
fn http_client() -> reqwest::Client {
    reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(MODEL_HTTP_TIMEOUT_SECS))
        .build()
        .unwrap_or_default()
}

/// An OpenAI-compatible chat endpoint runner.
pub struct EndpointRunner {
    base_url: String,
    model: String,
    /// Standing instructions prepended as the system message (the agent's skill).
    role_prompt: Option<String>,
    client: reqwest::Client,
}

impl EndpointRunner {
    pub fn new(base_url: impl Into<String>, model: impl Into<String>) -> Self {
        Self {
            base_url: normalize_base(&base_url.into()),
            model: model.into(),
            role_prompt: None,
            client: http_client(),
        }
    }

    pub fn with_role_prompt(mut self, prompt: String) -> Self {
        self.role_prompt = Some(prompt);
        self
    }

    fn chat_url(&self) -> String {
        format!("{}/chat/completions", self.base_url)
    }
}

#[async_trait]
impl EngineRunner for EndpointRunner {
    async fn run(&self, _role: Role, prompt: &str) -> EngineOutcome {
        if !is_http_url(&self.base_url) {
            return failed(format!(
                "endpoint base_url must be http(s): {}",
                self.base_url
            ));
        }
        if is_metadata_endpoint(&self.base_url) {
            return failed(format!(
                "refusing to call a cloud instance-metadata endpoint: {}",
                self.base_url
            ));
        }
        let body = chat_request_body(&self.model, self.role_prompt.as_deref(), prompt);
        let resp = self.client.post(self.chat_url()).json(&body).send().await;
        match resp {
            Ok(r) => match r.json::<Value>().await {
                Ok(v) => match parse_chat_response(&v) {
                    Ok(text) => EngineOutcome {
                        // Local models are unmetered → cost 0; surface on the floor.
                        signals: vec![CliSignal::Spawned],
                        success: true,
                        cost: 0.0,
                        final_text: text,
                    },
                    Err(e) => failed(e),
                },
                Err(e) => failed(format!("endpoint returned non-JSON: {e}")),
            },
            Err(e) => failed(format!("endpoint unreachable: {e}")),
        }
    }
}

fn failed(msg: String) -> EngineOutcome {
    EngineOutcome {
        signals: vec![],
        success: false,
        cost: 0.0,
        final_text: msg,
    }
}

/// Reachability + served models for an endpoint (preflight).
#[derive(Debug, Clone, serde::Serialize, PartialEq)]
pub struct EndpointStatus {
    pub reachable: bool,
    /// Model ids the endpoint advertises (from `GET {base_url}/models`).
    pub models: Vec<String>,
    pub error: Option<String>,
}

/// Ping an endpoint's `/models` and report what it serves. Never panics.
pub async fn ping(base_url: &str) -> EndpointStatus {
    if !is_http_url(base_url) {
        return EndpointStatus {
            reachable: false,
            models: vec![],
            error: Some("base_url must start with http:// or https://".into()),
        };
    }
    if is_metadata_endpoint(base_url) {
        return EndpointStatus {
            reachable: false,
            models: vec![],
            error: Some("refusing to ping a cloud instance-metadata endpoint".into()),
        };
    }
    let url = format!("{}/models", normalize_base(base_url));
    let client = http_client();
    match client.get(&url).send().await {
        Ok(r) => match r.json::<Value>().await {
            Ok(v) => EndpointStatus {
                reachable: true,
                models: parse_models_response(&v),
                error: None,
            },
            Err(e) => EndpointStatus {
                reachable: true,
                models: vec![],
                error: Some(format!("models endpoint returned non-JSON: {e}")),
            },
        },
        Err(e) => EndpointStatus {
            reachable: false,
            models: vec![],
            error: Some(e.to_string()),
        },
    }
}

// ---- pure wire mapping ----------------------------------------------------

/// Trim a trailing slash so `{base}/chat/completions` is well-formed.
fn normalize_base(base: &str) -> String {
    base.trim().trim_end_matches('/').to_string()
}

/// Only http(s) endpoints are allowed — rejects `file://` and other schemes so a
/// stray/hostile base_url can't be coerced into a non-HTTP fetch. (reqwest also
/// refuses non-http(s), but failing fast here gives a clear error + explicit intent.)
fn is_http_url(base: &str) -> bool {
    let b = base.trim();
    b.starts_with("http://") || b.starts_with("https://")
}

/// Block the well-known cloud instance-metadata endpoints — never a legitimate
/// local-model host, and the one SSRF target worth refusing outright should the
/// host ever run in a cloud/CI context (where it would leak IAM credentials).
/// Matched on the lowercased, bracket-stripped string so an IPv4-mapped IPv6
/// literal (`[::ffff:169.254.169.254]`) can't slip past. Localhost / LAN model
/// hosts are unaffected. (Defense in depth — the deployment should also egress-
/// firewall the link-local range.)
fn is_metadata_endpoint(base: &str) -> bool {
    let lower = base.trim().to_lowercase();
    // Strip IPv6 brackets so `[…]` literals are matched on their inner address.
    let stripped = lower.replace(['[', ']'], "");
    const BLOCKED: [&str; 7] = [
        "169.254.169.254",          // AWS / Azure / GCP IMDS (IPv4)
        "ffff:169.254.169.254",     // IPv4-mapped IPv6 form of the above
        "fd00:ec2::254",            // AWS IMDS over IPv6
        "metadata.google.internal", // GCP metadata hostname
        "100.100.100.200",          // Alibaba Cloud metadata
        "metadata.azure.com",       // Azure IMDS hostname alias
        "0.0.0.0",                  // unspecified host — never a real model host
    ];
    BLOCKED.iter().any(|needle| stripped.contains(needle))
}

/// Build the chat-completions request body. The role prompt (if any) becomes the
/// system message; the task prompt is the user message.
pub fn chat_request_body(model: &str, role_prompt: Option<&str>, prompt: &str) -> Value {
    let mut messages = Vec::new();
    if let Some(sys) = role_prompt.filter(|s| !s.trim().is_empty()) {
        messages.push(json!({"role": "system", "content": sys}));
    }
    messages.push(json!({"role": "user", "content": prompt}));
    json!({ "model": model, "messages": messages, "stream": false })
}

/// Extract the assistant message text from an OpenAI-style chat response.
pub fn parse_chat_response(v: &Value) -> Result<String, String> {
    v.get("choices")
        .and_then(|c| c.get(0))
        .and_then(|c| c.get("message"))
        .and_then(|m| m.get("content"))
        .and_then(|t| t.as_str())
        .map(|s| s.to_string())
        .ok_or_else(|| "no choices[0].message.content in response".to_string())
}

/// Extract model ids from an OpenAI-style `/models` response (`{data:[{id}]}`).
pub fn parse_models_response(v: &Value) -> Vec<String> {
    v.get("data")
        .and_then(|d| d.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|m| m.get("id").and_then(|i| i.as_str()).map(String::from))
                .collect()
        })
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn request_body_carries_model_and_user_prompt() {
        let body = chat_request_body("llama3", None, "do the thing");
        assert_eq!(body["model"], "llama3");
        assert_eq!(body["messages"][0]["role"], "user");
        assert_eq!(body["messages"][0]["content"], "do the thing");
        assert_eq!(body["stream"], false);
    }

    #[test]
    fn role_prompt_becomes_a_leading_system_message() {
        let body = chat_request_body("m", Some("you are precise"), "task");
        assert_eq!(body["messages"][0]["role"], "system");
        assert_eq!(body["messages"][0]["content"], "you are precise");
        assert_eq!(body["messages"][1]["role"], "user");
    }

    #[test]
    fn blank_role_prompt_is_omitted() {
        let body = chat_request_body("m", Some("   "), "task");
        assert_eq!(body["messages"].as_array().unwrap().len(), 1);
        assert_eq!(body["messages"][0]["role"], "user");
    }

    #[test]
    fn parses_assistant_content_from_a_chat_response() {
        let v = json!({
            "choices": [{"message": {"role": "assistant", "content": "the answer"}}]
        });
        assert_eq!(parse_chat_response(&v).unwrap(), "the answer");
    }

    #[test]
    fn chat_response_without_content_is_an_error() {
        assert!(parse_chat_response(&json!({"choices": []})).is_err());
        assert!(parse_chat_response(&json!({})).is_err());
    }

    #[test]
    fn parses_served_model_ids() {
        let v = json!({"object": "list", "data": [{"id": "qwen2.5-coder"}, {"id": "llama3"}]});
        assert_eq!(parse_models_response(&v), vec!["qwen2.5-coder", "llama3"]);
        assert_eq!(parse_models_response(&json!({})), Vec::<String>::new());
    }

    #[test]
    fn base_url_trailing_slash_is_normalized() {
        let r = EndpointRunner::new("http://localhost:11434/v1/", "m");
        assert_eq!(r.chat_url(), "http://localhost:11434/v1/chat/completions");
    }

    #[test]
    fn non_http_schemes_are_rejected() {
        assert!(is_http_url("http://localhost:8000/v1"));
        assert!(is_http_url("https://host/v1"));
        assert!(!is_http_url("file:///etc/passwd"));
        assert!(!is_http_url("ftp://host"));
        assert!(!is_http_url("localhost:8000"));
    }

    #[tokio::test]
    async fn run_rejects_a_non_http_base_url_without_calling_out() {
        let r = EndpointRunner::new("file:///etc/passwd", "m");
        let out = r.run(Role::Execute, "x").await;
        assert!(!out.success);
        assert!(out.final_text.contains("http(s)"));
    }

    #[test]
    fn metadata_endpoints_are_blocked_but_local_hosts_are_not() {
        // Every cloud IMDS form, including the bracketed IPv4-mapped IPv6 literal
        // that would slip past a naive substring match.
        assert!(is_metadata_endpoint("http://169.254.169.254/latest/meta-data"));
        assert!(is_metadata_endpoint("http://[::ffff:169.254.169.254]/"));
        assert!(is_metadata_endpoint("http://[fd00:ec2::254]/latest"));
        assert!(is_metadata_endpoint("http://metadata.google.internal/x"));
        assert!(is_metadata_endpoint("http://100.100.100.200/latest"));
        assert!(is_metadata_endpoint("http://0.0.0.0:80/v1"));
        // Legitimate local / LAN model hosts are unaffected.
        assert!(!is_metadata_endpoint("http://localhost:11434/v1"));
        assert!(!is_metadata_endpoint("http://192.168.1.50:8000/v1"));
        assert!(!is_metadata_endpoint("http://127.0.0.1:8080/v1"));
    }

    #[tokio::test]
    async fn run_refuses_a_cloud_metadata_endpoint() {
        let r = EndpointRunner::new("http://169.254.169.254/v1", "m");
        let out = r.run(Role::Execute, "x").await;
        assert!(!out.success);
        assert!(out.final_text.contains("metadata"));
    }
}
