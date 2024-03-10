use crate::memcache::store::MemcStore;

use crate::memory_store::moka_store::MemoryStore as MokaStore;
use crate::memory_store::store::MemoryStore;
use crate::server::timer;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

pub struct MockSystemTimer {
    pub current_time: AtomicUsize,
}

pub trait SetableTimer: timer::Timer {
    fn set(&self, time: u64);
}

impl MockSystemTimer {
    pub fn new() -> Self {
        MockSystemTimer {
            current_time: AtomicUsize::new(0),
        }
    }
}

impl timer::Timer for MockSystemTimer {
    fn timestamp(&self) -> u64 {
        self.current_time.load(Ordering::Relaxed) as u64
    }
}

impl SetableTimer for MockSystemTimer {
    fn set(&self, time: u64) {
        self.current_time.store(time as usize, Ordering::Relaxed)
    }
}

pub struct MockServer {
    pub timer: Arc<MockSystemTimer>,
    pub storage: MemcStore,
}

impl MockServer {
    pub fn new() -> Self {
        let timer = Arc::new(MockSystemTimer::new());
        let store = Arc::new(MokaStore::new(timer.clone(), 1024 * 1024));
        MockServer {
            timer: timer,
            storage: MemcStore::new(store),
        }
    }
}

pub fn create_server() -> MockServer {
    MockServer::new()
}

pub fn create_storage() -> Arc<MemcStore> {
    let timer = Arc::new(MockSystemTimer::new());
    Arc::new(MemcStore::new(Arc::new(MemoryStore::new(timer))))
}
