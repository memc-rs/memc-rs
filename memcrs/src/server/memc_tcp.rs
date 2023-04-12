use socket2::{Domain, SockAddr, Socket, Type};
use std::net::ToSocketAddrs;
use std::sync::Arc;

use tokio::io;
use tokio::net::TcpListener;
use tokio::sync::Semaphore;

use tracing::{debug, error};

//use tracing_attributes::instrument;

use super::client_handler;
use crate::memcache::store as storage;
use crate::storage::cache::Cache;

#[derive(Clone, Copy)]
pub struct MemcacheServerConfig {
    timeout_secs: u32,
    connection_limit: u32,
    item_memory_limit: u32,
    listen_backlog: u32,
}

impl MemcacheServerConfig {
    pub fn new(
        timeout_secs: u32,
        connection_limit: u32,
        item_memory_limit: u32,
        listen_backlog: u32,
    ) -> Self {
        MemcacheServerConfig {
            timeout_secs,
            connection_limit,
            item_memory_limit,
            listen_backlog,
        }
    }
}
#[derive(Clone)]
pub struct MemcacheTcpServer {
    storage: Arc<storage::MemcStore>,
    limit_connections: Arc<Semaphore>,
    config: MemcacheServerConfig,
}

impl MemcacheTcpServer {
    pub fn new(
        config: MemcacheServerConfig,
        store: Arc<dyn Cache + Send + Sync>,
    ) -> MemcacheTcpServer {
        MemcacheTcpServer {
            storage: Arc::new(storage::MemcStore::new(store)),
            limit_connections: Arc::new(Semaphore::new(config.connection_limit as usize)),
            config,
        }
    }

    pub async fn run<A: ToSocketAddrs>(&mut self, addr: A) -> io::Result<()> {
        let listener = self.get_tcp_listener(addr)?;
        loop {
            tokio::select! {
                connection = listener.accept() => {
                    match connection {
                        Ok((socket, addr)) => {
                            let peer_addr = addr;
                            socket.set_nodelay(true)?;
                            socket.set_linger(None)?;
                            let mut client = client_handler::Client::new(
                                Arc::clone(&self.storage),
                                socket,
                                peer_addr,
                                self.get_client_config(),
                                Arc::clone(&self.limit_connections)
                            );

                            self.limit_connections.acquire().await.unwrap().forget();
                            // Like with other small servers, we'll `spawn` this client to ensure it
                            // runs concurrently with all other clients. The `move` keyword is used
                            // here to move ownership of our store handle into the async closure.
                            tokio::spawn(async move { client.handle().await });
                        },
                        Err(err) => {
                            error!("Accept error: {}", err);
                        }
                    }
                }
            }
        }
    }

    fn get_tcp_listener<A: ToSocketAddrs>(
        &mut self,
        addr: A,
    ) -> Result<TcpListener, std::io::Error> {
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
        TcpListener::from_std(std_listener)
    }

    fn get_client_config(&self) -> client_handler::ClientConfig {
        client_handler::ClientConfig {
            item_memory_limit: self.config.item_memory_limit,
            rx_timeout_secs: self.config.timeout_secs,
            _wx_timeout_secs: self.config.timeout_secs,
        }
    }
}
