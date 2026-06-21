//! Best-effort spoken-cancel matcher (spec 015, FR-005a).
//!
//! The council decision (spec.md §Clarifications, 2026-06-21) put the *deterministic*
//! stop on a **physical** control. This matcher is the **flexible, best-effort**
//! convenience path: it recognizes a natural spoken cancel ("stop", "never mind",
//! "knock it off") and routes it to the same cancel action — but it is explicitly
//! NOT the safety guarantee.
//!
//! Safety rule (FR-005a): only fire on a **short utterance with no trailing
//! goal-like content**. We accept an open set of cancel "lead" phrasings, tolerate
//! a few trailing filler words, and treat anything with real trailing content as
//! free-form. So "stop" / "stop it now" / "never mind" cancel, but "stop wasting
//! tokens" and "let's stop here" are free-form goals/steers — never an accidental
//! abort.
//!
//! This is pure, deterministic string logic (no audio, no bus): the STT upstream is
//! the only non-deterministic part, which is exactly why this path is best-effort.

/// What a transcript should be routed to on the intake path.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpokenIntent {
    /// A spoken cancel — route to the run-cancel action (best-effort).
    Cancel,
    /// Anything else — route as a free-form goal/steer to the agent.
    FreeForm,
}

/// Cancel "lead" phrasings, normalized + lowercase. Multiword phrases are matched
/// as whole-utterance or as a prefix followed only by filler. Order does not matter
/// for correctness (exact match is checked before prefix), but keep them lowercase.
// ponytail: a curated open list, not an ML intent model — the physical control is the
// guarantee, so a missed phrasing is harmless. Add phrasings here as they come up.
const CANCEL_LEADS: &[&str] = &[
    "never mind",
    "nevermind",
    "knock it off",
    "cut it out",
    "that's enough",
    "thats enough",
    "forget it",
    "stop",
    "cancel",
    "abort",
    "halt",
    "quit",
];

/// Trailing tokens allowed after a lead without making the utterance free-form —
/// addressing, politeness, and pronoun fillers. Every trailing token must be a
/// filler for the utterance to still count as cancel.
const FILLERS: &[&str] = &[
    "it", "that", "this", "now", "please", "wagner", "everything", "ok", "okay", "then",
];

/// Classify a transcript as a best-effort spoken cancel or free-form (FR-005a).
pub fn classify_spoken(transcript: &str) -> SpokenIntent {
    let normalized = normalize(transcript);
    if normalized.is_empty() {
        return SpokenIntent::FreeForm;
    }

    for lead in CANCEL_LEADS {
        if normalized == *lead {
            return SpokenIntent::Cancel;
        }
        // Prefix match must be on a word boundary (the trailing space) so "stop"
        // does not match "stopwatch".
        if let Some(rest) = normalized.strip_prefix(&format!("{lead} ")) {
            if rest.split_whitespace().all(|tok| FILLERS.contains(&tok)) {
                return SpokenIntent::Cancel;
            }
            // A lead followed by real (non-filler) content is a free-form utterance
            // (e.g. "stop wasting tokens", "cancel the deployment"). Stop scanning —
            // a later lead cannot rescue trailing goal content.
            return SpokenIntent::FreeForm;
        }
    }

    SpokenIntent::FreeForm
}

/// Lowercase, trim, drop surrounding/trailing punctuation, and collapse internal
/// whitespace. Internal apostrophes are kept ("that's enough").
fn normalize(s: &str) -> String {
    let lowered = s.to_lowercase();
    let trimmed = lowered.trim_matches(|c: char| c.is_whitespace() || matches!(c, '.' | ',' | '!' | '?' | ';' | ':'));
    trimmed.split_whitespace().collect::<Vec<_>>().join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cancels(s: &str) -> bool {
        classify_spoken(s) == SpokenIntent::Cancel
    }

    #[test]
    fn bare_cancel_words_cancel() {
        for s in ["stop", "Stop", "STOP.", "  stop  ", "cancel", "abort", "halt", "quit"] {
            assert!(cancels(s), "{s:?} should cancel");
        }
    }

    #[test]
    fn multiword_cancel_phrases_cancel() {
        for s in ["never mind", "nevermind", "knock it off", "cut it out", "that's enough", "forget it"] {
            assert!(cancels(s), "{s:?} should cancel");
        }
    }

    #[test]
    fn lead_plus_filler_cancels() {
        for s in ["stop it", "stop now", "stop please", "stop wagner", "cancel that", "stop it now please"] {
            assert!(cancels(s), "{s:?} should cancel (filler trailing)");
        }
    }

    #[test]
    fn trailing_goal_content_is_freeform() {
        // The critical safety case from the council: a steer that merely contains a
        // cancel word must NOT abort the run.
        for s in [
            "stop wasting tokens",
            "cancel the deployment and start over",
            "abort the mission rewrite",
            "halt the indexing job please and restart it differently",
        ] {
            assert_eq!(classify_spoken(s), SpokenIntent::FreeForm, "{s:?} must be free-form");
        }
    }

    #[test]
    fn cancel_word_mid_sentence_is_freeform() {
        // Lead must be at the start; "stop" buried in a goal is free-form.
        for s in ["let's stop here", "research stop conditions for the loop", "we should not stop"] {
            assert_eq!(classify_spoken(s), SpokenIntent::FreeForm, "{s:?} must be free-form");
        }
    }

    #[test]
    fn lead_is_not_a_prefix_of_a_longer_word() {
        // "stopwatch" / "cancellation" must not match the "stop"/"cancel" lead.
        assert_eq!(classify_spoken("stopwatch the build"), SpokenIntent::FreeForm);
        assert_eq!(classify_spoken("cancellation policy draft"), SpokenIntent::FreeForm);
    }

    #[test]
    fn empty_is_freeform() {
        assert_eq!(classify_spoken(""), SpokenIntent::FreeForm);
        assert_eq!(classify_spoken("   "), SpokenIntent::FreeForm);
    }
}
