use log::{debug, info};
use std::env;
use std::net::{SocketAddr};
use std::process;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use tokio::runtime::Builder;

extern crate clap;
extern crate memcrs;

#[cfg(feature = "jemallocator")]
use jemallocator::Jemalloc;

#[cfg(feature = "jemallocator")]
#[global_allocator]
static GLOBAL: Jemalloc = Jemalloc;

fn main() {
    let _cpus = (num_cpus::get_physical() + 1).to_string();
    let runtimes = (num_cpus::get_physical()).to_string();

    let cli_config = match memcrs::memcache::cli::parser::parse(runtimes, env::args().collect()) {
        Ok(config) => config,
        Err(err) => {
            eprint!("{}", err);
            process::exit(1);
        }
    };

    // Vary the output based on how many times the user used the "verbose" flag
    // (i.e. 'myprog -v -v -v' or 'myprog -vvv' vs 'myprog -v'
    tracing_subscriber::fmt()
        .with_max_level(cli_config.log_level)
        .init();

    info!("Listen address: {}", cli_config.listen_address.to_string());
    info!("Listen port: {}", cli_config.port);
    info!("Connection limit: {}", cli_config.connection_limit);
    info!("Number of runtimes: {}", cli_config.runtimes);
    info!("Number of threads total: {}", (cli_config.runtimes) + 1);
    info!("Max item size: {}", cli_config.item_size_limit.get_bytes());
    info!("Memory limit: {} MB", cli_config.memory_limit_mb);

    let config = memcrs::server::memc_tcp::MemcacheServerConfig::new(
        60,
        cli_config.connection_limit,
        cli_config.item_size_limit.get_bytes() as u32,
        cli_config.backlog_limit,
    );
    let store_config = memcrs::memcache::builder::MemcacheStoreConfig::new(cli_config.memory_limit);

    let system_timer: Arc<memcrs::storage::timer::SystemTimer> =
        Arc::new(memcrs::storage::timer::SystemTimer::new());
    let memcache_store = memcrs::memcache::builder::MemcacheStoreBuilder::from_config(
        store_config,
        system_timer.clone(),
    );

    let addr = SocketAddr::new(cli_config.listen_address, cli_config.port);
    for i in 0..cli_config.runtimes {
        let store = Arc::clone(&memcache_store);
        std::thread::spawn(move || {
            debug!("Creating runtime {}", i);
            let child_runtime = create_runtime();
            let mut tcp_server = memcrs::server::memc_tcp::MemcacheTcpServer::new(config, store);
            child_runtime.block_on(tcp_server.run(addr)).unwrap()
        });
    }
    let parent_runtime = create_runtime();
    parent_runtime.block_on(system_timer.run())
}

fn create_runtime() -> tokio::runtime::Runtime {
    let runtime = Builder::new_current_thread()
        //.worker_threads(threads as usize)
        .thread_name_fn(|| {
            static ATOMIC_ID: AtomicUsize = AtomicUsize::new(0);
            let id = ATOMIC_ID.fetch_add(1, Ordering::SeqCst);
            let str = format!("memcrsd-wrk-{}", id);
            str
        })
        //.max_blocking_threads(2)
        .enable_all()
        .build()
        .unwrap();
    runtime
}
