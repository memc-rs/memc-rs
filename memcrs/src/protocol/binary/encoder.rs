use crate::cache::error::CacheError;
use crate::protocol::binary::network;
use bytes::{BufMut, Bytes, BytesMut};

/// Server response
#[derive(Debug)]
pub enum BinaryResponse {
    Error(network::ErrorResponse),
    Get(network::GetResponse),
    GetQuietly(network::GetQuietlyResponse),
    GetKey(network::GetKeyResponse),
    GetKeyQuietly(network::GetKeyQuietlyResponse),
    Set(network::SetResponse),
    Add(network::AddResponse),
    Replace(network::ReplaceResponse),
    Append(network::AppendResponse),
    Prepend(network::PrependResponse),
    Version(network::VersionResponse),
    Noop(network::NoopResponse),
    Delete(network::DeleteResponse),
    Flush(network::FlushResponse),
    Increment(network::IncrementResponse),
    Decrement(network::DecrementResponse),
    Quit(network::QuitResponse),
    Stats(network::StatsResponse),
}

impl BinaryResponse {
    pub fn get_header(&'_ self) -> &'_ network::ResponseHeader {
        match self {
            BinaryResponse::Error(response) => &response.header,
            BinaryResponse::Get(response) => &response.header,
            BinaryResponse::GetKey(response) => &response.header,
            BinaryResponse::GetKeyQuietly(response) => &response.header,
            BinaryResponse::GetQuietly(response) => &response.header,
            BinaryResponse::Set(response) => &response.header,
            BinaryResponse::Replace(response) => &response.header,
            BinaryResponse::Add(response) => &response.header,
            BinaryResponse::Append(response) => &response.header,
            BinaryResponse::Prepend(response) => &response.header,
            BinaryResponse::Version(response) => &response.header,
            BinaryResponse::Noop(response) => &response.header,
            BinaryResponse::Delete(response) => &response.header,
            BinaryResponse::Flush(response) => &response.header,
            BinaryResponse::Increment(response) => &response.header,
            BinaryResponse::Decrement(response) => &response.header,
            BinaryResponse::Quit(response) => &response.header,
            BinaryResponse::Stats(response) => &response.header,
        }
    }
}

pub fn storage_error_to_response(
    err: CacheError,
    response_header: &mut network::ResponseHeader,
) -> BinaryResponse {
    let message = err.to_static_string();
    response_header.status = err as u16;
    response_header.body_length = message.len() as u32;
    BinaryResponse::Error(network::ErrorResponse {
        header: *response_header,
        error: message,
    })
}

pub struct ResponseMessage {
    // header+key?+value?
    pub(crate) data: Bytes,
}

pub struct MemcacheBinaryEncoder {}
impl Default for MemcacheBinaryEncoder {
    fn default() -> Self {
        Self::new()
    }
}

impl MemcacheBinaryEncoder {
    const RESPONSE_HEADER_LEN: usize = 24;

    pub fn new() -> MemcacheBinaryEncoder {
        MemcacheBinaryEncoder {}
    }

    pub fn get_length(&self, msg: &BinaryResponse) -> usize {
        self.get_len_from_header(self.get_header(msg))
    }

    fn get_header<'a>(&self, msg: &'a BinaryResponse) -> &'a network::ResponseHeader {
        msg.get_header()
    }

    fn get_len_from_header(&self, header: &network::ResponseHeader) -> usize {
        MemcacheBinaryEncoder::RESPONSE_HEADER_LEN
            + (header.body_length as usize)
            + (header.extras_length as usize)
    }

    ///
    /// Encodes a msg into a dst
    // if msg value is large i.e. bigger than SOCKET_BUFFER to avoid double buffering
    // it is returned as  Option<Bytes> so there are no
    // necessary copies made into dst and can be
    // written into socket directly.
    //
    pub fn encode_message(&self, msg: &BinaryResponse) -> ResponseMessage {
        let len = self.get_length(msg);
        let mut dst = BytesMut::with_capacity(len);
        self.write_header_impl(self.get_header(msg), &mut dst);
        self.encode_data(msg, dst)
    }

    fn encode_data(&self, msg: &BinaryResponse, mut dst: BytesMut) -> ResponseMessage {
        match msg {
            BinaryResponse::Error(response) => {
                dst.put(response.error.as_bytes());
            }
            BinaryResponse::Get(response)
            | BinaryResponse::GetKey(response)
            | BinaryResponse::GetKeyQuietly(response)
            | BinaryResponse::GetQuietly(response) => {
                dst.put_u32(response.flags);
                if !response.key.is_empty() {
                    dst.put_slice(&response.key[..]);
                }
                dst.put(response.value.clone());
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
            BinaryResponse::Stats(_response) => {}
            BinaryResponse::Increment(response) | BinaryResponse::Decrement(response) => {
                dst.put_u64(response.value);
            }
        }
        ResponseMessage { data: dst.freeze() }
    }

    fn write_header_impl(&self, header: &network::ResponseHeader, dst: &mut BytesMut) {
        dst.put_u8(header.magic);
        dst.put_u8(header.opcode);
        dst.put_u16(header.key_length);
        dst.put_u8(header.extras_length);
        dst.put_u8(header.data_type);
        dst.put_u16(header.status);
        dst.put_u32(header.body_length);
        dst.put_u32(header.opaque);
        dst.put_u64(header.cas);
    }
}

#[cfg(test)]
mod binary_encoder_tests;
