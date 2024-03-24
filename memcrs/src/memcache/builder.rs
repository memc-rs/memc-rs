use crate::cache::eviction_policy::EvictionPolicy;
use crate::cache::cache::Cache;
use crate::memory_store::store::{MemoryStore as DashMapStore};
use crate::memory_store::moka_store::{MemoryStore as MokaStore};
use crate::memory_store::StoreEngine;
use crate::server::timer;
use std::sync::Arc;
#[allow(dead_code)]
pub struct MemcacheStoreConfig {
    engine: StoreEngine,
    policy: EvictionPolicy,
    memory_limit: u64,
}

impl MemcacheStoreConfig {
    pub fn new(engine: StoreEngine, memory_limit: u64, policy: EvictionPolicy) -> MemcacheStoreConfig {
        MemcacheStoreConfig {
            engine,
            policy,
            memory_limit,
        }
    }
}

#[derive(Default)]
pub struct MemcacheStoreBuilder {}

impl MemcacheStoreBuilder {
    pub fn new() -> MemcacheStoreBuilder {
        MemcacheStoreBuilder {}
    }

    pub fn from_config(
        config: MemcacheStoreConfig,
        timer: Arc<dyn timer::Timer + Send + Sync>,
    ) -> Arc<dyn Cache + Send + Sync> {

        let store: Arc<dyn Cache + Send + Sync> = match config.engine {
            StoreEngine::DashMap => {
                Arc::new(DashMapStore::new(timer))
            },
            StoreEngine::Moka => {
                Arc::new(MokaStore::new(timer, config.memory_limit))
            }
        };
        store
    }
}
