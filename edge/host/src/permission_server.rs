//! Localhost permission server (US2 cross-process channel).
//!
//! The MCP gate that `claude` spawns lives in a *different process* than the
//! Construct app. When Claude wants to use a gated tool it calls the gate's
//! `--permission-prompt-tool`; the gate forwards the `can_use_tool` payload to
//! THIS server over `127.0.0.1`. We open a `Transmission`, emit it to the floor,
//! block until the engineer answers, and return Claude's permission JSON
//! (`{"behavior":"allow","updatedInput":…}` / `{"behavior":"deny","message":…}`).
//!
//! The HTTP transport is intentionally hand-rolled: a single localhost POST
//! endpoint with a client we ship ourselves does not justify a full HTTP stack.
//! Transport (`serve`) and policy (`handle_permission`) are split so each is
//! tested in isolation.

use crate::transmissions::{
    parse_permission_request, permission_response, Decision, TransmissionRegistry,
};
use serde_json::Value;
use std::future::Future;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

/// Socket read chunk + initial header-buffer capacity.
const READ_CHUNK: usize = 1024;
/// Hard cap on a request body before allocation — a hand-rolled parser must not
/// honor a forged `Content-Length` and allocate gigabytes (M3, local DoS).
const MAX_BODY: usize = 16 * 1024;
/// The per-run auth header the gate MCP server must present (M2).
const GATE_TOKEN_HEADER: &str = "x-gate-token";

/// Turn one permission request (the gate's forwarded `arguments` object) into a
/// floor transmission, wait for the engineer's decision, and return the
/// permission-response JSON Claude expects. Denies if the request is malformed
/// or the transmission is cancelled (channel dropped).
///
/// On a blocked-too-long timeout it denies-and-continues (so the CLI is never
/// left hanging) AND invokes `on_timeout`, which the app uses to promote the
/// stall into a whole-run `BlockedTimeout` halt (T042/FR-016).
pub async fn handle_permission(
    reg: &TransmissionRegistry,
    emit: &(dyn Fn(Value) + Send + Sync),
    raised_at: &str,
    args: Value,
    blocked_timeout_secs: u64,
    on_timeout: &(dyn Fn() + Send + Sync),
) -> Value {
    let Some(req) = parse_permission_request(&args) else {
        return serde_json::to_value(permission_response(Decision::Deny, &Value::Null)).unwrap();
    };
    // Correlate the transmission to Claude's tool call when we have its id.
    let id = req
        .tool_use_id
        .clone()
        .unwrap_or_else(|| ulid::Ulid::new().to_string());
    let subtask_id = id.clone();

    // Register the waiter BEFORE emitting, so an instant answer can't race us.
    let rx = reg.open(id.clone());
    emit(crate::transmissions::request_to_transmission_json(
        &req,
        &id,
        &subtask_id,
        raised_at,
    ));

    // Fail safe: if the engineer never answers within the blocked-too-long
    // window, cancel the transmission and deny — the run must never hang.
    let timeout = std::time::Duration::from_secs(blocked_timeout_secs);
    let decision = match tokio::time::timeout(timeout, rx).await {
        Ok(Ok(d)) => d,
        _ => {
            reg.cancel(&id);
            on_timeout();
            Decision::Deny
        }
    };
    serde_json::to_value(permission_response(decision, &req.input)).unwrap()
}

/// Serve permission POSTs on `listener` until cancelled. Each connection is a
/// single `POST /permission` carrying the gate's `arguments` JSON; `handler`
/// maps that JSON to the response JSON. Every request must present the per-run
/// `expected_token` in `X-Gate-Token` — requests without it are rejected 401 so
/// no other local process can approve tool use (M2). Runs forever — spawn it.
pub async fn serve<H, Fut>(listener: TcpListener, handler: H, expected_token: String)
where
    H: Fn(Value) -> Fut + Clone + Send + Sync + 'static,
    Fut: Future<Output = Value> + Send + 'static,
{
    loop {
        let Ok((stream, _)) = listener.accept().await else {
            continue;
        };
        let handler = handler.clone();
        let token = expected_token.clone();
        tokio::spawn(async move {
            let _ = serve_connection(stream, handler, &token).await;
        });
    }
}

