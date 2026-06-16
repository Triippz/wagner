//! Scheduler — concurrency cap and worktree-isolation decisions (R-ISOLATION).
//!
//! Pure planning logic: given a set of planned subtasks and their declared
//! write-paths, decide (a) how many run at once and (b) which need an isolated
//! git worktree because their write-paths overlap a concurrently-running peer.

use std::collections::HashSet;

/// Default concurrency cap: min(CPU - 2, 4), floored at 1 (R-ISOLATION).
pub fn default_concurrency(cpus: usize) -> usize {
    cpus.saturating_sub(2).clamp(1, 4)
}

/// Whether two subtasks' declared write-path sets overlap.
pub fn paths_overlap(a: &[String], b: &[String]) -> bool {
    let sa: HashSet<&String> = a.iter().collect();
    b.iter().any(|p| sa.contains(p))
}

/// Given the write-path sets of the subtasks selected to run together in one
/// wave, return the indices that MUST run in an isolated worktree because they
/// collide with at least one peer in the wave. Read-only subtasks (empty paths)
/// never need isolation.
pub fn isolation_required(wave: &[Vec<String>]) -> Vec<usize> {
    let mut required = Vec::new();
    for (i, paths_i) in wave.iter().enumerate() {
        if paths_i.is_empty() {
            continue;
        }
        let collides = wave
            .iter()
            .enumerate()
            .any(|(j, paths_j)| i != j && paths_overlap(paths_i, paths_j));
        if collides {
            required.push(i);
        }
    }
    required
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn concurrency_default_is_min_cpu_minus_two_and_four() {
        assert_eq!(default_concurrency(16), 4);
        assert_eq!(default_concurrency(8), 4);
        assert_eq!(default_concurrency(5), 3);
        assert_eq!(default_concurrency(4), 2);
        assert_eq!(default_concurrency(2), 1); // floor
        assert_eq!(default_concurrency(1), 1); // floor
    }

    #[test]
    fn detects_overlap() {
        assert!(paths_overlap(
            &["src/a.rs".into(), "src/b.rs".into()],
            &["src/b.rs".into()]
        ));
        assert!(!paths_overlap(&["src/a.rs".into()], &["src/c.rs".into()]));
    }

    #[test]
    fn isolates_only_colliding_writers() {
        let wave = vec![
            vec!["src/a.rs".to_string()],                // 0 collides with 2
            vec!["src/b.rs".to_string()],                // 1 unique
            vec!["src/a.rs".to_string(), "x.rs".into()], // 2 collides with 0
            vec![],                                      // 3 read-only
        ];
        assert_eq!(isolation_required(&wave), vec![0, 2]);
    }

    #[test]
    fn no_isolation_when_all_disjoint() {
        let wave = vec![
            vec!["a".to_string()],
            vec!["b".to_string()],
            vec!["c".to_string()],
        ];
        assert!(isolation_required(&wave).is_empty());
    }

    #[test]
    fn read_only_never_isolated() {
        let wave = vec![vec![], vec![]];
        assert!(isolation_required(&wave).is_empty());
    }
}
