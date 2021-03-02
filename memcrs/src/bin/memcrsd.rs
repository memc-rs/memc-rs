use log::info;
use std::net::{IpAddr, SocketAddr};
use tracing_subscriber;
use tokio::runtime::Builder;
use num_cpus;
use std::sync::atomic::{AtomicUsize, Ordering};

extern crate clap;
extern crate memcrs;
use clap::{value_t, App, Arg};


fn main() {

    let cpus = (num_cpus::get_physical()+1).to_string();

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
        .get_matches();
    
    let port: u16 = value_t!(matches.value_of("port"), u16).unwrap_or_else(|e| e.exit());
    let connection_limit: u32 =
        value_t!(matches.value_of("conn-limit"), u32).unwrap_or_else(|e| e.exit());
    let memory_limit: u32 =
        value_t!(matches.value_of("memory-limit"), u32).unwrap_or_else(|e| e.exit());

    let threads: u32 =
        value_t!(matches.value_of("threads"), u32).unwrap_or_else(|e| e.exit());

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
    info!(
        "Connection limit: {}",
        matches.value_of("conn-limit").unwrap()
    );
    info!(
        "Number of threads: {}",
        matches.value_of("threads").unwrap()
    );
    let config = memcrs::server::memc_tcp::MemcacheServerConfig::new(60, connection_limit, memory_limit);
    let addr = SocketAddr::new(listen_address, port);
    let mut tcp_server = memcrs::server::memc_tcp::MemcacheTcpServer::new(config);

    let runtime = Builder::new_multi_thread()
            .worker_threads(threads as usize)
            .thread_name_fn(|| {
                static ATOMIC_ID: AtomicUsize = AtomicUsize::new(0);
                let id = ATOMIC_ID.fetch_add(1, Ordering::SeqCst);
                let str  = format!("memcrsd-wrk-{}", id);
                str
             })
            .max_blocking_threads(2)
            .enable_all()
            .build()
            .unwrap();

    runtime.block_on(tcp_server.run(addr)).unwrap();
}
