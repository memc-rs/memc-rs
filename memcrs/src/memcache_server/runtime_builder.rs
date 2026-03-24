extern crate core_affinity;
use crate::memcache;
use crate::memcache::builder::EngineStoreConfig;
use crate::memcache::cli::parser::RuntimeType;
use crate::memcache_server::server_context::ServerContext;
use crate::memcache_server::threadpool_server_builder::ThreadpoolServerBuilder;
use crate::memcache_server::{self, register_cancellation, server_thread};
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::runtime::Builder;

use crate::memcache::cli::parser::MemcrsdConfig;

fn create_current_thread_runtime() -> tokio::runtime::Runtime {
    let runtime = Builder::new_current_thread()
        //.worker_threads(threads as usize)
        .thread_name_fn(server_thread::get_worker_thread_name)
        //.max_blocking_threads(2)
        .enable_all()
        .build()
        .unwrap();
    runtime
}

fn create_current_thread_server(config: MemcrsdConfig, ctxt: ServerContext) {
    let cancellation_token = ctxt.cancellation_token();
    let system_timer = ctxt.system_timer();
    let store = ctxt.store();
    let task_runner = ctxt.pending_tasks_runner();

    let addr = SocketAddr::new(config.listen_address, config.port);

    let memc_config = memcache_server::memc_tcp::MemcacheServerConfig::new(
        60,
        config.connection_limit,
        config.item_size_limit as u32,
        config.backlog_limit,
    );
    let listener_factory = memcache_server::listener_factory::ListenerFactory::new(memc_config);

    let core_ids = core_affinity::get_core_ids().unwrap();

    for i in 0..config.threads {
        let store_rc = Arc::clone(&store);
        let core_ids_clone = core_ids.clone();
        let cancellation_token = cancellation_token.clone();
        std::thread::spawn(move || {
            debug!("Creating runtime {}", i);
            let core_id = core_ids_clone[i % core_ids_clone.len()];
            let mut res = false;
            if !config.cpu_no_pin {
                res = core_affinity::set_for_current(core_id);
            }
            let create_runtime = || {
                let child_runtime = create_current_thread_runtime();
                let mut tcp_server = memcache_server::memc_tcp::MemcacheTcpServer::new(
                    memc_config,
                    store_rc,
                    cancellation_token.clone(),
                );
                let listener = listener_factory.get_tcp_listener(addr).unwrap_or_else(|e| {
                    log::error!("Failed to create TCP listener: {}; addr: {}", e, addr);
                    std::process::exit(1);
                });
                child_runtime.block_on(tcp_server.run(listener)).unwrap()
            };
            if res {
                debug!(
                    "Thread pinned {:?} to core {:?}",
                    std::thread::current().id(),
                    core_id.id
                );
                create_runtime();
            } else {
                if config.cpu_no_pin {
                    info!(
                        "Threads not pinned to core as per user request, core {}",
                        core_id.id
                    );
                } else {
                    warn!("Cannot pin thread to core {}", core_id.id);
                }

                create_runtime();
            }
        });
    }
    let mut runtime = create_current_thread_runtime();
    register_cancellation::register_ctrlc_handler(&mut runtime, cancellation_token);
    runtime.spawn(async move { task_runner.run().await });
    runtime.block_on(system_timer.run())
}

fn create_threadpool_server(config: MemcrsdConfig, ctxt: ServerContext) {
    let system_timer = ctxt.system_timer();
    let runtime_builder = ThreadpoolServerBuilder::new(config, ctxt.clone());
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
