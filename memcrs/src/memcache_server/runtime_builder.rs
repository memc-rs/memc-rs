extern crate core_affinity;
use crate::memcache;
use crate::memcache::builder::EngineStoreConfig;
use crate::memcache::cli::parser::RuntimeType;
use crate::memcache_server::current_thread_runtime_builder::CurrentThreadRuntimeBuilder;
use crate::memcache_server::server_context::ServerContext;
use crate::memcache_server::threadpool_runtime_builder::ThreadpoolRuntimeBuilder;

use crate::memcache::cli::parser::MemcrsdConfig;

fn create_current_thread_server(config: MemcrsdConfig, ctxt: ServerContext) {
    let system_timer = ctxt.system_timer();
    let runtime_builder = CurrentThreadRuntimeBuilder::new(config, ctxt.clone());
    let runtime = runtime_builder.build();
    runtime.block_on(system_timer.run())
}

fn create_threadpool_server(config: MemcrsdConfig, ctxt: ServerContext) {
    let system_timer = ctxt.system_timer();
    let runtime_builder = ThreadpoolRuntimeBuilder::new(config, ctxt.clone());
    let runtime = runtime_builder.build();
    runtime.block_on(system_timer.run())
}

pub fn start_memcrs_server(config: MemcrsdConfig) {
    let engine_store_config = match config.store_engine {
        crate::memory_store::StoreEngine::DashMap => {
            EngineStoreConfig::DashMap(config.dash_map.clone().unwrap())
        }
        crate::memory_store::StoreEngine::Moka => {
            EngineStoreConfig::Moka(config.moka.clone().unwrap())
        }
    };

    let store_config =
        memcache::builder::MemcacheStoreConfig::new(config.store_engine, engine_store_config);
    let ctxt = ServerContext::get_default_server_context(store_config);
    start_memcrs_server_with_ctxt(config, ctxt)
}

pub fn start_memcrs_server_with_ctxt(config: MemcrsdConfig, ctxt: ServerContext) {
    match config.runtime_type {
        RuntimeType::CurrentThread => create_current_thread_server(config, ctxt),
        RuntimeType::MultiThread => create_threadpool_server(config, ctxt),
    }
}
