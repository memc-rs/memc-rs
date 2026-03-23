use crate::memcache_server::memc_tcp::MemcacheServerConfig;
use socket2::{Domain, SockAddr, Socket, Type};
use std::net::ToSocketAddrs;
use tracing::{debug, error};

#[derive(Clone, Copy)]
pub struct ListenerFactory {
    config: MemcacheServerConfig,
}

impl ListenerFactory {
    pub fn new(config: MemcacheServerConfig) -> ListenerFactory {
        ListenerFactory { config }
    }

    pub fn get_tcp_listener<A: ToSocketAddrs>(
        self,
        addr: A,
    ) -> Result<std::net::TcpListener, std::io::Error> {
        let socket = Socket::new(Domain::IPV4, Type::STREAM, None)?;
        socket.set_reuse_address(true)?;
        socket.set_reuse_port(true)?;
        socket.set_nonblocking(true)?;
        let addrs_iter = addr.to_socket_addrs()?;
        for socket_addr in addrs_iter {
            debug!("Binding to addr: {:?}", socket_addr);
            let sock_addr = SockAddr::from(socket_addr);
            let res = socket.bind(&sock_addr);
            if let Err(err) = res {
                error!("Can't bind to: {:?}, err {:?}", sock_addr, err);
                return Err(err);
            }
        }

        if let Err(err) = socket.listen(self.config.listen_backlog as i32) {
            error!("Listen error: {:?}", err);
            return Err(err);
        }

        let std_listener: std::net::TcpListener = socket.into();
        Ok(std_listener)
    }
}
