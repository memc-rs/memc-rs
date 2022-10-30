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
    info!("Number of threads: {}", cli_config.threads);
    info!("Runtime type: {}", cli_config.runtime_type.as_str());
    info!("Max item size: {}", cli_config.item_size_limit.get_bytes());
    info!("Memory limit: {} MB", cli_config.memory_limit_mb);
    let system_timer: Arc<memcrs::storage::timer::SystemTimer> = Arc::new(memcrs::storage::timer::SystemTimer::new());

    let parent_runtime = memcrs::server::runtime_builder::create_memcrs_server(cli_config, system_timer.clone());
    parent_runtime.block_on(system_timer.run())
}

