use bytes::{BufMut, Bytes, BytesMut};
use crate::protocol::binary::binary;
use crate::cache::error::CacheError;

/// Server response
#[derive(Debug)]
pub enum BinaryResponse {
    Error(binary::ErrorResponse),
    Get(binary::GetResponse),
    GetQuietly(binary::GetQuietlyResponse),
    GetKey(binary::GetKeyResponse),
    GetKeyQuietly(binary::GetKeyQuietlyResponse),
    Set(binary::SetResponse),
    Add(binary::AddResponse),
    Replace(binary::ReplaceResponse),
    Append(binary::AppendResponse),
    Prepend(binary::PrependResponse),
    Version(binary::VersionResponse),
    Noop(binary::NoopResponse),
    Delete(binary::DeleteResponse),
    Flush(binary::FlushResponse),
    Increment(binary::IncrementResponse),
    Decrement(binary::DecrementResponse),
    Quit(binary::QuitResponse),
    Stats(binary::StatsResponse),
}

impl BinaryResponse {
    pub fn get_header(&'_ self) -> &'_ binary::ResponseHeader {
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
    response_header: &mut binary::ResponseHeader,
) -> BinaryResponse {
    let message = err.to_static_string();
    response_header.status = err as u16;
    response_header.body_length = message.len() as u32;
    BinaryResponse::Error(binary::ErrorResponse {
        header: *response_header,
        error: message,
    })
}

pub struct ResponseMessage {
    // header+key?+value?
    pub(crate) data: Bytes,
}

pub struct MemcacheBinaryEncoder {
}

impl MemcacheBinaryEncoder {
    const RESPONSE_HEADER_LEN: usize = 24;

    pub fn new() -> MemcacheBinaryEncoder {
        MemcacheBinaryEncoder {
        }
    }

    pub fn get_length(&self, msg: &BinaryResponse) -> usize {
        self.get_len_from_header(self.get_header(msg))
    }

    fn get_header<'a>(&self, msg: &'a BinaryResponse) -> &'a binary::ResponseHeader {
        msg.get_header()
    }

    fn get_len_from_header(&self, header: &binary::ResponseHeader) -> usize {
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


    fn write_header_impl(&self, header: &binary::ResponseHeader, dst: &mut BytesMut) {
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
