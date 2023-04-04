use super::eviction_policy::EvictionPolicy;
//use super::random_policy::RandomPolicy;
use super::store::{MemcStore, MemCacheStoreType};
use crate::storage::timer;
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
    ) -> Arc<dyn MemCacheStoreType + Send + Sync> {
        let store_engine = Arc::new(MemcStore::new(timer));
        let store: Arc<dyn MemCacheStoreType + Send + Sync> = match config.policy {
            EvictionPolicy::Random => {
                store_engine
            }
            EvictionPolicy::None => store_engine,
        };
        store
    }
}
