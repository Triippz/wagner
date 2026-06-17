//! Wikilink rewrite tests — rewrite_wikilinks_renamed behaviour.

use std::collections::HashMap;
use wagner_edge_host::vault::linker::rewrite_wikilinks_renamed;

#[test]
fn test_rewrite_single_rename() {
    let mut renames = HashMap::new();
    renames.insert("old-name".to_string(), "new-name".to_string());

    let result = rewrite_wikilinks_renamed("See [[old-name]] for details.", &renames);
    assert_eq!(result, "See [[new-name]] for details.");
}

#[test]
fn test_no_op_when_name_unchanged() {
    let mut renames = HashMap::new();
    renames.insert("other-note".to_string(), "renamed-note".to_string());

    let original = "See [[my-note]] for details.";
    let result = rewrite_wikilinks_renamed(original, &renames);
    assert_eq!(result, original, "unrenamed wikilinks must be preserved verbatim");
}

#[test]
fn test_unknown_wikilink_preserved() {
    let renames: HashMap<String, String> = HashMap::new();
    let original = "See [[unknown]] for details.";
    let result = rewrite_wikilinks_renamed(original, &renames);
    assert_eq!(result, original, "unknown wikilinks must be preserved verbatim");
}

#[test]
fn test_multiple_renames_in_body() {
    let mut renames = HashMap::new();
    renames.insert("alpha".to_string(), "Alpha".to_string());
    renames.insert("beta".to_string(), "Beta".to_string());

    let result = rewrite_wikilinks_renamed("[[alpha]] and [[beta]] and [[gamma]]", &renames);
    assert_eq!(result, "[[Alpha]] and [[Beta]] and [[gamma]]");
}
