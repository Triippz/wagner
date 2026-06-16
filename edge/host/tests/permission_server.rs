//! US2 — localhost permission server: HTTP transport round-trip + the
//! request→transmission→answer→response policy wired to `TransmissionRegistry`.

use serde_json::{json, Value};
use std::sync::{Arc, Mutex};
use wagner_edge_host::permission_server::{handle_permission, serve};
use wagner_edge_host::transmissions::{Decision, TransmissionRegistry};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

const TOKEN: &str = "test-gate-token";

/// Build a raw HTTP/1.1 POST. `token` adds the `X-Gate-Token` header; `cl_override`
/// forges a `Content-Length` different from the real body (for the M3 cap test).
fn build_post(body: &str, token: Option<&str>, cl_override: Option<usize>) -> String {
    let cl = cl_override.unwrap_or(body.len());
    let mut head = format!(
        "POST /permission HTTP/1.1\r\nHost: localhost\r\nContent-Length: {cl}\r\nConnection: close\r\n"
    );
    if let Some(t) = token {
        head.push_str(&format!("X-Gate-Token: {t}\r\n"));
    }
    format!("{head}\r\n{body}")
}

async fn send(addr: std::net::SocketAddr, req: &str) -> String {
    let mut stream = TcpStream::connect(addr).await.unwrap();
    stream.write_all(req.as_bytes()).await.unwrap();
    let mut resp = String::new();
    stream.read_to_string(&mut resp).await.unwrap();
    resp
}

/// The gate POSTs a JSON body WITH its token; the server runs the handler and returns its JSON.
#[tokio::test]
async fn serve_round_trips_an_authenticated_post_over_http() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let handler = |args: Value| async move { json!({"behavior": "allow", "echo": args}) };
    tokio::spawn(serve(listener, handler, TOKEN.to_string()));

    let body = r#"{"tool_name":"Bash","input":{"command":"ls"},"tool_use_id":"t1"}"#;
    let resp = send(addr, &build_post(body, Some(TOKEN), None)).await;

    assert!(resp.contains("200 OK"), "expected 200, got: {resp}");
    let body_start = resp.find("\r\n\r\n").unwrap() + 4;
    let v: Value = serde_json::from_str(resp[body_start..].trim()).unwrap();
    assert_eq!(v["behavior"], "allow");
    assert_eq!(v["echo"]["tool_name"], "Bash");
}

/// M2 — a request without (or with a wrong) gate token is rejected 401 and the
/// handler never runs, so no other local process can approve tool use.
#[tokio::test]
async fn serve_rejects_a_request_without_the_gate_token() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let ran = Arc::new(std::sync::atomic::AtomicBool::new(false));
    let flag = ran.clone();
    let handler = move |_args: Value| {
        let flag = flag.clone();
        async move {
            flag.store(true, std::sync::atomic::Ordering::SeqCst);
            json!({"behavior": "allow"})
        }
    };
    tokio::spawn(serve(listener, handler, TOKEN.to_string()));

    let body = r#"{"tool_name":"Bash","input":{}}"#;
    // No token at all.
    let no_tok = send(addr, &build_post(body, None, None)).await;
    assert!(no_tok.contains("401 Unauthorized"), "missing token must 401: {no_tok}");
    // Wrong token.
    let bad_tok = send(addr, &build_post(body, Some("nope"), None)).await;
    assert!(bad_tok.contains("401 Unauthorized"), "wrong token must 401: {bad_tok}");
    let body_start = bad_tok.find("\r\n\r\n").unwrap() + 4;
    let v: Value = serde_json::from_str(bad_tok[body_start..].trim()).unwrap();
    assert_eq!(v["behavior"], "deny");
    assert!(
        !ran.load(std::sync::atomic::Ordering::SeqCst),
        "handler must NOT run for an unauthenticated request"
    );
}

