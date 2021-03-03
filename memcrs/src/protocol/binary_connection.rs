use futures_util::__private::async_await;
use tokio::io::{AsyncReadExt, AsyncWriteExt, BufWriter};
use tokio::net::TcpStream;
use tokio_util::codec::{Decoder, Encoder};
use std::io::{Error, ErrorKind};
use crate::protocol::binary_codec::{MemcacheBinaryCodec, BinaryRequest, BinaryResponse};
use std::{io, u8};
use bytes::{Buf, BufMut, BytesMut};

pub struct MemcacheBinaryConnection {
    pub(crate) stream: TcpStream,
    codec: MemcacheBinaryCodec,
}


impl  MemcacheBinaryConnection  {
    const SOCKET_BUFFER: usize  = 4096;
    pub fn new(socket: TcpStream) -> Self {
        MemcacheBinaryConnection {
            stream: socket,
            codec: MemcacheBinaryCodec::new()
        }
    }

    pub async fn read_frame(&mut self) -> Result<Option<BinaryRequest>, io::Error> {
        let mut buffer = BytesMut::with_capacity(24);
        loop {            
            // Attempt to parse a frame from the buffered data. If enough data
            // has been buffered, the frame is returned.
            if let Some(frame) = self.codec.decode(&mut buffer)? {
                buffer.clear();
                return Ok(Some(frame));
            }

            // There is not enough buffered data to read a frame. Attempt to
            // read more data from the socket.
            //
            // On success, the number of bytes is returned. `0` indicates "end
            // of stream".
            if 0 == self.stream.read_buf(&mut buffer).await? {
                // The remote closed the connection. For this to be a clean
                // shutdown, there should be no data in the read buffer. If
                // there is, this means that the peer closed the socket while
                // sending a frame.
                if buffer.is_empty() {
                    return Ok(None);
                } else {
                    return Err(Error::new(
                        ErrorKind::ConnectionReset,
                        "Connection reset by peer",
                    ));
                }
            }
        }
    }

    pub async fn write(&mut self, msg: &BinaryResponse) -> io::Result<()> {
        let mut dst = BytesMut::with_capacity(self.codec.get_length(&msg));        
        self.codec.write_header(msg, &mut dst);
        self.write_data_to_stream(msg, &mut dst).await?;
        Ok(())
    }

    async fn write_data_to_stream(&mut self, msg: &BinaryResponse, dst: &mut BytesMut) -> io::Result<()> {
        let response_len = self.codec.get_length(msg);
        let buffered_write = response_len <  MemcacheBinaryConnection::SOCKET_BUFFER;
        match msg {
            BinaryResponse::Error(response) => {
                dst.put(response.error.as_bytes());
            }
            BinaryResponse::Get(response)
            | BinaryResponse::GetKey(response)
            | BinaryResponse::GetKeyQuietly(response)
            | BinaryResponse::GetQuietly(response) => {
                dst.put_u32(response.flags);
                if buffered_write {
                    if response.key.len() > 0 {
                        dst.put_slice(&response.key[..]);
                    }
                    dst.put(response.value.clone());
                }              
            }
            BinaryResponse::Set(_response)
            | BinaryResponse::Replace(_response)
            | BinaryResponse::Add(_response)
            | BinaryResponse::Append(_response)
            | BinaryResponse::Prepend(_response) => {}
            BinaryResponse::Version(response) => {
                dst.put_slice(response.version.as_bytes());                
            }
            BinaryResponse::Noop(_response) => {}
            BinaryResponse::Delete(_response) => {}
            BinaryResponse::Flush(_response) => {}
            BinaryResponse::Quit(_response) => {}
            BinaryResponse::Increment(response) | BinaryResponse::Decrement(response) => {
                dst.put_u64(response.value);
            }
        }
        self.stream.write_all(&dst[..]).await?;
        match msg {
            BinaryResponse::Get(response)
            | BinaryResponse::GetKey(response)
            | BinaryResponse::GetKeyQuietly(response)
            | BinaryResponse::GetQuietly(response) => {                
                if buffered_write == false {
                    if response.key.len() > 0 {
                        self.stream.write_all(&response.key).await?;
                    }
                    self.stream.write_all(&response.value).await?;                
                }                
            }            
            _ => {                
            }
        }
        Ok(())        
    }

    pub async fn shutdown(&mut self) -> io::Result<()> {
        self.stream.shutdown().await?;  
        Ok(())      
    }
}