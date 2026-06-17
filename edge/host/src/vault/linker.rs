//! Deterministic `[[wikilink]]` parsing. Pure functions, no I/O, no LLM. A real
//! scanner (not a regex) so links inside fenced/inline code are correctly
//! ignored and the canonical link target is recovered from `[[target|alias]]`.

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

#[cfg(test)]
mod tests {
    use super::*;

    fn names(body: &str) -> Vec<String> {
        parse_wikilinks(body).into_iter().map(|l| l.display_name).collect()
    }

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
}
