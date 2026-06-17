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
//! | `router::{VoiceRouter, RouteRequest, EngineHandles}` | Engine selection |
//! | `pipeline::{VoicePipeline, PipelineResult}` | STT→TTS sequencing |

pub mod pipeline;
pub mod router;
pub mod stt;
pub mod tts;
pub mod types;
