use std::process;

use memcrs::{
    memcache,
    memcache_server::{
        runtime_builder::start_memcrs_server_with_ctxt, server_context::ServerContext,
    },
};
use nix::errno::Errno;
use tokio_util::sync::CancellationToken;

use crate::common::{random_port::pseudoRanomPort, MemcrsdServerParamsBuilder};

pub struct MemcrsdMultiThreadTestServer {
    thread_join_handle: Option<std::thread::JoinHandle<()>>,
    cancellation_token: CancellationToken,
    port: u16,
}

impl MemcrsdMultiThreadTestServer {
    fn new(
        thread_join_handle: std::thread::JoinHandle<()>,
        cancellation_token: CancellationToken,
        port: u16,
    ) -> MemcrsdMultiThreadTestServer {
        MemcrsdMultiThreadTestServer {
            thread_join_handle: Some(thread_join_handle),
            cancellation_token,
            port,
        }
    }

    fn kill(&mut self) -> Result<(), Errno> {
        self.cancellation_token.cancel();
        if let Some(thread_join_handle) = self.thread_join_handle.take() {
            thread_join_handle.join().unwrap();
        }
        Ok(())
    }

    pub fn get_connection_string(&self) -> String {
        String::from(format!(
            "memcache://127.0.0.1:{}?timeout=5&tcp_nodelay=true&protocol=binary",
            self.port
        ))
    }
}

impl Drop for MemcrsdMultiThreadTestServer {
    fn drop(&mut self) {
        match self.kill() {
            Ok(_) => {}
            Err(err) => {
                eprintln!("Problem when killing process: {err}");
            }
        }
    }
}

fn spawn_server_args(args: Vec<String>) -> MemcrsdMultiThreadTestServer {
    let config = match memcache::cli::parser::parse(args) {
        Ok(config) => config,
        Err(err) => {
            eprint!("{}", err);
            process::exit(1);
        }
    };
    let store_config = memcache::builder::MemcacheStoreConfig::new(
        config.store_engine,
        config.memory_limit,
        config.eviction_policy,
    );
    let ctxt = ServerContext::get_default_server_context(store_config);
    let cancellation_token = ctxt.cancellation_token();
    let port = config.port;
    let handle = std::thread::spawn(move || start_memcrs_server_with_ctxt(config, ctxt));
    MemcrsdMultiThreadTestServer::new(handle, cancellation_token, port)
}

pub fn spawn_server(mut params: MemcrsdServerParamsBuilder) -> MemcrsdMultiThreadTestServer {
    let port = pseudoRanomPort.lock().unwrap().get_next_port();
    params.with_port(port);
    let args = params.build();
    spawn_server_args(args)
}
