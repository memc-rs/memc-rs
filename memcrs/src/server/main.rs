use crate::memcache;
use crate::memcache::cli::parser::MemcrsdConfig;
use crate::memcache_server;
use log::info;
use std::process;
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
            eprintln!("{}", err);
            process::exit(1);
        }
    };

    // Vary the output based on how many times the user used the "verbose" flag
    // (i.e. 'myprog -v -v -v' or 'myprog -vvv' vs 'myprog -v'
    tracing_subscriber::fmt()
        .with_max_level(get_log_level(cli_config.verbose))
        .init();
    log_config(&cli_config);
    memcache_server::runtime_builder::start_memcrs_server(cli_config);
}


fn log_config(cli_config: &MemcrsdConfig) {
    info!("Listen address: {}", cli_config.listen_address);
    info!("Listen port: {}", cli_config.port);
    info!("Connection limit: {}", cli_config.connection_limit);
    info!("Number of threads: {}", cli_config.threads);
    info!("Store engine: {}", cli_config.store_engine.as_str());
    let dashmap_config = cli_config.dash_map.clone();
    let moka_config = cli_config.moka.clone();
    if let Some(cfg) = dashmap_config.clone() {
        info!(
        "Memory limit: {}",
        byte_unit::Byte::from_u64(cfg.memory_limit)
            .get_appropriate_unit(byte_unit::UnitType::Decimal)
        );
    }
    if let Some(cfg) = moka_config.clone() {
        info!("Eviction policy: {}", cfg.eviction_policy.as_str());
        info!("Maximum capacity: {}", cfg.max_capacity);
    }
    
    info!("Runtime type: {}", cli_config.runtime_type.as_str());
    info!(
        "Max item size: {}",
        byte_unit::Byte::from_u64(cli_config.item_size_limit)
            .get_appropriate_unit(byte_unit::UnitType::Decimal)
    );
    

    if cli_config.store_engine == crate::memory_store::StoreEngine::DashMap {
        warn!(
            "{} memory store does not yet support eviction of items.",
            cli_config.store_engine.as_str()
        );
    }
}