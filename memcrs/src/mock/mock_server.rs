use crate::memcache::store::MemcStore;

use crate::cache::cache::Cache;
use crate::memory_store::dash_map_store::DashMapMemoryStore;
use crate::memory_store::moka_store::MokaMemoryStore as MokaStore;
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
    pub fn new(store: Arc<dyn Cache + Send + Sync>, timer: Arc<MockSystemTimer>) -> Self {
        MockServer {
            timer: timer,
            storage: MemcStore::new(store),
        }
    }
}

pub fn create_moka_server() -> MockServer {
    let timer = Arc::new(MockSystemTimer::new());
    MockServer::new(Arc::new(MokaStore::new(timer.clone(), 1024 * 1024)), timer)
}

pub fn create_dash_map_server() -> MockServer {
    let timer = Arc::new(MockSystemTimer::new());
    MockServer::new(Arc::new(DashMapMemoryStore::new(timer.clone())), timer)
}

pub fn create_dash_map_storage() -> Arc<MemcStore> {
    let timer = Arc::new(MockSystemTimer::new());
    Arc::new(MemcStore::new(Arc::new(DashMapMemoryStore::new(timer))))
}

pub fn create_moka_storage() -> Arc<MemcStore> {
    let timer = Arc::new(MockSystemTimer::new());
    Arc::new(MemcStore::new(Arc::new(DashMapMemoryStore::new(timer))))
}
