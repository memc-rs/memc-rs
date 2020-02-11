use tokio::net::TcpListener;
use tokio::stream::StreamExt;
use tokio_util::codec::{Framed};

use futures::SinkExt;
use std::collections::HashMap;
use std::env;
use std::error::Error;
use std::sync::{Arc, Mutex};
extern crate memix;

#[tokio::main]
async fn main() {
    let addr = IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1), 11211);
    let mut listener = TcpListener::bind(addr).await?;
    println!("Listening on: {}", addr);

    loop {
        match listener.accept().await {
            Ok((socket, _)) => {                
                // Like with other small servers, we'll `spawn` this client to ensure it
                // runs concurrently with all other clients. The `move` keyword is used
                // here to move ownership of our db handle into the async closure.
                tokio::spawn(async move {
                    // Since our protocol is line-based we use `tokio_codecs`'s `LineCodec`
                    // to convert our stream of bytes, `socket`, into a `Stream` of lines
                    // as well as convert our line based responses into a stream of bytes.
                    let mut packets = Framed::new(socket, memix::protocol::binary_codec::MemcacheBinaryCodec::new());                    

                    // Here for every line we get back from the `Framed` decoder,
                    // we parse the request, and if it's valid we generate a response
                    // based on the values in the database.
                    while let Some(result) = packets.next().await {
                        match result {
                            Ok(request) => {
                                let response = handle_request(&request);

                                let response = response.serialize();

                                if let Err(e) = packets.send(response).await {
                                    println!("error on sending response; error = {:?}", e);
                                }
                            }
                            Err(e) => {
                                println!("error on decoding from socket; error = {:?}", e);
                            }
                        }
                    }

                    // The connection will be closed at this point as `lines.next()` has returned `None`.
                });
            }
            Err(e) => println!("error accepting socket; error = {:?}", e),
        }
    }
}


fn handle_request(req: &BinaryRequest) -> BinaryResponse {
    println!("Received request: {:?}", req);

}