use futures_util::sink::SinkExt;
use std::error::Error;
use std::net::{IpAddr, Ipv4Addr, SocketAddr, ToSocketAddrs};
use std::sync::Arc;
use tokio::io;
use tokio::net::{TcpListener, TcpStream, ToSocketAddrs as TokioToSocketAddrs};
use tokio::stream::{Stream, StreamExt as TokioStreamExt};
use tokio_util::codec::{FramedRead, FramedWrite};

use super::handler;
use super::storage;
use crate::protocol::binary_codec;

pub struct TcpServer {
    storage: Arc<storage::Storage>,
}

impl Default for TcpServer {
    fn default() -> Self {
        TcpServer {
            storage: Arc::new(storage::Storage::new()),
        }
    }
}

impl TcpServer {
    pub fn new() -> TcpServer {
        Default::default()
    }

    pub async fn run<A: ToSocketAddrs + TokioToSocketAddrs>(&mut self, addr: A) -> io::Result<()> {
        let mut listener = TcpListener::bind(addr).await?;
        loop {
            match listener.accept().await {
                Ok((mut socket, peer_addr)) => {
                    let db = self.storage.clone();
                    info!("Incoming connection: {}", peer_addr);
                    // Like with other small servers, we'll `spawn` this client to ensure it
                    // runs concurrently with all other clients. The `move` keyword is used
                    // here to move ownership of our db handle into the async closure.
                    tokio::spawn(async move {
                        let mut handler = handler::BinaryHandler::new(db);
                        // Since our protocol is line-based we use `tokio_codecs`'s `LineCodec`
                        // to convert our stream of bytes, `socket`, into a `Stream` of lines
                        // as well as convert our line based responses into a stream of bytes.
                        let (rx, tx) = socket.split();

                        let mut reader =
                            FramedRead::new(rx, binary_codec::MemcacheBinaryCodec::new());
                        let mut writer =
                            FramedWrite::new(tx, binary_codec::MemcacheBinaryCodec::new());

                        // Here for every packet we get back from the `Framed` decoder,
                        // we parse the request, and if it's valid we generate a response
                        // based on the values in the database.
                        while let Some(result) = reader.next().await {
                            match result {
                                Ok(request) => {
                                    let response = handler.handle_request(request);
                                    match response {
                                        Some(response) => {
                                            if let Err(e) = writer.send(response).await {
                                                error!(
                                                    "error on sending response; error = {:?}",
                                                    e
                                                );
                                            }
                                        }
                                        None => {}
                                    }
                                }
                                Err(e) => {
                                    error!("error on decoding from socket; error = {:?}", e);
                                }
                            }
                        }

                        // The connection will be closed at this point as `lines.next()` has returned `None`.
                    });
                }
                Err(e) => error!("error accepting socket; error = {:?}", e),
            }
        }
    }
}
