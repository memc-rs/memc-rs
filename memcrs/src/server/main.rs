use crate::memcache;
use crate::memcache_server;
use crate::server::timer;
use log::info;
use std::process;
use std::sync::Arc;
use tracing_log::LogTracer;
extern crate clap;

#[cfg(feature = "jemallocator")]
use jemallocator::Jemalloc;

#[cfg(feature = "jemallocator")]
#[global_allocator]
static GLOBAL: Jemalloc = Jemalloc;

fn get_log_level(verbose: u8) -> tracing::Level {
    // Vary the output based on how many times the user used the "verbose" flag
    // // (i.e. 'myprog -v -v -v' or 'myprog -vvv' vs 'myprog -v'
    match verbose {
        0 => tracing::Level::ERROR,
        1 => tracing::Level::WARN,
        2 => tracing::Level::INFO,
        3 => tracing::Level::DEBUG,
        _ => tracing::Level::TRACE,
    }
}

pub fn run(args: Vec<String>) {
    LogTracer::init().expect("Cannot initialize logger");

    let cli_config = match memcache::cli::parser::parse(args) {
        Ok(config) => config,
        Err(err) => {
            eprint!("{}", err);
            process::exit(1);
        }
    };
    // Vary the output based on how many times the user used the "verbose" flag
    // (i.e. 'myprog -v -v -v' or 'myprog -vvv' vs 'myprog -v'
    tracing_subscriber::fmt()
        .with_max_level(get_log_level(cli_config.verbose))
        .init();

    info!("Listen address: {}", cli_config.listen_address);
    info!("Listen port: {}", cli_config.port);
    info!("Connection limit: {}", cli_config.connection_limit);
    info!("Number of threads: {}", cli_config.threads);
    info!("Store engine: {}", cli_config.store_engine.as_str());
    info!("Eviction policy: {}", cli_config.eviction_policy.as_str());
    info!("Runtime type: {}", cli_config.runtime_type.as_str());
    info!(
        "Max item size: {}",
        byte_unit::Byte::from_u64(cli_config.item_size_limit)
            .get_appropriate_unit(byte_unit::UnitType::Decimal)
    );
    info!(
        "Memory limit: {}",
        byte_unit::Byte::from_u64(cli_config.memory_limit)
            .get_appropriate_unit(byte_unit::UnitType::Decimal)
    );

    let system_timer: Arc<timer::SystemTimer> = Arc::new(timer::SystemTimer::new());
    let parent_runtime =
        memcache_server::runtime_builder::create_memcrs_server(cli_config, system_timer.clone());
    parent_runtime.block_on(system_timer.run())
}
