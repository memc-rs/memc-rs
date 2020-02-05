use std::io;

use crate::protocol::binary;
use actix::Message;
use byteorder::{BigEndian, ByteOrder};
use bytes::{Buf, BufMut, BytesMut};
use serde_derive::{Deserialize, Serialize};
use tokio_util::codec::{Decoder, Encoder};

/// Client request
#[derive(Serialize, Deserialize, Debug, Message)]
#[rtype(result = "()")]
pub enum BinaryRequest {
    Get(binary::GetRequest),
    GetQuietly(binary::GetQuietRequest),
    GetKey(binary::GetKeyRequest),
    GetKeyQuietly(binary::GetKeyQuietRequest),
    Set(binary::SetRequest),
    Add(binary::AddRequest),
    Replace(binary::ReplaceRequest),
}

/// Server response
#[derive(Serialize, Deserialize, Debug, Message)]
#[rtype(result = "()")]
pub enum BinaryResponse {
    Get(binary::GetResponse),
    GetQuietly(binary::GetQuietlyResponse),
    GetKey(binary::GetKeyResponse),
    GetKeyQuietly(binary::GetKeyQuietlyResponse),
    Set(binary::SetResponse),
    Add(binary::AddResponse),
    Replace(binary::ReplaceResponse),
}

pub struct MemcacheBinaryCodec {
    header: binary::RequestHeader
}

impl MemcacheBinaryCodec {
    pub fn header_from_slice(&mut self, src: &mut BytesMut) {
        assert!(src.len() >= 24);

        self.header = binary::RequestHeader {
            magic: src.get_u8(),
            opcode: src.get_u8(),
            key_length: BigEndian::read_u16(&src),
            extras_length: src.get_u8(),
            data_type: src.get_u8(),
            reserved: BigEndian::read_u16(&src),
            body_length: BigEndian::read_u32(&src),
            opaque: BigEndian::read_u32(&src),
            cas: BigEndian::read_u64(&src),
        }
    }
}

impl MemcacheBinaryCodec {
    const HEADER_LEN: usize = 24;
}

impl Decoder for MemcacheBinaryCodec {
    type Item = BinaryRequest;
    type Error = io::Error;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        let size = {
            if src.len() < MemcacheBinaryCodec::HEADER_LEN {
                return Ok(None);
            }
        };
        Ok(None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_encode_decode() {}
}
