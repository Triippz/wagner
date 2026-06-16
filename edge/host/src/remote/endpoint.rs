//! iroh endpoint + connection path policy (T027, FR-203, R-1/R-2, ADR-0003).
//!
//! The live iroh `Endpoint` (QUIC, Ed25519 node identity, per-ALPN channels) is
//! wired as integration-only code; the pieces tested headlessly here are the
//! transport-INDEPENDENT policy + types:
//!  - `select_path`: prefer a direct path, fall back to the org-run relay, fail
//!    gracefully when neither is available (US2-AS-2);
//!  - `RelayFrame`: the relay's view of traffic — an OPAQUE, size-only frame it
//!    can log but never decrypt (F-1; the relay is zero-knowledge of content);
//!  - `LoopbackStream`: an in-memory event channel used by attach tests in place
//!    of a live transport (D-TEST-4).

/// Which path an attach used.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectPath {
    /// A direct NAT-traversed iroh connection.
    Direct,
    /// The org-run relay fallback (still E2E-encrypted; relay is zero-knowledge).
    Relay,
}

/// Why a connection could not be formed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectError {
    /// Neither a direct path nor the relay is reachable — fail gracefully.
    NoPath,
}

/// Choose the connection path: direct preferred, relay fallback, else fail.
pub fn select_path(direct_available: bool, relay_available: bool) -> Result<ConnectPath, ConnectError> {
    if direct_available {
        Ok(ConnectPath::Direct)
    } else if relay_available {
        Ok(ConnectPath::Relay)
    } else {
        Err(ConnectError::NoPath)
    }
}

/// The org-run relay's view of one forwarded frame: it is an OPAQUE encrypted
/// blob. The relay can measure its size (logging / flow control) but has no
/// access to the plaintext — `RelayFrame` deliberately stores only the size, so
/// there is no field through which content could leak (F-1, SC-006).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RelayFrame {
    size: usize,
}

impl RelayFrame {
    /// Build a relay frame from a ciphertext blob — only its length is retained.
    pub fn from_ciphertext(ciphertext: &[u8]) -> Self {
        RelayFrame { size: ciphertext.len() }
    }

    pub fn size(&self) -> usize {
        self.size
    }

    /// The log line the relay records — size only, never payload.
    pub fn log_entry(&self) -> String {
        format!("relay_frame size={}", self.size)
    }
}

/// An in-memory event stream standing in for a live transport in attach tests
/// (D-TEST-4). Pushes are delivered to the attached client in order.
#[derive(Debug, Default)]
pub struct LoopbackStream {
    frames: Vec<String>,
}

impl LoopbackStream {
    pub fn new() -> Self {
        LoopbackStream { frames: Vec::new() }
    }

    /// Enqueue one host→client event frame.
    pub fn push(&mut self, frame: impl Into<String>) {
        self.frames.push(frame.into());
    }

    /// Take all delivered frames in order.
    pub fn drain(&mut self) -> Vec<String> {
        std::mem::take(&mut self.frames)
    }
}
