use log::info;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use tokio::io;
use tracing_subscriber;

extern crate clap;
extern crate memcrs;
use clap::{value_t, App, Arg, Error};

#[tokio::main]
async fn main() -> io::Result<()> {
    let app = App::new("memcrsd");
    let matches = app
        .version(memcrs::memcrs::version::MEMCRS_VERSION)
        .author("Dariusz Ostolski <dariusz.ostolski@gmail.com>")
        .about("Rust memcache compatible server implementation")
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
                .default_value("0.0.0.0")
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
        .get_matches();

    let port: u16 = value_t!(matches.value_of("port"), u16).unwrap_or_else(|e| e.exit());
    let connection_limit: u32 =
        value_t!(matches.value_of("conn-limit"), u32).unwrap_or_else(|e| e.exit());
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
                .with_max_level(tracing::Level::WARN)
                .init();
        }
        1 => {
            tracing_subscriber::fmt()
                .with_max_level(tracing::Level::INFO)
                .init();
        }
        2 => {
            tracing_subscriber::fmt()
                .with_max_level(tracing::Level::DEBUG)
                .init();
        }
        3 | _ => {
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

    let addr = SocketAddr::new(listen_address, port);
    let mut tcp_server = memcrs::memcrs::server::TcpServer::new(60, connection_limit);
    tcp_server.run(addr).await
}
