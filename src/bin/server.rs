use std::fs::File;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use tokio::io;

extern crate memix;

#[tokio::main]
async fn main() -> io::Result<()> {
    let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)), 11211);
    let mut tcp_server = memix::memcache::server::TcpServer::new();
    tcp_server.run(addr).await
}
