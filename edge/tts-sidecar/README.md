# wagner-tts-sidecar

Kokoro-82M ONNX text-to-speech sidecar for the Wagner voice pillar.
Exposes an OpenAI-compatible `POST /v1/audio/speech` endpoint.
Listens on `127.0.0.1:8772` by default.

## Quick start

```bash
# Set model paths (files are gitignored — download separately)
export WAGNER_TTS_MODEL=/path/to/model_quantized.onnx
export WAGNER_TTS_VOICES=/path/to/voices-v1.0.bin

cargo run -p wagner-tts-sidecar
```

Or with CLI flags:

```bash
cargo run -p wagner-tts-sidecar -- \
  --model /path/to/model_quantized.onnx \
  --voices /path/to/voices-v1.0.bin \
  --port 8772
```

## Configuration

| Source              | Variable / Flag          | Default           |
|---------------------|--------------------------|-------------------|
| Environment         | `WAGNER_TTS_PORT`        | `8772`            |
| Environment / flag  | `WAGNER_TTS_MODEL`       | _(required)_      |
| Environment / flag  | `WAGNER_TTS_VOICES`      | _(required)_      |
| CLI flag            | `--port <n>`             | `8772`            |
| CLI flag            | `--model <path>`         | _(required)_      |
| CLI flag            | `--voices <path>`        | _(required)_      |

Environment variables take precedence over CLI flags.

## Model files

| File | URL |
|------|-----|
| `model_quantized.onnx` | <https://huggingface.co/hexgrad/Kokoro-82M/resolve/main/kokoro-v0_19.onnx> |
| `voices-v1.0.bin`      | <https://huggingface.co/hexgrad/Kokoro-82M/resolve/main/voices-v1.0.bin>   |

Model files are excluded from git (`.gitignore`: `*.onnx`, `*.bin`).

## License chain

| Dependency     | License    | Notes                                  |
|----------------|------------|----------------------------------------|
| `misaki-rs`    | MIT        | Pure-Rust G2P, no espeak (no GPL risk) |
| `ort`          | MIT        | ONNX Runtime Rust bindings             |
| Kokoro weights | Apache-2.0 | hexgrad/Kokoro-82M on Hugging Face     |

No GPL dependencies. Safe for proprietary distribution.
