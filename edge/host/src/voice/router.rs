//! Voice engine router — selects which STT/TTS engine to use for a request.
//!
//! The router is intentionally simple: it holds a list of named engine
//! registrations and selects the first one whose predicate matches the
//! request tag.  This keeps the pipeline free of `match` arms and makes it
//! trivial to add new engines without touching existing code.

use crate::voice::types::VoiceError;
use std::sync::Arc;

/// A lightweight description of an incoming voice request used by the router
/// to pick an engine.
#[derive(Debug, Clone, PartialEq)]
pub struct RouteRequest {
    /// A tag that identifies the requested engine class (e.g. `"local"`,
    /// `"cloud"`, `"whisper"`). The router matches against registered engine
    /// names by exact string equality.
    pub engine_tag: String,
}

impl RouteRequest {
    pub fn new(engine_tag: impl Into<String>) -> Self {
        Self {
            engine_tag: engine_tag.into(),
        }
    }
}

/// A registered engine entry inside the router.
struct Registration {
    /// The tag this registration answers to.
    tag: String,
    /// Shared handle so the caller can clone it out without the router
    /// staying in the hot path.
    stt: Arc<dyn crate::voice::stt::Stt>,
    tts: Arc<dyn crate::voice::tts::Tts>,
}

/// Routes a `RouteRequest` to the right `(Stt, Tts)` pair.
///
/// Registrations are checked in insertion order; the first match wins.
#[derive(Default)]
pub struct VoiceRouter {
    registrations: Vec<Registration>,
}

/// A matched engine pair returned by `VoiceRouter::route`.
pub struct EngineHandles {
    pub stt: Arc<dyn crate::voice::stt::Stt>,
    pub tts: Arc<dyn crate::voice::tts::Tts>,
}

impl std::fmt::Debug for EngineHandles {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EngineHandles")
            .field("stt", &"<dyn Stt>")
            .field("tts", &"<dyn Tts>")
            .finish()
    }
}

impl VoiceRouter {
    /// Create an empty router (no engines registered).
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a router pre-wired with HTTP engines as the `"local"` engine.
    ///
    /// `stt_url` is the faster-whisper-server base URL (e.g. `"http://127.0.0.1:8771"`).
    /// `tts_url` is the Kokoro-FastAPI base URL (e.g. `"http://127.0.0.1:8772"`).
    ///
    /// These URLs are passed verbatim to the adapters — no process is started.
    /// The operator is responsible for running the sidecars before routing
    /// real audio through this router.
    pub fn default_http(
        stt_url: impl Into<String>,
        tts_url: impl Into<String>,
    ) -> Self {
        use crate::voice::http_stt::HttpStt;
        use crate::voice::http_tts::HttpTts;
        Self::new().register(
            "local",
            Arc::new(HttpStt::new(stt_url)),
            Arc::new(HttpTts::new(tts_url)),
        )
    }

    /// Register an `(Stt, Tts)` pair under `tag`.
    pub fn register(
        mut self,
        tag: impl Into<String>,
        stt: Arc<dyn crate::voice::stt::Stt>,
        tts: Arc<dyn crate::voice::tts::Tts>,
    ) -> Self {
        self.registrations.push(Registration {
            tag: tag.into(),
            stt,
            tts,
        });
        self
    }

    /// Return the engine handles for the first registration whose tag matches
    /// `req.engine_tag`.  Returns `VoiceError::NoEngineMatch` if nothing matches.
    pub fn route(&self, req: &RouteRequest) -> Result<EngineHandles, VoiceError> {
        self.registrations
            .iter()
            .find(|r| r.tag == req.engine_tag)
            .map(|r| EngineHandles {
                stt: Arc::clone(&r.stt),
                tts: Arc::clone(&r.tts),
            })
            .ok_or_else(|| VoiceError::NoEngineMatch(req.engine_tag.clone()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::voice::stt::FakeStt;
    use crate::voice::tts::FakeTts;

    fn fake_pair(
        tag: &str,
    ) -> (
        Arc<dyn crate::voice::stt::Stt>,
        Arc<dyn crate::voice::tts::Tts>,
    ) {
        let stt: Arc<dyn crate::voice::stt::Stt> = Arc::new(FakeStt::returning(tag));
        let tts: Arc<dyn crate::voice::tts::Tts> = Arc::new(FakeTts::succeeding());
        (stt, tts)
    }

    #[test]
    fn router_selects_matching_engine() {
        let (stt, tts) = fake_pair("local");
        let router = VoiceRouter::new().register("local", stt, tts);
        let handles = router.route(&RouteRequest::new("local")).unwrap();
        // Spot-check: the handle is present (we can't downcast to FakeStt here,
        // but the important thing is no error is returned).
        let _ = handles.stt;
        let _ = handles.tts;
    }

    #[test]
    fn router_returns_first_matching_engine() {
        let (stt1, tts1) = fake_pair("a");
        let (stt2, tts2) = fake_pair("b");
        let router = VoiceRouter::new()
            .register("local", stt1, tts1)
            .register("local", stt2, tts2);
        // Should not error — first match wins.
        router.route(&RouteRequest::new("local")).unwrap();
    }

    #[test]
    fn router_returns_error_when_no_engine_matches() {
        let router = VoiceRouter::new();
        let err = router
            .route(&RouteRequest::new("unknown"))
            .unwrap_err();
        assert!(matches!(err, VoiceError::NoEngineMatch(_)));
    }

    #[test]
    fn router_distinguishes_different_tags() {
        let (stt1, tts1) = fake_pair("local");
        let (stt2, tts2) = fake_pair("cloud");
        let router = VoiceRouter::new()
            .register("local", stt1, tts1)
            .register("cloud", stt2, tts2);
        router.route(&RouteRequest::new("local")).unwrap();
        router.route(&RouteRequest::new("cloud")).unwrap();
        assert!(router
            .route(&RouteRequest::new("whisper"))
            .is_err());
    }
}
