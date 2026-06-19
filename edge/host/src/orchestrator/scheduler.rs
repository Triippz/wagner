//! Scheduler — concurrency cap and worktree-isolation decisions (R-ISOLATION).
//!
//! Pure planning logic: given a set of planned subtasks and their declared
//! write-paths, decide (a) how many run at once and (b) which need an isolated
//! git worktree because their write-paths overlap a concurrently-running peer.

use std::collections::HashSet;

/// CPUs held back for the app/UI when sizing the concurrency cap.
const CPU_RESERVE: usize = 2;
/// Floor / ceiling on concurrent subtasks (R-ISOLATION).
const MIN_CONCURRENCY: usize = 1;
const MAX_CONCURRENCY: usize = 4;

/// Default concurrency cap: min(CPU - CPU_RESERVE, MAX), floored at MIN (R-ISOLATION).
pub fn default_concurrency(cpus: usize) -> usize {
    cpus.saturating_sub(CPU_RESERVE)
        .clamp(MIN_CONCURRENCY, MAX_CONCURRENCY)
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

/// Partition subtasks (by their declared write-path sets) into waves such that
/// within a wave no two subtasks' write-paths overlap (011 P5). Waves run
/// sequentially; the members of each wave run concurrently (up to the cap) with
/// no file-clobber risk. Read-only / empty-path subtasks overlap nothing, so they
/// pack into the earliest wave with room. Greedy, order-preserving.
pub fn non_overlapping_waves(write_paths: &[Vec<String>]) -> Vec<Vec<usize>> {
    let mut waves: Vec<Vec<usize>> = Vec::new();
    'next: for (i, paths_i) in write_paths.iter().enumerate() {
        for wave in waves.iter_mut() {
            if !wave.iter().any(|&j| paths_overlap(paths_i, &write_paths[j])) {
                wave.push(i);
                continue 'next;
            }
        }
        waves.push(vec![i]);
    }
    waves
}

/// Run `tasks` concurrently, at most `cap` in flight at once, collecting every
/// result (in completion order). The concurrency primitive behind parallel
/// subtask dispatch (011 P5): polling-based concurrency, no extra task spawns, so
/// the futures need not be `'static`.
pub async fn run_bounded<'a, T>(
    cap: usize,
    tasks: Vec<futures::future::BoxFuture<'a, T>>,
) -> Vec<T> {
    use futures::stream::StreamExt;
    futures::stream::iter(tasks).buffer_unordered(cap.max(1)).collect().await
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

    #[test]
    fn disjoint_and_readonly_subtasks_share_one_wave() {
        let paths = vec![vec!["a".to_string()], vec!["b".to_string()], vec![]];
        assert_eq!(non_overlapping_waves(&paths), vec![vec![0, 1, 2]]);
    }

    #[test]
    fn overlapping_writers_split_into_sequential_waves() {
        // 0 & 2 both write "a" → must be in different waves; 1 is disjoint.
        let paths = vec![
            vec!["a".to_string()],
            vec!["b".to_string()],
            vec!["a".to_string()],
        ];
        let waves = non_overlapping_waves(&paths);
        assert_eq!(waves, vec![vec![0, 1], vec![2]]);
    }

    #[tokio::test]
    async fn run_bounded_overlaps_but_caps_concurrency() {
        use std::sync::atomic::{AtomicUsize, Ordering};
        use std::sync::Arc;
        use std::time::Duration;

        let active = Arc::new(AtomicUsize::new(0));
        let max_seen = Arc::new(AtomicUsize::new(0));
        let cap = 3;

        let tasks: Vec<futures::future::BoxFuture<'static, ()>> = (0..6)
            .map(|_| {
                let active = active.clone();
                let max_seen = max_seen.clone();
                Box::pin(async move {
                    let now = active.fetch_add(1, Ordering::SeqCst) + 1;
                    max_seen.fetch_max(now, Ordering::SeqCst);
                    // Hold the slot briefly so peers overlap in wall-clock time.
                    tokio::time::sleep(Duration::from_millis(20)).await;
                    active.fetch_sub(1, Ordering::SeqCst);
                }) as futures::future::BoxFuture<'static, ()>
            })
            .collect();

        let results = run_bounded(cap, tasks).await;
        assert_eq!(results.len(), 6, "every task completes");
        let peak = max_seen.load(Ordering::SeqCst);
        assert!(peak > 1, "tasks ran concurrently (peak {peak})");
        assert!(peak <= cap, "concurrency never exceeded the cap (peak {peak})");
    }
}
