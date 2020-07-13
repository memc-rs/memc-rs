use simplelog::{CombinedLogger, Config, LevelFilter, WriteLogger};
use std::fs::File;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use tokio::io;

extern crate memix;

#[tokio::main]
async fn main() -> io::Result<()> {
    CombinedLogger::init(vec![WriteLogger::new(
        LevelFilter::Off,
        Config::default(),
        File::create("memix.log").unwrap(),
    )])
    .unwrap();
    let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 11211);
    let mut tcp_server = memix::memcache::server::TcpServer::new();
    tcp_server.run(addr).await
}
