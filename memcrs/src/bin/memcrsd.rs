use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use log::info;
use tokio::io;

extern crate memcrs;
extern crate clap;
use clap::{Arg, App, value_t, Error};

#[tokio::main]
async fn main() -> io::Result<()> {
    let app = App::new("memcrsd");
    let matches = app
                          .version(memcrs::memcache::version::MEMCRS_VERSION)
                          .author("Dariusz Ostolski <dariusz.ostolski@gmail.com>")
                          .about("Rust memcache compatible server implementation")
                          .arg(Arg::with_name("port")
                               .short("p")
                               .long("port")                               
                               .default_value("11211")
                               .help("TCP port to listen on")
                               .takes_value(true))
                           .arg(Arg::with_name("listen")
                               .short("l")
                               .long("listen")                               
                               .default_value("0.0.0.0")
                               .help("interface to listen on")
                               .takes_value(true))
                          .arg(Arg::with_name("conn-limit")
                               .short("c")
                               .long("conn-limit")
                               .default_value("1024")
                               .help("max simultaneous connections")
                               .takes_value(true))
                          .arg(Arg::with_name("v")
                               .short("v")
                               .multiple(true)
                               .help("Sets the level of verbosity"))                          
                          .get_matches();

    let port: u16 = value_t!(matches.value_of("port"), u16).unwrap_or_else(|e| e.exit());
    let connection_limit: u32 = value_t!(matches.value_of("conn-limit"), u32).unwrap_or_else(|e| e.exit());
    let listen_address = matches.value_of("listen").unwrap().parse::<IpAddr>().unwrap_or_else( |e| {
        let clap_error = clap::Error{
            message: e.to_string(),
            kind: clap::ErrorKind::InvalidValue,
            info: None,
        };
        clap_error.exit()
    });
    
    // Vary the output based on how many times the user used the "verbose" flag
    // (i.e. 'myprog -v -v -v' or 'myprog -vvv' vs 'myprog -v'
    match matches.occurrences_of("v") {
        0 => println!("No verbose info"),
        1 => println!("Some verbose info"),
        2 => println!("Tons of verbose info"),
        3 | _ => println!("Don't be crazy"),
    }

    info!("Listen address: {}", matches.value_of("listen").unwrap());
    info!("Listen port: {}", port);
    info!("Connection limit: {}", matches.value_of("conn-limit").unwrap());
    
    let addr = SocketAddr::new(listen_address, port);
    let mut tcp_server = memcrs::memcache::server::TcpServer::new(60, connection_limit);
    tcp_server.run(addr).await
}
