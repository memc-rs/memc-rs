use super::eviction_policy::EvictionPolicy;
use super::random_policy::RandomPolicy;
use crate::storage::store::{KVStore, KeyValueStore};
use crate::storage::timer;
use std::{mem, sync::Arc};

pub struct MemcacheStoreConfig {
    policy: EvictionPolicy,
    memory_limit: u64,
}

impl MemcacheStoreConfig {
    pub fn new(memory_limit: u64) -> MemcacheStoreConfig {
        MemcacheStoreConfig {
            policy: EvictionPolicy::None,
            memory_limit: memory_limit,
        }
    }
}

pub struct MemcacheStoreBuilder {}

impl MemcacheStoreBuilder {
    pub fn new() -> MemcacheStoreBuilder {
        MemcacheStoreBuilder {}
    }

    pub fn from_config(
        config: MemcacheStoreConfig,
        timer: Arc<dyn timer::Timer + Send + Sync>,
    ) -> Arc<dyn KVStore + Send + Sync> {
        let store: Arc<dyn KVStore + Send + Sync> = Arc::new(KeyValueStore::new(timer));
        Arc::new(RandomPolicy::new(store, config.memory_limit))
    }
}
