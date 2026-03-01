use crate::cache::cache::Cache;
use crate::memcache::cli::parser::{DashMapConfig, MokaConfig};
use crate::memcache::store::MemcStore;
use crate::memory_store::dash_map_store::DashMapMemoryStore;
use crate::memory_store::moka_store::MokaMemoryStore as MokaStore;
use crate::server::timer;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;

pub struct MockSystemTimer {
    pub current_time: AtomicU32,
}

pub trait SetableTimer: timer::Timer {
    fn set(&self, time: u32);
    fn add_seconds(&self, seconds: u32);
}

impl MockSystemTimer {
    pub fn new() -> Self {
        MockSystemTimer {
            current_time: AtomicU32::new(0),
        }
    }
}

impl Default for MockSystemTimer {
    fn default() -> Self {
        Self::new()
    }
}

impl timer::Timer for MockSystemTimer {
    fn timestamp(&self) -> u32 {
        self.current_time.load(Ordering::Relaxed)
    }
}

impl SetableTimer for MockSystemTimer {
    fn set(&self, time: u32) {
        self.current_time.store(time, Ordering::Relaxed)
    }

    fn add_seconds(&self, seconds: u32) {
        self.current_time.fetch_add(seconds, Ordering::Release);
    }
}

pub struct MockServer {
    pub timer: Arc<MockSystemTimer>,
    pub storage: MemcStore,
}

impl MockServer {
    pub fn new(store: Arc<dyn Cache + Send + Sync>, timer: Arc<MockSystemTimer>) -> Self {
        MockServer {
            timer,
            storage: MemcStore::new(store),
        }
    }
}

pub fn create_moka_server() -> MockServer {
    let config = MokaConfig::default();
    let timer = Arc::new(MockSystemTimer::new());
    MockServer::new(Arc::new(MokaStore::new(timer.clone(), config)), timer)
}

pub fn create_dash_map_server() -> MockServer {
    let timer = Arc::new(MockSystemTimer::new());
    let config = DashMapConfig::default();
    MockServer::new(
        Arc::new(DashMapMemoryStore::new(timer.clone(), config)),
        timer,
    )
}

pub struct StoreWithMockTimer {
    pub timer: Arc<MockSystemTimer>,
    pub memc_store: Arc<MemcStore>,
}

pub fn create_dash_map_storage() -> StoreWithMockTimer {
    let timer = Arc::new(MockSystemTimer::new());
    let config = DashMapConfig::default();
    let memc_store = Arc::new(MemcStore::new(Arc::new(DashMapMemoryStore::new(
        timer.clone(),
        config,
    ))));
    StoreWithMockTimer { timer, memc_store }
}

pub fn create_moka_storage() -> StoreWithMockTimer {
    let config = MokaConfig::default();
    let timer = Arc::new(MockSystemTimer::new());
    let memc_store = Arc::new(MemcStore::new(Arc::new(MokaStore::new(
        timer.clone(),
        config,
    ))));
    StoreWithMockTimer { timer, memc_store }
}
