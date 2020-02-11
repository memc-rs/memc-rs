use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use tokio::io;
use tokio::net::TcpListener;
use tokio::stream::StreamExt;
use tokio_util::codec::{Framed};

use futures::SinkExt;
// We want to use the lines codec with separated variables for read and write, 
// so the LinesCodec + ReadHalf and LinesCodec + WriteHalf are encapsulated in 
// a frame use tokio_util::codec::{FramedRead, FramedWrite, LinesCodec};
use std::error::Error;
extern crate memix;

#[tokio::main]
async fn main() -> io::Result<()> {
    let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 11211);
    let mut listener = TcpListener::bind("127.0.0.1:11211").await?;
    println!("Listening on: {}", addr);

    loop {
        match listener.accept().await {
            Ok((socket, peer_addr)) => {      
                println!("Incoming connection: {}", peer_addr);          
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

                                /*if let Err(e) = packets.send(response).await {
                                    println!("error on sending response; error = {:?}", e);
                                }*/
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


fn handle_request(req: &memix::protocol::binary_codec::BinaryRequest) -> () {
    println!("Received request: {:?}", req);

}