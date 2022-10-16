use byte_unit::{Byte, ByteUnit};
use log::{debug, info};
use std::net::{IpAddr, SocketAddr};
use std::process;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use tokio::runtime::Builder;

extern crate clap;
extern crate memcrs;
use clap::{command, Arg};

#[cfg(feature = "jemallocator")]
use jemallocator::Jemalloc;

#[cfg(feature = "jemallocator")]
#[global_allocator]
static GLOBAL: Jemalloc = Jemalloc;

fn main() {
    let _cpus = (num_cpus::get_physical() + 1).to_string();
    let runtimes = (num_cpus::get_physical()).to_string();

    let app = command!();
    let matches = app
        .version(memcrs::version::MEMCRS_VERSION)
        .author("Dariusz Ostolski <memc-rs@memc.rs>")
        .about("memcrsd - memcached compatible server implementation in Rust")
        .arg(
            Arg::new("port")
                .short('p')
                .long("port")
                .default_value("11211")
                .help("TCP port to listen on")
                .takes_value(true),
        )
        .arg(
            Arg::new("listen")
                .short('l')
                .long("listen")
                .default_value("127.0.0.1")
                .help("interface to listen on")
                .takes_value(true),
        )
        .arg(
            Arg::new("conn-limit")
                .short('c')
                .long("conn-limit")
                .default_value("1024")
                .help("max simultaneous connections")
                .takes_value(true),
        )
        .arg(
            Arg::new("listen-backlog")
                .short('b')
                .long("listen-backlog")
                .default_value("1024")
                .help("set the backlog queue limit")
                .takes_value(true),
        )
        .arg(
            Arg::new("v")
                .short('v')
                .multiple_occurrences(true)
                .help("Sets the level of verbosity"),
        )
        .arg(
            Arg::new("memory-limit")
                .short('m')
                .long("memory-limit")
                .default_value("64")
                .help("item memory in megabytes")
                .takes_value(true),
        )
        .arg(
            Arg::new("max-item-size")
                .short('I')
                .long("max-item-size")
                .default_value("1m")
                .help("adjusts max item size (min: 1k, max: 1024m)")
                .takes_value(true),
        )
        .arg(
            Arg::new("runtimes")
                .short('r')
                .long("runtimes")
                .default_value(&runtimes)
                .help("number of runtimes to use, each runtime will have n number of threads")
                .takes_value(true),
        )
        .get_matches_mut();

    let port: u16 = matches.value_of_t("port").unwrap_or_else(|e| e.exit());
    let connection_limit: u32 = matches
        .value_of_t("conn-limit")
        .unwrap_or_else(|e| e.exit());

    let backlog_limit: u32 = matches
        .value_of_t("listen-backlog")
        .unwrap_or_else(|e| e.exit());

    let memory_limit_mb: u64 = matches
        .value_of_t("memory-limit")
        .unwrap_or_else(|e| e.exit());
    let memory_limit_res = Byte::from_unit(memory_limit_mb as f64, ByteUnit::MiB).unwrap();
    let memory_limit: u64 = memory_limit_res.get_bytes() as u64;

    let item_size_limit_str: String = matches
        .value_of_t("max-item-size")
        .unwrap_or_else(|e| e.exit());
    let item_size_limit_res = Byte::from_str(item_size_limit_str).unwrap();
    let item_size_limit_max = Byte::from_unit(1000f64, ByteUnit::MiB).unwrap();

    if item_size_limit_res.get_bytes() > item_size_limit_max.get_bytes() {
        eprintln!(
            "Max item size cannot be greater than: {}",
            item_size_limit_max.get_appropriate_unit(false)
        );
        process::exit(1);
    }

    let runtimes: u32 = matches.value_of_t("runtimes").unwrap_or_else(|e| e.exit());

    let listen_address = matches
        .value_of("listen")
        .unwrap()
        .parse::<IpAddr>()
        .unwrap_or_else(|e| {
            let mut cmd = command!();
            cmd.error(clap::ErrorKind::InvalidValue, e.to_string())
                .exit();
        });

    // Vary the output based on how many times the user used the "verbose" flag
    // (i.e. 'myprog -v -v -v' or 'myprog -vvv' vs 'myprog -v'
    match matches.occurrences_of("v") {
        0 => {
            tracing_subscriber::fmt()
                .with_max_level(tracing::Level::ERROR)
                .init();
        }
        1 => {
            tracing_subscriber::fmt()
                .with_max_level(tracing::Level::WARN)
                .init();
        }
        2 => {
            tracing_subscriber::fmt()
                .with_max_level(tracing::Level::INFO)
                .init();
        }
        3 => {
            tracing_subscriber::fmt()
                .with_max_level(tracing::Level::DEBUG)
                .init();
        }
        _ => {
            tracing_subscriber::fmt()
                .with_max_level(tracing::Level::TRACE)
                .init();
        }
    }

    info!("Listen address: {}", matches.value_of("listen").unwrap());
    info!("Listen port: {}", port);
    info!("Connection limit: {}", connection_limit);
    info!("Number of runtimes: {}", runtimes);
    info!("Number of threads total: {}", (runtimes) + 1);
    info!("Max item size: {}", item_size_limit_res.get_bytes());
    info!("Memory limit: {} MB", memory_limit_mb);

    let config = memcrs::server::memc_tcp::MemcacheServerConfig::new(
        60,
        connection_limit,
        item_size_limit_res.get_bytes() as u32,
        backlog_limit,
    );
    let store_config = memcrs::memcache::builder::MemcacheStoreConfig::new(memory_limit);

    let system_timer: Arc<memcrs::storage::timer::SystemTimer> =
        Arc::new(memcrs::storage::timer::SystemTimer::new());
    let memcache_store = memcrs::memcache::builder::MemcacheStoreBuilder::from_config(
        store_config,
        system_timer.clone(),
    );

    let addr = SocketAddr::new(listen_address, port);
    for i in 0..runtimes {
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