async fn serve_connection<H, Fut>(
    mut stream: TcpStream,
    handler: H,
    expected_token: &str,
) -> std::io::Result<()>
where
    H: Fn(Value) -> Fut,
    Fut: Future<Output = Value>,
{
    let Some((headers, body)) = read_request(&mut stream).await? else {
        return write_response(&mut stream, 400, &Value::Null).await;
    };
    // Authenticate before doing any work — a forged request never reaches the
    // engineer or the transmission registry (M2).
    if header_value(&headers, GATE_TOKEN_HEADER) != Some(expected_token) {
        let deny = serde_json::to_value(permission_response(Decision::Deny, &Value::Null))
            .unwrap_or(Value::Null);
        return write_response(&mut stream, 401, &deny).await;
    }
    let args: Value = serde_json::from_slice(&body).unwrap_or(Value::Null);
    let resp = handler(args).await;
    write_response(&mut stream, 200, &resp).await
}

/// Read an HTTP/1.1 request, returning `(raw header block, body bytes)` — or
/// `None` if the headers are malformed or the body exceeds [`MAX_BODY`]. Only the
/// `Content-Length` framing our own client uses is honored.
async fn read_request(stream: &mut TcpStream) -> std::io::Result<Option<(String, Vec<u8>)>> {
    let mut buf = Vec::with_capacity(READ_CHUNK);
    let mut chunk = [0u8; READ_CHUNK];
    // Read until end-of-headers.
    let headers_end = loop {
        if let Some(pos) = find_subslice(&buf, b"\r\n\r\n") {
            break pos + 4;
        }
        let n = stream.read(&mut chunk).await?;
        if n == 0 {
            return Ok(None);
        }
        buf.extend_from_slice(&chunk[..n]);
        // A request whose headers alone blow past the body cap is malformed.
        if buf.len() > MAX_BODY {
            return Ok(None);
        }
    };
    let header_str = String::from_utf8_lossy(&buf[..headers_end]).into_owned();
    let content_length = header_str
        .lines()
        .find_map(|l| {
            let (k, v) = l.split_once(':')?;
            k.trim()
                .eq_ignore_ascii_case("content-length")
                .then(|| v.trim().parse::<usize>().ok())
                .flatten()
        })
        .unwrap_or(0);
    // Cap BEFORE growing the body buffer — never trust the declared length (M3).
    if content_length > MAX_BODY {
        return Ok(None);
    }

    let mut body = buf[headers_end..].to_vec();
    while body.len() < content_length {
        let n = stream.read(&mut chunk).await?;
        if n == 0 {
            break;
        }
        body.extend_from_slice(&chunk[..n]);
        if body.len() > MAX_BODY {
            return Ok(None);
        }
    }
    body.truncate(content_length);
    Ok(Some((header_str, body)))
}

/// Find the value of an HTTP header by (case-insensitive) name in a raw header block.
fn header_value<'a>(headers: &'a str, name: &str) -> Option<&'a str> {
    headers.lines().find_map(|l| {
        let (k, v) = l.split_once(':')?;
        k.trim().eq_ignore_ascii_case(name).then(|| v.trim())
    })
}

async fn write_response(stream: &mut TcpStream, status: u16, body: &Value) -> std::io::Result<()> {
    let body = serde_json::to_vec(body).unwrap_or_default();
    let reason = match status {
        200 => "OK",
        401 => "Unauthorized",
        _ => "Bad Request",
    };
    let head = format!(
        "HTTP/1.1 {status} {reason}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        body.len()
    );
    stream.write_all(head.as_bytes()).await?;
    stream.write_all(&body).await?;
    stream.flush().await
}

fn find_subslice(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack.windows(needle.len()).position(|w| w == needle)
}
