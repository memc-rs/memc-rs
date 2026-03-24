use std::sync::atomic::{AtomicUsize, Ordering};

pub fn get_worker_thread_name() -> String {
    static ATOMIC_ID: AtomicUsize = AtomicUsize::new(0);
    let id = ATOMIC_ID.fetch_add(1, Ordering::SeqCst);
    let str = format!("memcrsd-wrk-{}", id);
    str
}
