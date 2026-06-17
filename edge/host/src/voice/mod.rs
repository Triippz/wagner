//! Voice pillar — speech-to-text / text-to-speech infrastructure.
//!
//! # Architecture
//!
//! The Voice pillar is organised around two trait ports (`Stt`, `Tts`) and a
//! thin pipeline that sequences them.  A router selects the right engine pair
//! for each request.  No real audio, network, or Python code is compiled into
//! this module: production adapters slot in at the application boundary; tests
//! use `FakeStt` / `FakeTts` scripted doubles.
//!
//! ```text
//! AudioChunk
//!     │
//!     ▼
//! VoiceRouter ──► (Stt impl, Tts impl)
//!                         │
//!                         ▼
//!                 VoicePipeline::run
//!                         │
//!                 ┌───────┴───────┐
//!                 ▼               ▼
//!           Stt::transcribe  Tts::synthesise
//!                 │               │
//!                 └───────┬───────┘
//!                         ▼
//!                  PipelineResult
//! ```
//!
//! # Public surface
//!
//! | Item | Purpose |
//! |------|---------|
//! | `types::{AudioChunk, Transcript, SpeechChunk, VoiceError}` | Core domain types |
//! | `stt::{Stt, FakeStt}` | STT port + test double |
//! | `tts::{Tts, FakeTts}` | TTS port + test double |
//! | `http_stt::HttpStt` | Production STT adapter (faster-whisper-server) |
//! | `http_tts::HttpTts` | Production TTS adapter (Kokoro-FastAPI) |
//! | `router::{VoiceRouter, RouteRequest, EngineHandles}` | Engine selection |
//! | `pipeline::{VoicePipeline, PipelineResult}` | STT→TTS sequencing |

pub mod http_stt;
pub mod http_tts;
pub mod manager;
pub mod models;
pub mod pipeline;
pub mod router;
pub mod stt;
pub mod tts;
pub mod types;

// Convenience re-exports so callers can write `voice::HttpStt` / `voice::HttpTts`
// without reaching into submodules.
pub use http_stt::HttpStt;
pub use http_tts::HttpTts;

// Re-export the domain types and router items at this level for test convenience.
pub use router::{EngineHandles, RouteRequest, VoiceRouter};
pub use types::{AudioChunk, SpeechChunk, Transcript, VoiceError};

// VoiceManager and VoiceStatus re-exported at voice:: level for the shell layer.
pub use manager::{VoiceManager, VoiceStatus};

// Models download manager re-exported for the shell layer.
pub use models::{
    all_models_ready, download_models, models_status, ModelError, ModelProgress, ModelState,
    ModelsStatus, MODELS,
};
