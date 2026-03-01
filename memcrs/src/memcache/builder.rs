use crate::cache::cache::Cache;
use crate::memcache::cli::parser::{DashMapConfig, MokaConfig};
use crate::memory_store::dash_map_store::DashMapMemoryStore as DashMapStore;
use crate::memory_store::moka_store::MokaMemoryStore as MokaStore;
use crate::memory_store::StoreEngine;
use crate::server::timer;
use std::sync::Arc;

#[derive(Clone)]
pub enum EngineStoreConfig {
    Moka(MokaConfig),
    DashMap(DashMapConfig),
}

#[allow(dead_code)]
pub struct MemcacheStoreConfig {
    engine: StoreEngine,
    config: EngineStoreConfig,
}

impl MemcacheStoreConfig {
    pub fn new(engine: StoreEngine, config: EngineStoreConfig) -> MemcacheStoreConfig {
        MemcacheStoreConfig { engine, config }
    }

    pub fn engine(&self) -> StoreEngine {
        self.engine
    }

    pub fn config(&self) -> EngineStoreConfig {
        self.config.clone()
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
        let mut dashmap_config: Option<DashMapConfig> = None;
        let mut moka_config: Option<MokaConfig> = None;
        match config.config {
            EngineStoreConfig::DashMap(cfg) => {
                dashmap_config = Some(cfg);
            }
            EngineStoreConfig::Moka(cfg) => {
                moka_config = Some(cfg);
            }
        }
        let store: Arc<dyn Cache + Send + Sync> = match config.engine {
            StoreEngine::DashMap => Arc::new(DashMapStore::new(timer, dashmap_config.unwrap())),
            StoreEngine::Moka => Arc::new(MokaStore::new(timer, moka_config.unwrap())),
        };
        store
    }
}
