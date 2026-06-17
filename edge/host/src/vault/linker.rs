//! Deterministic `[[wikilink]]` parsing and rewriting. Pure functions, no I/O, no LLM.
//!
//! **Parsing** (`parse_wikilinks`): a real scanner (not a regex) that extracts
//! wikilink targets from markdown, correctly ignoring fenced and inline code spans.
//!
//! **Rewriting** (`rewrite_wikilinks`, `rewrite_wikilinks_renamed`): Phase-5 (006)
//! additions that rewrite `[[name]]` tokens in-place given a rename map, used by
//! the CRDT projector when notes are renamed.

use std::collections::HashMap;

use uuid::Uuid;

// ── Parsing ──────────────────────────────────────────────────────────────────

/// One wikilink found in a note body. `display_name` is the link target as
/// written (resolved to a note UID elsewhere via the name index); `alias` is the
/// optional `[[target|alias]]` display override.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WikiLink {
    pub display_name: String,
    pub alias: Option<String>,
}

/// Extract every `[[wikilink]]` from markdown `body`, in document order. Skips
/// fenced code blocks (``` / ~~~) and inline code spans (`` `…` ``) so brackets
/// in code are never mistaken for links. UTF-8 safe (slices only at ASCII
/// delimiters found via `str::find`).
pub fn parse_wikilinks(body: &str) -> Vec<WikiLink> {
    let mut out = Vec::new();
    let mut in_fence = false;
    for line in body.lines() {
        let trimmed = line.trim_start();
        if trimmed.starts_with("```") || trimmed.starts_with("~~~") {
            in_fence = !in_fence;
            continue;
        }
        if in_fence {
            continue;
        }
        extract_line(&strip_inline_code(line), &mut out);
    }
    out
}

/// Drop inline-code spans so `[[x]]` inside backticks is ignored. ponytail:
/// single-backtick toggling; double-backtick (``code``) spans aren't special-
/// cased — add only if note bodies need them.
fn strip_inline_code(line: &str) -> String {
    let mut result = String::new();
    let mut in_code = false;
    for c in line.chars() {
        if c == '`' {
            in_code = !in_code;
            continue;
        }
        if !in_code {
            result.push(c);
        }
    }
    result
}

fn extract_line(text: &str, out: &mut Vec<WikiLink>) {
    let mut rest = text;
    while let Some(open) = rest.find("[[") {
        let after = &rest[open + 2..];
        let Some(close) = after.find("]]") else { break };
        let mut inner = &after[..close];
        // Nested `[[a [[b]]]]` → take the innermost target (after the last `[[`).
        if let Some(last) = inner.rfind("[[") {
            inner = &inner[last + 2..];
        }
        let (display, alias) = match inner.split_once('|') {
            Some((a, b)) => (a.trim().to_string(), Some(b.trim().to_string())),
            None => (inner.trim().to_string(), None),
        };
        if !display.is_empty() {
            out.push(WikiLink { display_name: display, alias });
        }
        rest = &after[close + 2..];
    }
}

// ── Rewriting (Phase 5 / 006) ─────────────────────────────────────────────────

/// Rewrite `[[display-name]]` wikilinks in `body`.
///
/// `index` maps note UUID to its CURRENT display name. Any `[[name]]` token that appears
/// in the index VALUES is preserved (already current). Any `[[name]]` not found is also
/// preserved (unknown link — out-of-scope for this index). This function is idempotent
/// over the current-name set; callers that need old→new rewriting should use
/// [`rewrite_wikilinks_renamed`] with an explicit rename map.
pub fn rewrite_wikilinks(body: &str, index: &HashMap<Uuid, String>) -> String {
    // ponytail: index values are the current names; any name present or absent is preserved verbatim.
    let _ = index;
    scan_wikilinks(body, |name| name.to_string())
}

/// Rewrite `[[old-name]]` wikilinks using an explicit old→new rename map.
///
/// Tokens not present in `renames` are preserved verbatim.
pub fn rewrite_wikilinks_renamed(body: &str, renames: &HashMap<String, String>) -> String {
    scan_wikilinks(body, |name| {
        renames.get(name).cloned().unwrap_or_else(|| name.to_string())
    })
}

