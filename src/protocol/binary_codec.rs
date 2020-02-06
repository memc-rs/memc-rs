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

#[derive(PartialEq, Debug)]
enum RequestParserState {
    None,
    HeaderParsed,
    RequestParsed,
}

pub struct RequestMemcacheBinaryCodec {
    header: binary::RequestHeader,
    state: RequestParserState,
}

impl RequestMemcacheBinaryCodec {
    pub fn new() -> RequestMemcacheBinaryCodec {
        RequestMemcacheBinaryCodec {
            header: binary::RequestHeader {
                magic: 0,
                opcode: 0,
                key_length: 0,
                extras_length: 0,
                data_type: 0,
                reserved: 0,
                body_length: 0,
                opaque: 0,
                cas: 0,
            },
            state: RequestParserState::None,
        }
    }

    pub fn parse_header(&mut self, src: &mut BytesMut) {
        assert!(src.len() >= RequestMemcacheBinaryCodec::HEADER_LEN);

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
        };
        self.state = RequestParserState::HeaderParsed;
    }

    pub fn get_length(&self) -> usize {
        (self.header.extras_length as usize)
            + (self.header.key_length as usize)
            + (self.header.body_length as usize)
    }

    pub fn parse(&mut self, src: &mut BytesMut) -> Option<BinaryRequest> {
        assert!(src.len() >= self.get_length());
        assert!(self.state == RequestParserState::HeaderParsed);
        self.state = RequestParserState::RequestParsed;
        None
    }
}

impl RequestMemcacheBinaryCodec {
    const HEADER_LEN: usize = 24;
}

impl Decoder for RequestMemcacheBinaryCodec {
    type Item = BinaryRequest;
    type Error = io::Error;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        match self.state {
            RequestParserState::None => {
                if src.len() < RequestMemcacheBinaryCodec::HEADER_LEN {
                    return Ok(None);
                }
                self.parse_header(src)
            }
            RequestParserState::HeaderParsed => {
                if src.len() < self.get_length() {
                    return Ok(None);
                }
                return Ok(self.parse(src));
            }
            RequestParserState::RequestParsed => {
                return Err(std::io::Error::new(std::io::ErrorKind::InvalidData, "Invalid data"));
            }
        }
        Ok(None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_encode_decode() {}
}
