//! T036 — Repo-scoped dev-context file read (FR-303, CL-202, EC-006, SC-002).
//!
//! Default-deny: an in-repo read succeeds; ANY path resolving outside the repo
//! root is refused — including `..` traversal, a symlink escape, and absolute
//! out-of-repo paths (`~/.ssh`, an out-of-repo `.env`). The guard canonicalizes
//! (resolving symlinks + `..`) before the scope check, so the refusal cannot be
//! tricked by path tricks (a stronger guarantee than a denylist).

use std::fs;
use std::os::unix::fs::symlink;
use wagner_edge_host::remote::devcontext::{check_repo_scope, FileAccess, RefusedReason};

fn unique(tag: &str) -> std::path::PathBuf {
    std::env::temp_dir().join(format!("wagner-scope-{}-{}", tag, std::process::id()))
}

#[test]
fn in_repo_read_is_allowed() {
    let root = unique("ok");
    let repo = root.join("repo");
    fs::create_dir_all(repo.join("src")).unwrap();
    fs::write(repo.join("src/main.rs"), b"fn main() {}").unwrap();

    match check_repo_scope(&repo, &repo.join("src/main.rs")) {
        FileAccess::Allowed(p) => assert!(p.ends_with("src/main.rs")),
        other => panic!("expected Allowed, got {other:?}"),
    }
    let _ = fs::remove_dir_all(&root);
}

#[test]
fn parent_traversal_escape_is_refused() {
    let root = unique("dotdot");
    let repo = root.join("repo");
    fs::create_dir_all(&repo).unwrap();
    fs::write(root.join("secret.txt"), b"top secret").unwrap();

    // repo/../secret.txt resolves outside the repo root.
    let escape = repo.join("../secret.txt");
    assert_eq!(
        check_repo_scope(&repo, &escape),
        FileAccess::Refused(RefusedReason::OutOfScope)
    );
    let _ = fs::remove_dir_all(&root);
}

#[test]
fn symlink_escape_is_refused() {
    let root = unique("symlink");
    let repo = root.join("repo");
    fs::create_dir_all(&repo).unwrap();
    fs::write(root.join("outside.txt"), b"outside").unwrap();
    // A symlink INSIDE the repo pointing OUTSIDE it.
    let link = repo.join("link-to-outside");
    symlink(root.join("outside.txt"), &link).unwrap();

    assert_eq!(
        check_repo_scope(&repo, &link),
        FileAccess::Refused(RefusedReason::OutOfScope)
    );
    let _ = fs::remove_dir_all(&root);
}

#[test]
fn absolute_out_of_repo_paths_are_refused() {
    let root = unique("abs");
    let repo = root.join("repo");
    fs::create_dir_all(&repo).unwrap();

    for outside in ["/etc/hosts", "/tmp"] {
        if std::path::Path::new(outside).exists() {
            assert_eq!(
                check_repo_scope(&repo, std::path::Path::new(outside)),
                FileAccess::Refused(RefusedReason::OutOfScope),
                "{outside} must be refused",
            );
        }
    }
    let _ = fs::remove_dir_all(&root);
}

#[test]
fn nonexistent_in_repo_path_is_refused_not_panicked() {
    let root = unique("missing");
    let repo = root.join("repo");
    fs::create_dir_all(&repo).unwrap();
    // A file that does not exist → refused (nothing to read), never a panic.
    assert_eq!(
        check_repo_scope(&repo, &repo.join("nope.rs")),
        FileAccess::Refused(RefusedReason::OutOfScope)
    );
    let _ = fs::remove_dir_all(&root);
}
