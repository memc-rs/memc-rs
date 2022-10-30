use std::sync::{atomic::{AtomicUsize, Ordering}, Arc};
use tokio::runtime::Builder;
use std::net::{SocketAddr};
use crate::{storage::{self, store::KVStore}, memcache::cli::parser::RuntimeType};
use crate::memcache;
use crate::server;

use crate::memcache::cli::parser::MemcrsArgs;

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

fn create_current_thread_server(config: MemcrsArgs, store: Arc<dyn KVStore + Send+ Sync>) -> tokio::runtime::Runtime {
  let addr = SocketAddr::new(config.listen_address, config.port);
  let memc_config = server::memc_tcp::MemcacheServerConfig::new(
    60,
    config.connection_limit,
    config.item_size_limit.get_bytes() as u32,
    config.backlog_limit,
  );
  for i in 0..config.threads {
      let store_rc = Arc::clone(&store);
      std::thread::spawn(move || {
          debug!("Creating runtime {}", i);
          let child_runtime = create_current_thread_runtime();
          let mut tcp_server = server::memc_tcp::MemcacheTcpServer::new(memc_config, store_rc);
          child_runtime.block_on(tcp_server.run(addr)).unwrap()
      });
  }
  create_current_thread_runtime()
}

pub fn create_memcrs_server(config: MemcrsArgs, system_timer: std::sync::Arc<storage::timer::SystemTimer>) -> tokio::runtime::Runtime {
  let store_config = memcache::builder::MemcacheStoreConfig::new(config.memory_limit);
  let memcache_store = memcache::builder::MemcacheStoreBuilder::from_config(
    store_config,
    system_timer.clone(),
  );
  
  match (config.runtime_type) {
    RuntimeType::CurrentThread => create_current_thread_server(config, memcache_store),
    RuntimeType::MultiThread => create_current_thread_runtime()
  }

}