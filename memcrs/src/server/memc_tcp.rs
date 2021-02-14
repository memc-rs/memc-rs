use futures::sink::SinkExt;
use io::AsyncWriteExt;
use std::net::{SocketAddr, ToSocketAddrs};
use std::sync::Arc;
use std::time::Duration;
use tokio::io;
use tokio::net::{TcpListener, TcpStream, ToSocketAddrs as TokioToSocketAddrs};
use futures::StreamExt;
use tokio::time::{interval_at, timeout, Instant};
use tokio_util::codec::{FramedRead, FramedWrite};
use tracing::{debug, error, info};
//use tracing_attributes::instrument;

use super::handler;
use crate::protocol::binary_codec::{BinaryRequest, BinaryResponse, MemcacheBinaryCodec};
use crate::storage::memcstore as storage;
use crate::storage::timer;
use crate::storage::timer::{SetableTimer, Timer};

//extern crate flame;
pub struct MemcacheServerConfig {
    connection_limit: u32,
    memory_limit: u32, 
}

pub struct MemcacheTcpServer {
    timer: Arc<timer::SystemTimer>,
    storage: Arc<storage::MemcStore>,
    timeout_secs: u64,
    connection_limit: u32,
}

struct Client {
    store: Arc<storage::MemcStore>,
    socket: TcpStream,
    addr: SocketAddr,
    _token: u32,
    rx_timeout_secs: u64,
    wx_timeout_secs: u64,
}

impl Client {
    pub fn new(
        store: Arc<storage::MemcStore>,
        socket: TcpStream,
        addr: SocketAddr,
        token: u32,
        rx_timeout_secs: u64,
        wx_timeout_secs: u64,
    ) -> Self {
        Client {
            store,
            socket,
            addr,
            _token: token,
            rx_timeout_secs,
            wx_timeout_secs,
        }
    }
}

impl MemcacheTcpServer {
    pub fn new(timeout_secs: u64, connection_limit: u32) -> MemcacheTcpServer {
        let timer = Arc::new(timer::SystemTimer::new());
        MemcacheTcpServer {
            connection_limit,
            timeout_secs,
            timer: timer.clone(),
            storage: Arc::new(storage::MemcStore::new(timer)),
        }
    }

    pub async fn run<A: ToSocketAddrs + TokioToSocketAddrs>(&mut self, addr: A) -> io::Result<()> {
        let mut listener = TcpListener::bind(addr).await?;
        // TODO: limit number of accepted connections just like memcache
        /*let mut incoming = listener
        .incoming()
        .log_warnings(log_accept_error)
        .handle_errors(Duration::from_millis(500)) // 1
        .backpressure(self.connection_limit as usize);*/

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
                            0,
                            self.timeout_secs,
                            self.timeout_secs,
                        );
                        // Like with other small servers, we'll `spawn` this client to ensure it
                        // runs concurrently with all other clients. The `move` keyword is used
                        // here to move ownership of our db handle into the async closure.
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

    async fn handle_client(mut client: Client) {
        debug!("New client connected: {}", client.addr);
        let handler = handler::BinaryHandler::new(client.store);
        //
        let (rx, tx) = client.socket.split();

        let mut reader = FramedRead::new(rx, MemcacheBinaryCodec::new());
        let mut writer = FramedWrite::new(tx, MemcacheBinaryCodec::new());

        // Here for every packet we get back from the `Framed` decoder,
        // we parse the request, and if it's valid we generate a response
        // based on the values in the storage.
        loop {
            match timeout(Duration::from_secs(client.rx_timeout_secs), reader.next()).await {
                Ok(req_or_none) => {
                    match req_or_none {
                        Some(req_or_error) => match req_or_error {
                            Ok(request) => {
                                debug!("Got request {:?}", request.get_header());

                                if let BinaryRequest::QuitQuietly(_req) = request {
                                    debug!("Closing client socket quit quietly");
                                    client.socket.shutdown();
                                    return;
                                }
                                
                                let response = handler.handle_request(request);

                                if let Some(response) = response {
                                    let mut socketClose = false;
                                    if let BinaryResponse::Quit(_resp) = &response {
                                        socketClose = true;                                    
                                    }

                                    debug!("Sending response {:?}", response);
                                    if let Err(e) = writer.send(response).await {
                                        error!("error on sending response; error = {:?}", e);
                                        return;
                                    }

                                    if socketClose {
                                        debug!("Closing client socket quit command");
                                        client.socket.shutdown();
                                        return;
                                    }
                                    
                                }
                            }
                            Err(e) => {
                                error!("Error decoding msg from socket; error = {:?}", e);
                            }
                        },
                        None => {
                            // The connection will be closed at this point as `lines.next()` has returned `None`.
                            debug!("Connection closed: {}", client.addr);
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

fn log_accept_error(e: &io::Error) {
    error!("Error: {}. Listener paused for 0.5s.", e); // 3
}
