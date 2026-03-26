use std::sync::Arc;

use tokio::io;
use tokio::net::TcpListener;
use tokio::sync::Semaphore;
use tokio_util::sync::CancellationToken;
use tracing::error;

//use tracing_attributes::instrument;

use super::client_handler;
use crate::cache::cache::Cache;
use crate::memcache::store as storage;

#[derive(Clone, Copy)]
pub struct MemcacheServerConfig {
    pub timeout_secs: u32,
    pub connection_limit: u32,
    pub item_memory_limit: u32,
}

impl MemcacheServerConfig {
    pub fn new(timeout_secs: u32, connection_limit: u32, item_memory_limit: u32) -> Self {
        MemcacheServerConfig {
            timeout_secs,
            connection_limit,
            item_memory_limit,
        }
    }
}
#[derive(Clone)]
pub struct MemcacheTcpServer {
    storage: Arc<storage::MemcStore>,
    limit_connections: Arc<Semaphore>,
    config: MemcacheServerConfig,
    cancellation_token: CancellationToken,
}

impl MemcacheTcpServer {
    pub fn new(
        config: MemcacheServerConfig,
        store: Arc<dyn Cache + Send + Sync>,
        cancellation_token: CancellationToken,
    ) -> MemcacheTcpServer {
        MemcacheTcpServer {
            storage: Arc::new(storage::MemcStore::new(store)),
            limit_connections: Arc::new(Semaphore::new(config.connection_limit as usize)),
            config,
            cancellation_token,
        }
    }

    pub async fn run(&mut self, std_listener: std::net::TcpListener) -> io::Result<()> {
        let listener = TcpListener::from_std(std_listener).unwrap_or_else(|e| {
            log::error!("Failed to create Tokio TCP listener: {}", e);
            std::process::exit(1);
        });

        loop {
            tokio::select! {
                connection = listener.accept() => {
                    match connection {
                        Ok((socket, addr)) => {
                            let peer_addr = addr;
                            socket.set_nodelay(true).unwrap_or_else(|err| {
                                log::error!("System call set_nodelay failure: {}", err);
                            });
                            socket.set_zero_linger().unwrap_or_else(|err| {
                                log::error!("System call set_zero_linger failure: {}", err);
                            });
                            let mut client = client_handler::Client::new(
                                Arc::clone(&self.storage),
                                socket,
                                peer_addr,
                                self.get_client_config(),
                                Arc::clone(&self.limit_connections),
                                self.cancellation_token.clone()
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
                 _ = self.cancellation_token.cancelled() => {
                        log::info!("Cancelling server loop...");
                        break io::Result::Ok(());
                }
            }
        }
    }

    fn get_client_config(&self) -> client_handler::ClientConfig {
        client_handler::ClientConfig {
            item_memory_limit: self.config.item_memory_limit,
            rx_timeout_secs: self.config.timeout_secs,
            _wx_timeout_secs: self.config.timeout_secs,
        }
    }
}
