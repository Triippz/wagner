//! Agent-authored panels (Phase 5) — extract a UI-spec an agent emitted in its
//! output so the inspector can render the agent's own view of its work.
//!
//! Agents include a fenced ```ui-spec block (or a trailing JSON object carrying a
//! `blocks` array) in their final text; we pull it out and pass the raw JSON to
//! the frontend, which validates it against the whitelisted primitive vocabulary
//! before rendering (the frontend `validateUiSpec` is the security boundary — no
//! model-authored markup or code is ever executed). This is a structural gate:
//! it only forwards a plausible panel (an object with a non-empty `blocks` array).

use serde_json::Value;

/// Pull a UI-spec panel out of an agent's final text, if it emitted one.
///
/// Recognizes a fenced ```ui-spec … ``` block first; otherwise scans for a
/// balanced JSON object that contains a `blocks` array. Returns the raw JSON
/// (frontend validates/sanitizes). `None` when there's no plausible panel.
pub fn extract_panel(final_text: &str) -> Option<Value> {
    if let Some(v) = fenced_block(final_text, "ui-spec").and_then(parse_panel) {
        return Some(v);
    }
    // Bound the brace scan: a pathological multi-MB agent dump should not be
    // fully scanned on the IPC path. Panels are small; cap the fallback input.
    const MAX_SCAN: usize = 256 * 1024;
    if final_text.len() > MAX_SCAN {
        return None;
    }
    // Fall back to any balanced top-level object carrying "blocks".
    scan_objects(final_text).into_iter().find_map(parse_panel)
}

/// The whitelisted primitive vocabulary (must match the frontend `uiSpec.ts`).
/// The frontend validator is the security boundary; this is backend
/// defense-in-depth so a non-panel JSON object (or one with no known blocks)
/// never goes on the wire.
const KNOWN_KINDS: &[&str] = &[
    "text", "badge", "progress", "list", "code", "kv", "timeline",
];

/// Parse + structurally gate a candidate: an object whose `blocks` array holds at
/// least one block of a known kind.
fn parse_panel(candidate: &str) -> Option<Value> {
    let v: Value = serde_json::from_str(candidate.trim()).ok()?;
    let blocks = v.get("blocks")?.as_array()?;
    let has_known = blocks.iter().any(|b| {
        b.get("kind")
            .and_then(|k| k.as_str())
            .is_some_and(|k| KNOWN_KINDS.contains(&k))
    });
    if !has_known {
        return None;
    }
    Some(v)
}

/// Extract the body of a ```<lang> … ``` fenced block, if present.
fn fenced_block<'a>(text: &'a str, lang: &str) -> Option<&'a str> {
    let open = format!("```{lang}");
    let start = text.find(&open)? + open.len();
    let rest = &text[start..];
    // skip to end of the opening fence line
    let body_start = rest.find('\n').map(|i| i + 1).unwrap_or(0);
    let body = &rest[body_start..];
    let end = body.find("```")?;
    Some(&body[..end])
}

/// Return every balanced `{…}` top-level object substring (cheap brace scan).
fn scan_objects(text: &str) -> Vec<&str> {
    let bytes = text.as_bytes();
    let mut out = Vec::new();
    let mut depth = 0usize;
    let mut start = 0usize;
    let mut in_str = false;
    let mut escaped = false;
    for (i, &b) in bytes.iter().enumerate() {
        if in_str {
            if escaped {
                escaped = false;
            } else if b == b'\\' {
                escaped = true;
            } else if b == b'"' {
                in_str = false;
            }
            continue;
        }
        match b {
            b'"' => in_str = true,
            b'{' => {
                if depth == 0 {
                    start = i;
                }
                depth += 1;
            }
            b'}' => {
                if depth > 0 {
                    depth -= 1;
                    if depth == 0 {
                        out.push(&text[start..=i]);
                    }
                }
            }
            _ => {}
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_a_fenced_ui_spec_block() {
        let text = "Here is my work.\n\n```ui-spec\n{\"title\":\"T\",\"blocks\":[{\"kind\":\"text\",\"text\":\"done\"}]}\n```\nthanks";
        let v = extract_panel(text).expect("panel");
        assert_eq!(v["title"], "T");
        assert_eq!(v["blocks"][0]["kind"], "text");
    }

    #[test]
    fn extracts_a_bare_object_with_blocks() {
        let text = "prose then {\"blocks\":[{\"kind\":\"badge\",\"label\":\"ok\"}]} trailing";
        let v = extract_panel(text).expect("panel");
        assert_eq!(v["blocks"][0]["label"], "ok");
    }

    #[test]
    fn no_panel_when_absent_or_empty() {
        assert!(extract_panel("just a normal summary, no json").is_none());
        assert!(extract_panel("{\"blocks\":[]}").is_none(), "empty blocks");
        assert!(extract_panel("{\"notblocks\":1}").is_none());
    }

    #[test]
    fn rejects_blocks_with_only_unknown_kinds() {
        // Defense-in-depth: a JSON object that happens to have a `blocks` array
        // but no whitelisted block kind is not a panel.
        assert!(extract_panel("{\"blocks\":[{\"kind\":\"script\",\"src\":\"x\"}]}").is_none());
    }

    #[test]
    fn ignores_braces_inside_strings() {
        // A stray "{" in prose must not confuse the scanner.
        let text = "note: use {curly} braces. {\"blocks\":[{\"kind\":\"text\",\"text\":\"x\"}]}";
        let v = extract_panel(text).expect("panel");
        assert_eq!(v["blocks"][0]["text"], "x");
    }
}