/// M3 — a forged oversized Content-Length is refused (400) instead of triggering a
/// multi-gigabyte allocation.
#[tokio::test]
async fn serve_refuses_a_forged_oversized_content_length() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let handler = |_args: Value| async move { json!({"behavior": "allow"}) };
    tokio::spawn(serve(listener, handler, TOKEN.to_string()));

    // Declare ~4 GiB of body but send a tiny one — the server must cap and 400.
    let resp = send(addr, &build_post("{}", Some(TOKEN), Some(4_294_967_295))).await;
    assert!(
        resp.contains("400 Bad Request"),
        "oversized Content-Length must be rejected, got: {resp}"
    );
}

/// handle_permission opens a transmission keyed by the tool_use_id, emits it,
/// and resolves to Claude's allow response once the engineer answers.
#[tokio::test]
async fn handle_permission_opens_transmission_and_returns_decision() {
    let reg = Arc::new(TransmissionRegistry::default());
    let emitted = Arc::new(Mutex::new(Vec::<Value>::new()));
    let e = emitted.clone();
    let emit = move |v: Value| e.lock().unwrap().push(v);

    let args = json!({"tool_name":"Bash","input":{"command":"ls"},"tool_use_id":"toolu_X"});
    let reg_for_task = reg.clone();
    let task = tokio::spawn(async move {
        handle_permission(
            &reg_for_task,
            &emit,
            "2026-06-13T00:00:00Z",
            args,
            3600,
            &|| {},
        )
        .await
    });

    // Wait until the waiter is registered, then answer it.
    while reg.open_count() == 0 {
        tokio::task::yield_now().await;
    }
    assert!(reg.answer("toolu_X", Decision::Allow));

    let resp = task.await.unwrap();
    assert_eq!(resp["behavior"], "allow");
    assert_eq!(resp["updatedInput"]["command"], "ls");

    let em = emitted.lock().unwrap();
    assert_eq!(em.len(), 1);
    assert_eq!(em[0]["kind"], "permission");
    assert_eq!(em[0]["id"], "toolu_X");
}

/// An unanswered transmission fails safe: it times out, cancels, denies (so the
/// CLI never hangs — R-TIMEOUT / FR-016) AND fires `on_timeout` so the app can
/// promote the stall to a whole-run halt (T042).
#[tokio::test]
async fn handle_permission_denies_and_signals_on_blocked_timeout() {
    let reg = Arc::new(TransmissionRegistry::default());
    let args = json!({"tool_name":"Bash","input":{"command":"ls"},"tool_use_id":"toolu_T"});
    let timed_out = Arc::new(std::sync::atomic::AtomicBool::new(false));
    let flag = timed_out.clone();
    let on_timeout = move || flag.store(true, std::sync::atomic::Ordering::SeqCst);
    // Never answered + a zero-length window → the timeout branch fires.
    let resp = handle_permission(
        &reg,
        &|_v: Value| {},
        "2026-06-13T00:00:00Z",
        args,
        0,
        &on_timeout,
    )
    .await;
    assert_eq!(resp["behavior"], "deny");
    assert_eq!(
        reg.open_count(),
        0,
        "timed-out transmission must be cancelled"
    );
    assert!(
        timed_out.load(std::sync::atomic::Ordering::SeqCst),
        "on_timeout must fire so the run can halt"
    );
}

/// A denied transmission yields Claude's deny response.
#[tokio::test]
async fn handle_permission_denies_when_engineer_denies() {
    let reg = Arc::new(TransmissionRegistry::default());
    let emit = |_v: Value| {};
    let args = json!({"tool_name":"Write","input":{"path":"x"},"tool_use_id":"toolu_Y"});
    let reg_for_task = reg.clone();
    let task = tokio::spawn(async move {
        handle_permission(
            &reg_for_task,
            &emit,
            "2026-06-13T00:00:00Z",
            args,
            3600,
            &|| {},
        )
        .await
    });
    while reg.open_count() == 0 {
        tokio::task::yield_now().await;
    }
    assert!(reg.answer("toolu_Y", Decision::Deny));
    let resp = task.await.unwrap();
    assert_eq!(resp["behavior"], "deny");
    assert!(resp["message"].as_str().unwrap().contains("engineer"));
}
