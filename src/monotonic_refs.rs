use std::sync::{Arc, Mutex, OnceLock};

pub(crate) struct MonotonicRefs {
    pub(crate) last_timestamp: u64,
    pub(crate) last_random: u64,
}

pub(crate) static MONOTONIC_REFS: OnceLock<Arc<Mutex<MonotonicRefs>>> = OnceLock::new();

pub(crate) fn get_monotonic_refs() -> Arc<Mutex<MonotonicRefs>> {
    MONOTONIC_REFS
        .get_or_init(|| {
            Arc::new(Mutex::new(MonotonicRefs {
                last_random: 0,
                last_timestamp: 0,
            }))
        })
        .clone()
}

pub(crate) fn reset_monotonic_refs() {
    let refs = get_monotonic_refs();
    let mut guard = refs.lock().unwrap();
    guard.last_timestamp = 0;
    guard.last_random = 0;
}
