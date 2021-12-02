use byte_unit::{Byte, ByteUnit};
use log::{debug, info};
use num_cpus;
use std::net::{IpAddr, SocketAddr};
use std::process;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use tokio::runtime::Builder;
use tracing_subscriber;

extern crate clap;
extern crate memcrs;
use clap::{value_t, App, Arg};

#[cfg(feature = "jemallocator")]
use jemallocator::Jemalloc;

#[cfg(feature = "jemallocator")]
#[global_allocator]
static GLOBAL: Jemalloc = Jemalloc;

fn main() {
    let cpus = (num_cpus::get_physical() + 1).to_string();
    let runtimes = ((num_cpus::get_physical())).to_string();

    let app = App::new("memcrsd");
    let matches = app
        .version(memcrs::version::MEMCRS_VERSION)
        .author("Dariusz Ostolski <dariusz.ostolski@gmail.com>")
        .about("memcrsd - memcached compatible server implementation in Rust")
        .arg(
            Arg::with_name("port")
                .short("p")
                .long("port")
                .default_value("11211")
                .help("TCP port to listen on")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("listen")
                .short("l")
                .long("listen")
                .default_value("127.0.0.1")
                .help("interface to listen on")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("conn-limit")
                .short("c")
                .long("conn-limit")
                .default_value("1024")
                .help("max simultaneous connections")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("listen-backlog")
                .short("b")
                .long("listen-backlog")
                .default_value("1024")
                .help("set the backlog queue limit")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("v")
                .short("v")
                .multiple(true)
                .help("Sets the level of verbosity"),
        )
        .arg(
            Arg::with_name("memory-limit")
                .short("m")
                .long("memory-limit")
                .default_value("64")
                .help("item memory in megabytes")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("threads")
                .short("t")
                .long("threads")
                .default_value(&cpus)
                .help("number of threads to use")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("max-item-size")
                .short("I")
                .long("max-item-size")
                .default_value("1m")
                .help("adjusts max item size (min: 1k, max: 1024m)")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("runtimes")
                .short("r")
                .long("runtimes")
                .default_value(&runtimes)
                .help("number of runtimes to use, each runtime will have n number of threads")
                .takes_value(true),
        )
        .get_matches();

    let port: u16 = value_t!(matches.value_of("port"), u16).unwrap_or_else(|e| e.exit());
    let connection_limit: u32 =
        value_t!(matches.value_of("conn-limit"), u32).unwrap_or_else(|e| e.exit());

    let backlog_limit: u32 =
        value_t!(matches.value_of("listen-backlog"), u32).unwrap_or_else(|e| e.exit());

    let memory_limit_mb =
        value_t!(matches.value_of("memory-limit"), u64).unwrap_or_else(|e| e.exit());
    let memory_limit_res = Byte::from_unit(memory_limit_mb as f64, ByteUnit::MiB).unwrap();
    let memory_limit: u64 = memory_limit_res.get_bytes() as u64;

    let item_size_limit_str =
        value_t!(matches.value_of("max-item-size"), String).unwrap_or_else(|e| e.exit());
    let item_size_limit_res = Byte::from_str(item_size_limit_str).unwrap();
    let item_size_limit_max = Byte::from_unit(1000f64, ByteUnit::MiB).unwrap();

    if item_size_limit_res.get_bytes() > item_size_limit_max.get_bytes() {
        eprintln!(
            "Max item size cannot be greater than: {}",
            item_size_limit_max.get_appropriate_unit(false).to_string()
        );
        process::exit(1);
    }

    let threads: u32 = value_t!(matches.value_of("threads"), u32).unwrap_or_else(|e| e.exit());
    let runtimes: u32 = value_t!(matches.value_of("runtimes"), u32).unwrap_or_else(|e| e.exit());

    let listen_address = matches
        .value_of("listen")
        .unwrap()
        .parse::<IpAddr>()
        .unwrap_or_else(|e| {
            let clap_error = clap::Error {
                message: e.to_string(),
                kind: clap::ErrorKind::InvalidValue,
                info: None,
            };
            clap_error.exit()
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
    info!("Number of threads per runtime: {}", threads);
    info!("Number of runtimes: {}", runtimes);
    info!(
        "Number of threads total: {}",
        (runtimes)  + 1
    );
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
            let child_runtime = create_runtime(threads);
            let mut tcp_server = memcrs::server::memc_tcp::MemcacheTcpServer::new(config, store);
            child_runtime.block_on(tcp_server.run(addr)).unwrap()
        });
    }
    let parent_runtime = create_runtime(1);
    parent_runtime.block_on(system_timer.run())
}

fn create_runtime(_threads: u32) -> tokio::runtime::Runtime {
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
