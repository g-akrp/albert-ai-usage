//! Runs independent jobs concurrently, one thread each, and returns
//! results in input order. Providers are slow (they shell out or hit
//! the network), so the menu waits for the slowest one rather than
//! the sum of all of them.

use std::thread;

/// Applies `f` to every item on its own scoped thread. A panicking
/// job yields `on_panic(item_index)` instead of taking the process
/// down.
pub fn map_parallel<T, R, F, P>(items: Vec<T>, f: F, on_panic: P) -> Vec<R>
where
    T: Send,
    R: Send,
    F: Fn(T) -> R + Sync,
    P: Fn(usize) -> R,
{
    thread::scope(|scope| {
        let f = &f;
        let handles: Vec<_> = items
            .into_iter()
            .map(|item| scope.spawn(move || f(item)))
            .collect();
        handles
            .into_iter()
            .enumerate()
            .map(|(i, h)| h.join().unwrap_or_else(|_| on_panic(i)))
            .collect()
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{Duration, Instant};

    #[test]
    fn keeps_input_order_even_when_later_jobs_finish_first() {
        let out = map_parallel(
            vec![30u64, 0, 10],
            |ms| {
                thread::sleep(Duration::from_millis(ms));
                ms
            },
            |_| 999,
        );
        assert_eq!(out, vec![30, 0, 10]);
    }

    #[test]
    fn runs_concurrently_not_serially() {
        let start = Instant::now();
        map_parallel(
            vec![150u64; 4],
            |ms| thread::sleep(Duration::from_millis(ms)),
            |_| (),
        );
        assert!(start.elapsed() < Duration::from_millis(450));
    }

    #[test]
    fn panicking_job_becomes_fallback_value() {
        let out = map_parallel(
            vec![1, 2, 3],
            |n| {
                if n == 2 {
                    panic!("boom");
                }
                n
            },
            |i| -(i as i32) - 100,
        );
        assert_eq!(out, vec![1, -101, 3]);
    }
}
