//! Remote observability contract (T032/T042, plan §1.3).
//!
//! The single source of truth for the remote metric + span names. The live
//! metric/span emission is wired where a Prometheus/OTel backend is configured
//! (integration-level); these constants pin the names so emit sites and
//! dashboards agree. A test asserts they match the plan.

/// US2 metrics (host).
pub const M_REMOTE_SESSIONS_ACTIVE: &str = "wagner_remote_sessions_active";
pub const M_REMOTE_ATTACH_SECONDS: &str = "wagner_remote_attach_seconds";
pub const M_DISCOVERY_RESOLVE_TOTAL: &str = "wagner_discovery_resolve_total";

/// US3 metrics (host).
pub const M_REMOTE_ACTION_TOTAL: &str = "wagner_remote_action_total";
pub const M_DEVCTX_REFUSED_TOTAL: &str = "wagner_devctx_refused_total";

/// Trace spans.
pub const SPAN_REMOTE_ATTACH: &str = "remote.attach";
pub const SPAN_REMOTE_CONTROL: &str = "remote.control";
pub const SPAN_DEVCTX_CMD: &str = "devctx.cmd";
pub const SPAN_DEVCTX_FILE: &str = "devctx.file";

/// The labelled outcome of a remote action, for `M_REMOTE_ACTION_TOTAL`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActionOutcome {
    Ok,
    Refused,
    Error,
}

impl ActionOutcome {
    pub fn label(&self) -> &'static str {
        match self {
            ActionOutcome::Ok => "ok",
            ActionOutcome::Refused => "refused",
            ActionOutcome::Error => "error",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn metric_names_match_the_plan() {
        // Plan §1.3 — host metrics.
        assert_eq!(M_REMOTE_SESSIONS_ACTIVE, "wagner_remote_sessions_active");
        assert_eq!(M_REMOTE_ATTACH_SECONDS, "wagner_remote_attach_seconds");
        assert_eq!(M_REMOTE_ACTION_TOTAL, "wagner_remote_action_total");
        assert_eq!(M_DEVCTX_REFUSED_TOTAL, "wagner_devctx_refused_total");
        assert_eq!(M_DISCOVERY_RESOLVE_TOTAL, "wagner_discovery_resolve_total");
    }

    #[test]
    fn span_names_match_the_plan() {
        assert_eq!(SPAN_REMOTE_ATTACH, "remote.attach");
        assert_eq!(SPAN_REMOTE_CONTROL, "remote.control");
    }

    #[test]
    fn action_outcome_labels() {
        assert_eq!(ActionOutcome::Ok.label(), "ok");
        assert_eq!(ActionOutcome::Refused.label(), "refused");
        assert_eq!(ActionOutcome::Error.label(), "error");
    }
}
