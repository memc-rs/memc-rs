extern crate core_affinity;
use crate::memcache;
use crate::memcache_server;
use crate::server;
use crate::{
    cache::cache::Cache, cache::pending_tasks_runner::PendingTasksRunner,
    memcache::cli::parser::RuntimeType,
};
use std::net::SocketAddr;
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};
use tokio::runtime::Builder;

use crate::memcache::cli::parser::MemcrsdConfig;

fn get_worker_thread_name() -> String {
    static ATOMIC_ID: AtomicUsize = AtomicUsize::new(0);
    let id = ATOMIC_ID.fetch_add(1, Ordering::SeqCst);
    let str = format!("memcrsd-wrk-{}", id);
    str
}

fn create_multi_thread_runtime(worker_threads: usize) -> tokio::runtime::Runtime {
    let runtime = Builder::new_multi_thread()
        .thread_name_fn(get_worker_thread_name)
        .worker_threads(worker_threads)
        .enable_all()
        .build()
        .unwrap();
    runtime
}

fn create_current_thread_runtime() -> tokio::runtime::Runtime {
    let runtime = Builder::new_current_thread()
        //.worker_threads(threads as usize)
        .thread_name_fn(get_worker_thread_name)
        //.max_blocking_threads(2)
        .enable_all()
        .build()
        .unwrap();
    runtime
}

fn create_current_thread_server(
    config: MemcrsdConfig,
    store: Arc<dyn Cache + Send + Sync>,
) -> tokio::runtime::Runtime {
    let addr = SocketAddr::new(config.listen_address, config.port);
    let memc_config = memcache_server::memc_tcp::MemcacheServerConfig::new(
        60,
        config.connection_limit,
        config.item_size_limit as u32,
        config.backlog_limit,
    );

    let core_ids = core_affinity::get_core_ids().unwrap();
    let task_runner = PendingTasksRunner::new(Arc::clone(&store));
    std::thread::spawn(move || {
        let child_runtime = create_current_thread_runtime();
        child_runtime.block_on(task_runner.run())
    });

    for i in 0..config.threads {
        let store_rc = Arc::clone(&store);
        let core_ids_clone = core_ids.clone();
        std::thread::spawn(move || {
            debug!("Creating runtime {}", i);
            let core_id = core_ids_clone[i % core_ids_clone.len()];
            let res = core_affinity::set_for_current(core_id);
            let create_runtime = || {
                let child_runtime = create_current_thread_runtime();
                let mut tcp_server =
                    memcache_server::memc_tcp::MemcacheTcpServer::new(memc_config, store_rc);
                child_runtime.block_on(tcp_server.run(addr)).unwrap()
            };
            if res {
                debug!(
                    "Thread pinned {:?} to core {:?}",
                    std::thread::current().id(),
                    core_id.id
                );
                create_runtime();
            } else {
                warn!("Cannot pin thread to core {}", core_id.id);
                create_runtime();
            }
        });
    }
    create_current_thread_runtime()
}

fn create_threadpool_server(
    config: MemcrsdConfig,
    store: Arc<dyn Cache + Send + Sync>,
) -> tokio::runtime::Runtime {
    let addr = SocketAddr::new(config.listen_address, config.port);
    let memc_config = memcache_server::memc_tcp::MemcacheServerConfig::new(
        60,
        config.connection_limit,
        config.item_size_limit as u32,
        config.backlog_limit,
    );
    let runtime = create_multi_thread_runtime(config.threads);
    let mut tcp_server =
        memcache_server::memc_tcp::MemcacheTcpServer::new(memc_config, Arc::clone(&store));
    let task_runner = PendingTasksRunner::new(Arc::clone(&store));
    runtime.spawn(async move { task_runner.run().await });
    runtime.spawn(async move { tcp_server.run(addr).await });
    runtime
}

pub fn create_memcrs_server(
    config: MemcrsdConfig,
    system_timer: std::sync::Arc<server::timer::SystemTimer>,
) -> tokio::runtime::Runtime {
    let store_config = memcache::builder::MemcacheStoreConfig::new(
        config.store_engine,
        config.memory_limit,
        config.eviction_policy,
    );
    let memcache_store =
        memcache::builder::MemcacheStoreBuilder::from_config(store_config, system_timer);

    match config.runtime_type {
        RuntimeType::CurrentThread => create_current_thread_server(config, memcache_store),
        RuntimeType::MultiThread => create_threadpool_server(config, memcache_store),
    }
}
