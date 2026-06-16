//! Shared balanced-`{…}`-object scanning.
//!
//! CLIs wrap their JSON output in prose or ```json fences. The oracle (plan
//! parse) and judge (goal-met verdict) gates both need to recover the JSON
//! object(s) from such text. This single string- and escape-aware scanner backs
//! them, so a fix to the scan reaches every gate.

/// Every top-level balanced `{…}` object substring in `text`, in order.
pub fn balanced_objects(text: &str) -> Vec<&str> {
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

/// The first top-level balanced `{…}` object substring, if any.
pub fn first_balanced_object(text: &str) -> Option<&str> {
    balanced_objects(text).into_iter().next()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_object_amid_prose() {
        let s = "here is the plan:\n```json\n{\"a\": 1}\n```\nthanks";
        assert_eq!(first_balanced_object(s), Some("{\"a\": 1}"));
    }

    #[test]
    fn braces_inside_strings_are_not_delimiters() {
        let s = r#"{"k": "a } b { c"}"#;
        assert_eq!(balanced_objects(s), vec![s]);
    }

    #[test]
    fn returns_every_top_level_object() {
        let s = "{\"a\":1} noise {\"b\":2}";
        assert_eq!(balanced_objects(s), vec!["{\"a\":1}", "{\"b\":2}"]);
    }

    #[test]
    fn none_when_no_object() {
        assert_eq!(first_balanced_object("no json here"), None);
    }
}
