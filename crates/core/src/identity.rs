//! Allocation monotone d’identités de session, sans réutilisation ni dépendance
//! à une API atomique plus récente que le minimum Rust du projet.
use std::sync::atomic::{AtomicU64, Ordering};

pub fn next(counter: &AtomicU64) -> u64 {
    let mut current = counter.load(Ordering::Relaxed);
    loop {
        let following = current.checked_add(1).expect("identity space exhausted");
        match counter.compare_exchange_weak(
            current,
            following,
            Ordering::Relaxed,
            Ordering::Relaxed,
        ) {
            Ok(_) => return current,
            Err(actual) => current = actual,
        }
    }
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use super::*;
    use std::sync::Arc;

    #[test]
    fn concurrent_allocations_are_unique_and_monotone() {
        let counter = Arc::new(AtomicU64::new(1));
        let handles: Vec<_> = (0..8)
            .map(|_| {
                let counter = counter.clone();
                std::thread::spawn(move || (0..64).map(|_| next(&counter)).collect::<Vec<_>>())
            })
            .collect();
        let mut ids: Vec<_> = handles
            .into_iter()
            .flat_map(|handle| handle.join().unwrap())
            .collect();
        ids.sort_unstable();
        assert_eq!(ids, (1..=512).collect::<Vec<_>>());
    }

    #[test]
    fn exhaustion_never_wraps_or_reuses_an_identity() {
        let counter = AtomicU64::new(u64::MAX - 1);
        assert_eq!(next(&counter), u64::MAX - 1);
        for _ in 0..2 {
            assert!(std::panic::catch_unwind(|| next(&counter)).is_err());
            assert_eq!(counter.load(Ordering::Relaxed), u64::MAX);
        }
    }
}
