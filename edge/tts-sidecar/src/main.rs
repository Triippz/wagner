//! Wagner TTS sidecar — Kokoro-82M ONNX, OpenAI-compatible /v1/audio/speech.
//!
//! Configuration (env vars override CLI flags):
//!   --port / WAGNER_TTS_PORT   — listen port (default 8772)
//!   WAGNER_TTS_MODEL           — path to model_quantized.onnx
//!   WAGNER_TTS_VOICES          — path to voices-v1.0.bin
//!
//! Usage:
//!   wagner-tts-sidecar --port 8772 --model /path/to/model.onnx --voices /path/to/voices.bin
//!
//! License chain: misaki-rs (MIT) · ort (MIT) · Kokoro weights (Apache-2.0)

mod kokoro;
mod server;
mod vocab;
mod voices;
mod wav;

use server::AppState;

fn main() {
    let args: Vec<String> = std::env::args().collect();

    // --- port ---
    let port: u16 = std::env::var("WAGNER_TTS_PORT")
        .ok()
        .and_then(|v| v.parse().ok())
        .or_else(|| flag_value(&args, "--port").and_then(|v| v.parse().ok()))
        .unwrap_or(8772);

    // --- model path ---
    let model_path = std::env::var("WAGNER_TTS_MODEL")
        .ok()
        .or_else(|| flag_value(&args, "--model"))
        .unwrap_or_else(|| {
            eprintln!(
                "[tts] ERROR: model path not set — use --model or WAGNER_TTS_MODEL env var"
            );
            std::process::exit(1);
        });

    // --- voices path ---
    let voices_path = std::env::var("WAGNER_TTS_VOICES")
        .ok()
        .or_else(|| flag_value(&args, "--voices"))
        .unwrap_or_else(|| {
            eprintln!(
                "[tts] ERROR: voices path not set — use --voices or WAGNER_TTS_VOICES env var"
            );
            std::process::exit(1);
        });

    let listen_addr = format!("127.0.0.1:{port}");

    let state = AppState::new(&model_path, &voices_path).unwrap_or_else(|e| {
        eprintln!("[tts] startup failed: {e}");
        std::process::exit(1);
    });

    server::run(state, &listen_addr);
}

/// Return the value that follows `--flag value` in `args`, if present.
fn flag_value(args: &[String], flag: &str) -> Option<String> {
    args.windows(2)
        .find(|w| w[0] == flag)
        .map(|w| w[1].clone())
}
