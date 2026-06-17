//! Wikilink rewriter — pure function, no I/O.
//!
//! The rename index maps uuid → current display name. Callers build a prior-name → current-name
//! lookup by diffing an old snapshot of the index against the current one; that diff is passed
//! to [`rewrite_wikilinks_renamed`]. The top-level [`rewrite_wikilinks`] is the public API that
//! accepts the current uuid-keyed index; see tests for usage.

use std::collections::HashMap;

use uuid::Uuid;

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
