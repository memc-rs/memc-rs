use crate::{memcache::cli::parser::MemcrsdConfig, memcache_server::server_context::ServerContext};
extern crate core_affinity;
use crate::memcache_server::{self, register_cancellation, server_thread};
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::runtime::Builder;

pub struct ThreadpoolRuntimeBuilder {
    config: MemcrsdConfig,
    ctxt: ServerContext,
}

impl ThreadpoolRuntimeBuilder {
    pub fn new(config: MemcrsdConfig, ctxt: ServerContext) -> ThreadpoolRuntimeBuilder {
        ThreadpoolRuntimeBuilder { config, ctxt }
    }

    pub fn build(&self) -> tokio::runtime::Runtime {
        let cancellation_token = self.ctxt.cancellation_token();
        let store = self.ctxt.store();
        let task_runner = self.ctxt.pending_tasks_runner();

        let memc_config = memcache_server::memc_tcp::MemcacheServerConfig::new(
            60,
            self.config.connection_limit,
            self.config.item_size_limit as u32,
            self.config.backlog_limit,
        );

        let addr = SocketAddr::new(self.config.listen_address, self.config.port);
        let listener_factory = memcache_server::listener_factory::ListenerFactory::new(memc_config);
        let listener = listener_factory.get_tcp_listener(addr).unwrap_or_else(|e| {
            log::error!("Failed to create TCP listener: {}; address {}", e, addr);
            std::process::exit(1);
        });

        let mut runtime = create_multi_thread_runtime(self.config.threads);
        let mut tcp_server = memcache_server::memc_tcp::MemcacheTcpServer::new(
            memc_config,
            Arc::clone(&store),
            cancellation_token.clone(),
        );

        runtime.spawn(async move { task_runner.run().await });
        runtime.spawn(async move { tcp_server.run(listener).await });
        register_cancellation::register_ctrlc_handler(&mut runtime, cancellation_token);
        runtime
    }
}

fn create_multi_thread_runtime(worker_threads: usize) -> tokio::runtime::Runtime {
    let runtime = Builder::new_multi_thread()
        .thread_name_fn(server_thread::get_worker_thread_name)
        .worker_threads(worker_threads)
        .enable_all()
        .build()
        .unwrap();
    runtime
}
