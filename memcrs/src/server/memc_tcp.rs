use futures::sink::SinkExt;
use futures::StreamExt;
use io::AsyncWriteExt;
use std::net::{SocketAddr, ToSocketAddrs};
use std::sync::Arc;
use std::time::Duration;
use tokio::io;
use tokio::io::BufWriter;
use tokio::net::{TcpListener, TcpStream, ToSocketAddrs as TokioToSocketAddrs};
use tokio::sync::Semaphore;
use tokio::time::{interval_at, timeout, Instant};
use tokio_util::codec::{FramedRead, FramedWrite};
use tracing::{debug, error};

//use tracing_attributes::instrument;

use super::handler;
use crate::protocol::binary_codec::{BinaryRequest, BinaryResponse};
use crate::protocol::binary_connection::MemcacheBinaryConnection;
use crate::storage::memcstore as storage;
use crate::storage::timer;
use crate::storage::timer::{SetableTimer, Timer};

pub struct MemcacheServerConfig {
    timeout_secs: u32,
    connection_limit: u32,
    memory_limit: u32,
    item_memory_limit: u32,
}

impl MemcacheServerConfig {
    pub fn new(timeout_secs: u32, connection_limit: u32, memory_limit: u32, item_memory_limit: u32) -> Self {
        MemcacheServerConfig {
            timeout_secs,
            connection_limit,
            memory_limit,
            item_memory_limit
        }
    }
}
pub struct MemcacheTcpServer {
    timer: Arc<timer::SystemTimer>,
    storage: Arc<storage::MemcStore>,
    limit_connections: Arc<Semaphore>,
    config: MemcacheServerConfig,
}

pub struct ClientConfig {
    item_memory_limit: u32,
    rx_timeout_secs: u32,
    wx_timeout_secs: u32,
}
struct Client {
    store: Arc<storage::MemcStore>,
    stream: MemcacheBinaryConnection,
    addr: SocketAddr,
    config: ClientConfig,
    /// Max connection semaphore.
    ///
    /// When the handler is dropped, a permit is returned to this semaphore. If
    /// the listener is waiting for connections to close, it will be notified of
    /// the newly available permit and resume accepting connections.
    limit_connections: Arc<Semaphore>,
}

impl Client {
    pub fn new(
        store: Arc<storage::MemcStore>,
        socket: TcpStream,
        addr: SocketAddr,
        config: ClientConfig,
        limit_connections: Arc<Semaphore>,
    ) -> Self {
        Client {
            store,
            stream: MemcacheBinaryConnection::new(socket),
            addr,
            config,
            limit_connections,
        }
    }
}

impl Drop for Client {
    fn drop(&mut self) {
        // Add a permit back to the semaphore.
        //
        // Doing so unblocks the listener if the max number of
        // connections has been reached.
        //
        // This is done in a `Drop` implementation in order to guarantee that
        // the permit is added even if the task handling the connection panics.
        // If `add_permit` was called at the end of the `run` function and some
        // bug causes a panic. The permit would never be returned to the
        // semaphore.
        self.limit_connections.add_permits(1);
    }
}

impl MemcacheTcpServer {
    pub fn new(config: MemcacheServerConfig) -> MemcacheTcpServer {
        let timer = Arc::new(timer::SystemTimer::new());
        MemcacheTcpServer {
            timer: timer.clone(),
            storage: Arc::new(storage::MemcStore::new(timer)),
            limit_connections: Arc::new(Semaphore::new(config.connection_limit as usize)),
            config: config,
        }
    }

    pub async fn run<A: ToSocketAddrs + TokioToSocketAddrs>(&mut self, addr: A) -> io::Result<()> {
        let listener = TcpListener::bind(addr).await?;

        let start = Instant::now();
        let mut interval = interval_at(start, Duration::from_secs(1));
        loop {
            tokio::select! {
                connection = listener.accept() => {
                    match connection {
                    Ok((socket, addr)) => {
                        let peer_addr = addr;
                        socket.set_nodelay(true)?;
                        socket.set_linger(None)?;
                        let client = Client::new(
                            self.storage.clone(),
                            socket,
                            peer_addr,
                            self.get_client_config(),
                            self.limit_connections.clone()
                        );

                        self.limit_connections.acquire().await.unwrap().forget();
                        // Like with other small servers, we'll `spawn` this client to ensure it
                        // runs concurrently with all other clients. The `move` keyword is used
                        // here to move ownership of our store handle into the async closure.
                        tokio::spawn(async move { MemcacheTcpServer::handle_client(client).await });
                    },
                    Err(err) => {
                        error!("{}", err);
                    }
                }

                },
                _ = interval.tick() => {
                    self.timer.add_second();
                    debug!("Server tick: {}", self.timer.secs());
                },
            }
        }
    }

    fn get_client_config(&self) -> ClientConfig {
        ClientConfig {
            item_memory_limit: self.config.item_memory_limit,
            rx_timeout_secs: self.config.timeout_secs,
            wx_timeout_secs: self.config.timeout_secs
        }
    }
    
    async fn handle_client(mut client: Client) {
        debug!("New client connected: {}", client.addr);
        let handler = handler::BinaryHandler::new(client.store.clone());

        // Here for every packet we get back from the `Framed` decoder,
        // we parse the request, and if it's valid we generate a response
        // based on the values in the storage.
        loop {
            match timeout(
                Duration::from_secs(client.rx_timeout_secs as u64),
                client.stream.read_frame(),
            )
            .await
            {
                Ok(req_or_none) => {
                    match req_or_none {
                        Ok(re) => {
                            match re {
                                Some(request) => {
                                    debug!("Got request {:?}", request.get_header());

                                    if let BinaryRequest::QuitQuietly(_req) = request {
                                        debug!("Closing client socket quit quietly");
                                        if let Err(_e) =
                                            client.stream.shutdown().await.map_err(log_error)
                                        {
                                        }
                                        return;
                                    }

                                    let response = handler.handle_request(request);

                                    if let Some(response) = response {
                                        let mut socket_close = false;
                                        if let BinaryResponse::Quit(_resp) = &response {
                                            socket_close = true;
                                        }

                                        debug!("Sending response {:?}", response);
                                        if let Err(e) = client.stream.write(&response).await {
                                            error!("error on sending response; error = {:?}", e);
                                            return;
                                        }

                                        if socket_close {
                                            debug!("Closing client socket quit command");
                                            if let Err(_e) =
                                                client.stream.shutdown().await.map_err(log_error)
                                            {
                                            }
                                            return;
                                        }
                                    }
                                }
                                None => {
                                    // The connection will be closed at this point as `lines.next()` has returned `None`.
                                    debug!("Connection closed: {}", client.addr);
                                    return;
                                }
                            }
                        }
                        Err(err) => {
                            error!("Error when reading frame; error = {:?}", err);
                            return;
                        }
                    }
                }
                Err(err) => {
                    debug!(
                        "Timeout {}s elapsed, disconecting client: {}, error: {}",
                        client.rx_timeout_secs, client.addr, err
                    );
                    return;
                }
            }
        }
    }
}

fn log_error(e: io::Error) {
    error!("Error: {}", e);
}
