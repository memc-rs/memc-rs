use super::eviction_policy::EvictionPolicy;
use super::random_policy::RandomPolicy;
use crate::cache::cache::Cache;
use crate::memory_store::store::MemoryStore;
use crate::server::timer;
use std::sync::Arc;

pub struct MemcacheStoreConfig {
    policy: EvictionPolicy,
    memory_limit: u64,
}

impl MemcacheStoreConfig {
    pub fn new(memory_limit: u64) -> MemcacheStoreConfig {
        MemcacheStoreConfig {
            policy: EvictionPolicy::None,
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
        let store_engine = Arc::new(MemoryStore::new(timer));
        let store: Arc<dyn Cache + Send + Sync> = match config.policy {
            EvictionPolicy::Random => {
                Arc::new(RandomPolicy::new(store_engine, config.memory_limit))
            }
            EvictionPolicy::None => store_engine,
        };
        store
    }
}
