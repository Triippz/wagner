//! US2 end-to-end (no Claude): the real `gate-server.mjs` MCP server, driven
//! over stdio, forwards a permission request to the loopback `permission_server`,
//! which opens a transmission; answering it resolves the gate's MCP response.
//!
//! This exercises the entire cross-process channel — the exact path Claude's
//! `--permission-prompt-tool` will drive — using node + the registry only.

use serde_json::{json, Value};
use std::process::Stdio;
use std::sync::Arc;
use wagner_edge_host::permission_server::{handle_permission, serve};
use wagner_edge_host::transmissions::{Decision, TransmissionRegistry};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpListener;
use tokio::process::Command;

/// Per-run gate token the server requires and the gate must present (M2).
const TOKEN: &str = "e2e-gate-token";

fn gate_script() -> String {
    concat!(env!("CARGO_MANIFEST_DIR"), "/resources/gate-server.mjs").to_string()
}

#[tokio::test]
async fn gate_server_forwards_permission_and_returns_engineer_decision() {
    // node is a documented prereq; skip cleanly if it is somehow absent.
    if Command::new("node")
        .arg("--version")
        .output()
        .await
        .is_err()
    {
        eprintln!("node not available — skipping gate e2e");
        return;
    }

    // 1. Loopback permission server wired to a registry.
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    let reg = Arc::new(TransmissionRegistry::default());
    let reg_for_server = reg.clone();
    let handler = move |args: Value| {
        let reg = reg_for_server.clone();
        async move {
            handle_permission(&reg, &|_v| {}, "2026-06-13T00:00:00Z", args, 3600, &|| {}).await
        }
    };
    tokio::spawn(serve(listener, handler, TOKEN.to_string()));

    // 2. Spawn the real gate MCP server pointed at the loopback URL, carrying the
    //    per-run token so its forwarded requests authenticate (M2).
    let mut child = Command::new("node")
        .arg(gate_script())
        .env(
            "CONSTRUCT_GATE_URL",
            format!("http://127.0.0.1:{port}/permission"),
        )
        .env("CONSTRUCT_GATE_TOKEN", TOKEN)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn gate server");
    let mut stdin = child.stdin.take().unwrap();
    let mut lines = BufReader::new(child.stdout.take().unwrap()).lines();

    // 3. MCP handshake + a permission tool call (the can_use_tool envelope).
    let msgs = [
        json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05"}}),
        json!({"jsonrpc":"2.0","method":"notifications/initialized"}),
        json!({"jsonrpc":"2.0","id":2,"method":"tools/call","params":{
            "name":"approve",
            "arguments":{"tool_name":"Bash","input":{"command":"ls"},"tool_use_id":"toolu_E2E"}
        }}),
    ];
    for v in msgs {
        stdin.write_all(format!("{v}\n").as_bytes()).await.unwrap();
    }

    // 4. Concurrently: once the transmission is open, the engineer allows it.
    let reg_for_answer = reg.clone();
    tokio::spawn(async move {
        while reg_for_answer.open_count() == 0 {
            tokio::task::yield_now().await;
        }
        reg_for_answer.answer("toolu_E2E", Decision::Allow);
    });

    // 5. Read gate output until the id:2 tools/call response.
    let mut allow: Option<Value> = None;
    while let Ok(Some(line)) = lines.next_line().await {
        let Ok(msg): Result<Value, _> = serde_json::from_str(&line) else {
            continue;
        };
        if msg["id"] == json!(2) {
            let text = msg["result"]["content"][0]["text"].as_str().unwrap();
            allow = Some(serde_json::from_str(text).unwrap());
            break;
        }
    }
    let _ = child.kill().await;

    let allow = allow.expect("gate must return a decision for id:2");
    assert_eq!(allow["behavior"], "allow");
    assert_eq!(allow["updatedInput"]["command"], "ls");
}
