//! First-party bus participants (spec 011 P6) built on the `Agent` contract +
//! `AgentRegistry` (P4): the [`scheduler`] (time/event triggers that dispatch
//! commands) and the first [`slack`] connector (subscribes to intents, calls an
//! external API behind an injectable transport, publishes facts, owns its own
//! retry). They prove "a new agent = a new integration = the same move."

pub mod scheduler;
pub mod slack;
pub mod voice_intake;
pub mod voice_projection;

pub use scheduler::{ScheduledCommand, SchedulerAgent};
pub use slack::{SlackConnector, SlackTransport};
pub use voice_intake::{route_transcript, IntakeAction, VoiceIntake};
pub use voice_projection::{speakable_text, VoiceProjection};
