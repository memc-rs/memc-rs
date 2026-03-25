use crate::{memcache::cli::parser::MemcrsdConfig, memcache_server::server_context::ServerContext};
extern crate core_affinity;
use crate::memcache_server::{self, register_cancellation, server_thread};
use core_affinity::CoreId;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::runtime::Builder;

pub struct CurrentThreadRuntimeBuilder {
    config: MemcrsdConfig,
    ctxt: ServerContext,
}

impl CurrentThreadRuntimeBuilder {
    pub fn new(config: MemcrsdConfig, ctxt: ServerContext) -> CurrentThreadRuntimeBuilder {
        CurrentThreadRuntimeBuilder { config, ctxt }
    }

    pub fn build(&self) -> tokio::runtime::Runtime {
        let cancellation_token = self.ctxt.cancellation_token();
        let task_runner = self.ctxt.pending_tasks_runner();
        let core_ids = core_affinity::get_core_ids().unwrap();

        for i in 0..self.config.threads {
            self.spawn_worker_runtime(core_ids.clone(), i);
        }
        let mut control_runtime = create_current_thread_runtime();
        register_cancellation::register_ctrlc_handler(&mut control_runtime, cancellation_token);
        control_runtime.spawn(async move { task_runner.run().await });
        control_runtime
    }

    fn spawn_worker_runtime(&self, core_ids_clone: Vec<CoreId>, i: usize) {
        let cancellation_token = self.ctxt.cancellation_token().clone();
        let store_rc = Arc::clone(&self.ctxt.store());
        let memc_config = memcache_server::memc_tcp::MemcacheServerConfig::new(
            60,
            self.config.connection_limit,
            self.config.item_size_limit as u32,
            self.config.backlog_limit,
        );
        let addr = SocketAddr::new(self.config.listen_address, self.config.port);
        let listener_factory = memcache_server::listener_factory::ListenerFactory::new(memc_config);
        let cpu_no_pin = self.config.cpu_no_pin;
        let core_id = core_ids_clone[i % core_ids_clone.len()];
        std::thread::spawn(move || {
            debug!("Creating runtime {}", i);

            pin_current_thread_to_core(cpu_no_pin, core_id);

            let worker_runtime = create_current_thread_runtime();
            let mut tcp_server = memcache_server::memc_tcp::MemcacheTcpServer::new(
                memc_config,
                store_rc,
                cancellation_token.clone(),
            );
            let listener = listener_factory.get_tcp_listener(addr).unwrap_or_else(|e| {
                log::error!("Failed to create TCP listener: {}; addr: {}", e, addr);
                std::process::exit(1);
            });
            worker_runtime.block_on(tcp_server.run(listener)).unwrap()
        });
    }
}

fn pin_current_thread_to_core(cpu_no_pin: bool, core_id: CoreId) {
    if !cpu_no_pin {
        let res = core_affinity::set_for_current(core_id);
        if res {
            debug!(
                "Thread pinned {:?} to core {:?}",
                std::thread::current().id(),
                core_id.id
            );
        } else {
            warn!("Cannot pin thread to core {}", core_id.id);
        }
    } else {
        info!(
            "Threads not pinned to core as per user request, core {}",
            core_id.id
        );
    }
}

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
