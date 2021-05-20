use crate::memcache::store::MemcStore;
use crate::storage::timer;
use crate::storage::store::KeyValueStore;
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
    fn secs(&self) -> u64 {
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
        let store = Arc::new(KeyValueStore::new(timer.clone()));
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
    Arc::new(MemcStore::new(Arc::new(KeyValueStore::new(timer))))
}
