
use std::net::{SocketAddr};
use std::sync::Arc;
use std::time::Duration;
use tokio::io;
use tokio::net::{TcpStream};
use tokio::sync::Semaphore;
use tokio::time::{timeout};
use tracing::{debug, error};

//use tracing_attributes::instrument;

use super::handler;
use crate::protocol::binary_codec::{BinaryRequest, BinaryResponse};
use crate::protocol::binary_connection::MemcacheBinaryConnection;
use crate::storage::memcstore as storage;


pub struct ClientConfig {
    pub(crate) item_memory_limit: u32,
    pub(crate) rx_timeout_secs: u32,
    pub(crate) wx_timeout_secs: u32,
}
pub struct Client {    
    stream: MemcacheBinaryConnection,
    addr: SocketAddr,
    config: ClientConfig,
    handler: handler::BinaryHandler,
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
            stream: MemcacheBinaryConnection::new(socket, config.item_memory_limit),
            addr,
            config,
            handler:  handler::BinaryHandler::new(store),
            limit_connections,
        }
    }

    pub async fn handle(&mut self) {        
        debug!("New client connected: {}", self.addr);
        

        // Here for every packet we get back from the `Framed` decoder,
        // we parse the request, and if it's valid we generate a response
        // based on the values in the storage.
        loop {
            match timeout(
                Duration::from_secs(self.config.rx_timeout_secs as u64),
                self.stream.read_frame(),
            )
            .await
            {
                Ok(req_or_none) => {                    
                    let finished = self.handle_frame(req_or_none).await;
                    if finished {
                        return;
                    }
                }
                Err(err) => {
                    debug!(
                        "Timeout {}s elapsed, disconecting client: {}, error: {}",
                        self.config.rx_timeout_secs, self.addr, err
                    );
                    return;
                }
            }
        }
    }

    async fn handle_frame(&mut self, req: Result<Option<BinaryRequest>, io::Error>) -> bool {
        match req {
            Ok(re) => {
                match re {
                    Some(request) => {
                        return self.handle_request(request).await
                    }
                    None => {
                        // The connection will be closed at this point as `lines.next()` has returned `None`.
                        debug!("Connection closed: {}", self.addr);
                        return true;
                    }
                }
            }
            Err(err) => {
                error!("Error when reading frame; error = {:?}", err);
                return true;
            }
        }        
    }

    async fn handle_request(&mut self, request: BinaryRequest) -> bool {
        debug!("Got request {:?}", request.get_header());

        if let BinaryRequest::QuitQuietly(_req) = request {
            debug!("Closing client socket quit quietly");
            if let Err(_e) =
                self.stream.shutdown().await.map_err(log_error)
            {
            }
            return true;
        }

        let resp = self.handler.handle_request(request);
        match resp {
            Some(response) => {
                let mut socket_close = false;
                if let BinaryResponse::Quit(_resp) = &response {
                    socket_close = true;
                }
    
                debug!("Sending response {:?}", response);
                if let Err(e) = self.stream.write(&response).await {
                    error!("error on sending response; error = {:?}", e);
                    return true;
                }
    
                if socket_close {
                    debug!("Closing client socket quit command");
                    if let Err(_e) =
                        self.stream.shutdown().await.map_err(log_error)
                    {
                    }
                    return true;
                } 
                return false;
            }
            None => {
                return true;
            }
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

fn log_error(e: io::Error) {
    error!("Error: {}", e);
}