/// Scan `[[...]]` tokens in `body`, applying `resolve` to each enclosed name.
fn scan_wikilinks(body: &str, mut resolve: impl FnMut(&str) -> String) -> String {
    if !body.contains("[[") {
        return body.to_string();
    }

    let mut result = String::with_capacity(body.len() + 16);
    let mut remaining = body;

    while let Some(open) = remaining.find("[[") {
        result.push_str(&remaining[..open]);
        let after_open = &remaining[open + 2..];
        if let Some(close) = after_open.find("]]") {
            let name = &after_open[..close];
            result.push_str("[[");
            result.push_str(&resolve(name));
            result.push_str("]]");
            remaining = &after_open[close + 2..];
        } else {
            // Unclosed bracket — emit literally and stop scanning.
            result.push_str("[[");
            result.push_str(after_open);
            remaining = "";
        }
    }
    result.push_str(remaining);
    result
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn names(body: &str) -> Vec<String> {
        parse_wikilinks(body).into_iter().map(|l| l.display_name).collect()
    }

    // — parse_wikilinks tests —

    #[test]
    fn simple_wikilink_extracted() {
        assert_eq!(names("See [[Auth Flow]]."), vec!["Auth Flow"]);
    }

    #[test]
    fn aliased_wikilink_extracted() {
        let links = parse_wikilinks("See [[auth-flow|Auth Flow]].");
        assert_eq!(links.len(), 1);
        assert_eq!(links[0].display_name, "auth-flow");
        assert_eq!(links[0].alias.as_deref(), Some("Auth Flow"));
    }

    #[test]
    fn wikilink_inside_code_fence_ignored() {
        let body = "before\n```\n[[Not a link]]\n```\nafter [[Real]]";
        assert_eq!(names(body), vec!["Real"]);
    }

    #[test]
    fn inline_code_wikilink_ignored() {
        assert_eq!(names("use `[[not-a-link]]` but [[Yes]]"), vec!["Yes"]);
    }

    #[test]
    fn multiple_links_in_one_body() {
        assert_eq!(
            names("[[A]] then [[B]] and [[C]]"),
            vec!["A", "B", "C"]
        );
    }

    #[test]
    fn no_links_returns_empty() {
        assert!(parse_wikilinks("plain prose, no brackets here").is_empty());
    }

    #[test]
    fn nested_brackets_take_innermost() {
        assert_eq!(names("[[a [[b]]]]"), vec!["b"]);
    }

    #[test]
    fn utf8_body_does_not_panic() {
        // Emoji + zero-width chars around a link must not break byte slicing.
        let body = "🚀 zero\u{200b}width [[Café Notes|résumé]] 🎯";
        let links = parse_wikilinks(body);
        assert_eq!(links.len(), 1);
        assert_eq!(links[0].display_name, "Café Notes");
    }

    #[test]
    fn unterminated_link_is_ignored() {
        assert!(parse_wikilinks("dangling [[never closed").is_empty());
    }

    // — rewrite_wikilinks_renamed tests —

    #[test]
    fn rewrite_single_rename() {
        let mut renames = HashMap::new();
        renames.insert("old-name".to_string(), "new-name".to_string());
        let result = rewrite_wikilinks_renamed("See [[old-name]] for details.", &renames);
        assert_eq!(result, "See [[new-name]] for details.");
    }

    #[test]
    fn rewrite_no_op_when_name_unchanged() {
        let mut renames = HashMap::new();
        renames.insert("other-note".to_string(), "renamed-note".to_string());
        let original = "See [[my-note]] for details.";
        let result = rewrite_wikilinks_renamed(original, &renames);
        assert_eq!(result, original, "unrenamed wikilinks must be preserved verbatim");
    }

    #[test]
    fn rewrite_unknown_wikilink_preserved() {
        let renames: HashMap<String, String> = HashMap::new();
        let original = "See [[unknown]] for details.";
        let result = rewrite_wikilinks_renamed(original, &renames);
        assert_eq!(result, original, "unknown wikilinks must be preserved verbatim");
    }

    #[test]
    fn rewrite_multiple_renames_in_body() {
        let mut renames = HashMap::new();
        renames.insert("alpha".to_string(), "Alpha".to_string());
        renames.insert("beta".to_string(), "Beta".to_string());
        let result = rewrite_wikilinks_renamed("[[alpha]] and [[beta]] and [[gamma]]", &renames);
        assert_eq!(result, "[[Alpha]] and [[Beta]] and [[gamma]]");
    }
}
