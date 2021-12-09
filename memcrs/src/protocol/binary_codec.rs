use std::{io, u8};

use crate::protocol::binary;
use crate::storage::error::StorageError;
use bytes::{Buf, BufMut, Bytes, BytesMut};
use num_traits::FromPrimitive;
use std::io::{Error, ErrorKind};
use tokio_util::codec::{Decoder, Encoder};

/// Client request
#[derive(Debug)]
pub enum BinaryRequest {
    Delete(binary::DeleteRequest),
    DeleteQuiet(binary::DeleteRequest),
    Get(binary::GetRequest),
    GetQuietly(binary::GetQuietRequest),
    GetKey(binary::GetKeyRequest),
    GetKeyQuietly(binary::GetKeyQuietRequest),
    Set(binary::SetRequest),
    SetQuietly(binary::SetRequest),
    Append(binary::AppendRequest),
    AppendQuietly(binary::AppendRequest),
    Prepend(binary::PrependRequest),
    PrependQuietly(binary::PrependRequest),
    Add(binary::AddRequest),
    AddQuietly(binary::AddRequest),
    Replace(binary::ReplaceRequest),
    ReplaceQuietly(binary::ReplaceRequest),
    Increment(binary::IncrementRequest),
    IncrementQuiet(binary::IncrementRequest),
    Decrement(binary::DecrementRequest),
    DecrementQuiet(binary::DecrementRequest),
    Noop(binary::NoopRequest),
    Flush(binary::FlushRequest),
    FlushQuietly(binary::FlushRequest),
    Version(binary::VersionRequest),
    Quit(binary::QuitRequest),
    QuitQuietly(binary::QuitRequest),
    ItemTooLarge(binary::SetRequest),
    Stats(binary::StatsRequest),
}

impl BinaryRequest {
    pub fn get_header(&'_ self) -> &'_ binary::RequestHeader {
        match self {
            BinaryRequest::Delete(request)
            | BinaryRequest::DeleteQuiet(request)
            | BinaryRequest::Get(request)
            | BinaryRequest::GetKey(request)
            | BinaryRequest::GetKeyQuietly(request)
            | BinaryRequest::GetQuietly(request) => &request.header,

            BinaryRequest::Set(request)
            | BinaryRequest::SetQuietly(request)
            | BinaryRequest::Replace(request)
            | BinaryRequest::ReplaceQuietly(request)
            | BinaryRequest::Add(request)
            | BinaryRequest::AddQuietly(request)
            | BinaryRequest::ItemTooLarge(request) => &request.header,

            BinaryRequest::Prepend(request)
            | BinaryRequest::PrependQuietly(request)
            | BinaryRequest::Append(request)
            | BinaryRequest::AppendQuietly(request) => &request.header,

            BinaryRequest::Increment(request)
            | BinaryRequest::IncrementQuiet(request)
            | BinaryRequest::Decrement(request)
            | BinaryRequest::DecrementQuiet(request) => &request.header,

            BinaryRequest::Noop(request)
            | BinaryRequest::Version(request)
            | BinaryRequest::Stats(request) => &request.header,

            BinaryRequest::Flush(request) | BinaryRequest::FlushQuietly(request) => &request.header,

            BinaryRequest::Quit(request) | BinaryRequest::QuitQuietly(request) => &request.header,
        }
    }
}

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
    err: StorageError,
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

#[derive(PartialEq, Debug)]
enum RequestParserState {
    None,
    HeaderParsed,
}

pub struct MemcacheBinaryCodec {
    header: binary::RequestHeader,
    state: RequestParserState,
    item_size_limit: u32,
}

impl MemcacheBinaryCodec {
    pub fn new(item_size_limit: u32) -> MemcacheBinaryCodec {
        MemcacheBinaryCodec {
            header: Default::default(),
            state: RequestParserState::None,
            item_size_limit,
        }
    }

    fn init_parser(&mut self) {
        self.header = Default::default();
        self.state = RequestParserState::None;
    }

    fn parse_header(&mut self, src: &mut BytesMut) -> Result<(), io::Error> {
        if src.len() < MemcacheBinaryCodec::HEADER_LEN {
            error!("Buffer len is less than MemcacheBinaryCodec::HEADER_LEN");
            return Err(Error::new(
                ErrorKind::InvalidData,
                "Buffer too small cannot parse header",
            ));
        }

        // println!("Header parsed: {:?} ", self.header);
        self.header = binary::RequestHeader {
            magic: src.get_u8(),
            opcode: src.get_u8(),
            key_length: src.get_u16(),
            extras_length: src.get_u8(),
            data_type: src.get_u8(),
            vbucket_id: src.get_u16(),
            body_length: src.get_u32(),
            opaque: src.get_u32(),
            cas: src.get_u64(),
        };

        self.state = RequestParserState::HeaderParsed;
        if !self.header_valid() {
            return Err(Error::new(ErrorKind::InvalidData, "Incorrect header"));
        }

        if self.header.body_length > self.item_size_limit {
            return Ok(());
        }

        src.reserve(self.header.body_length as usize);
        Ok(())
    }

    fn header_valid(&self) -> bool {
        if self.header.magic != binary::Magic::Request as u8 {
            error!("Invalid header: magic != binary::Magic::Request");
            return false;
        }

        if self.header.opcode >= binary::Command::OpCodeMax as u8 {
            error!("Invalid header: opcode >= OpCodeMax");
            return false;
        }

        if self.header.data_type != binary::DataTypes::RawBytes as u8 {
            error!("Invalid header: data_type != binary::DataTypes::RawBytes");
            return false;
        }
        true
    }

    fn parse_request(&mut self, src: &mut BytesMut) -> Result<Option<BinaryRequest>, io::Error> {
        if self.state != RequestParserState::HeaderParsed {
            error!("Incorrect parser state ({:?})", self.state);
            return Err(Error::new(ErrorKind::Other, "Header is not parsed"));
        }

        if self.header.body_length > self.item_size_limit {
            let result = self.parse_item_too_large(src);
            self.init_parser();
            return result;
        }

        if self.header.body_length as usize > src.len() {
            error!(
                "Header body length({:?}) larger than src buffer length({:?})",
                self.header.body_length,
                src.len()
            );
            return Err(Error::new(ErrorKind::Other, "Header body length too large"));
        }

        let result = match FromPrimitive::from_u8(self.header.opcode) {
            Some(binary::Command::Get)
            | Some(binary::Command::GetQuiet)
            | Some(binary::Command::GetKeyQuiet)
            | Some(binary::Command::GetKey) => self.parse_get_request(src),

            Some(binary::Command::Append)
            | Some(binary::Command::AppendQuiet)
            | Some(binary::Command::Prepend)
            | Some(binary::Command::PrependQuiet) => self.parse_append_prepend_request(src),

            Some(binary::Command::Set)
            | Some(binary::Command::SetQuiet)
            | Some(binary::Command::Add)
            | Some(binary::Command::Replace)
            | Some(binary::Command::AddQuiet)
            | Some(binary::Command::ReplaceQuiet) => self.parse_set_request(src),

            Some(binary::Command::Delete) | Some(binary::Command::DeleteQuiet) => {
                self.parse_delete_request(src)
            }

            Some(binary::Command::Increment)
            | Some(binary::Command::Decrement)
            | Some(binary::Command::IncrementQuiet)
            | Some(binary::Command::DecrementQuiet) => self.parse_inc_dec_request(src),

            Some(binary::Command::Noop)
            | Some(binary::Command::Quit)
            | Some(binary::Command::QuitQuiet)
            | Some(binary::Command::Stat)
            | Some(binary::Command::Version) => self.parse_header_only_request(src),

            Some(binary::Command::Flush) | Some(binary::Command::FlushQuiet) => {
                self.parse_flush_request(src)
            }

            Some(binary::Command::Touch)
            | Some(binary::Command::GetAndTouch)
            | Some(binary::Command::GetAndTouchQuiet)
            | Some(binary::Command::GetAndTouchKey)
            | Some(binary::Command::GetAndTouchKeyQuiet)
            | Some(binary::Command::SaslAuth)
            | Some(binary::Command::SaslListMechs)
            | Some(binary::Command::SaslStep) => {
                error!("Command not supported, opcode: {:?}", self.header.opcode);
                Ok(None)
            }

            Some(binary::Command::OpCodeMax) => {
                error!("Incorrect command opcode: {:?}", self.header.opcode);
                Err(Error::new(ErrorKind::InvalidData, "Incorrect opcode"))
            }
            None => {
                // println!("Cannot parse command opcode {:?}", self.header);
                error!("Cannot parse command opcode: {:?}", self.header.opcode);
                Err(Error::new(ErrorKind::InvalidData, "Incorrect op code"))
            }
        };
        self.init_parser();
        result
    }

    fn get_value_len(&self) -> usize {
        (self.header.body_length as usize)
            - ((self.header.key_length + self.header.extras_length as u16) as usize)
    }

    fn parse_get_request(&self, src: &mut BytesMut) -> Result<Option<BinaryRequest>, io::Error> {
        if !self.request_valid(src, true) {
            return Err(Error::new(ErrorKind::InvalidData, "Incorrect get request"));
        }

        let size = self.header.key_length as usize;
        let buf = src.split_to(size);
        let key = buf.to_vec();
        if self.header.opcode == binary::Command::Get as u8 {
            Ok(Some(BinaryRequest::Get(binary::GetRequest {
                header: self.header,
                key,
            })))
        } else if self.header.opcode == binary::Command::GetQuiet as u8 {
            Ok(Some(BinaryRequest::GetQuietly(binary::GetQuietRequest {
                header: self.header,
                key,
            })))
        } else if self.header.opcode == binary::Command::GetKey as u8 {
            Ok(Some(BinaryRequest::GetKey(binary::GetKeyRequest {
                header: self.header,
                key,
            })))
        } else {
            Ok(Some(BinaryRequest::GetKeyQuietly(
                binary::GetKeyQuietRequest {
                    header: self.header,
                    key,
                },
            )))
        }
    }

    fn parse_delete_request(&self, src: &mut BytesMut) -> Result<Option<BinaryRequest>, io::Error> {
        if !self.request_valid(src, true) {
            return Err(Error::new(ErrorKind::InvalidData, "Incorrect get request"));
        }

        let size = self.header.key_length as usize;
        let buf = src.split_to(size);
        let key = buf.to_vec();
        if self.header.opcode == binary::Command::Delete as u8 {
            Ok(Some(BinaryRequest::Delete(binary::DeleteRequest {
                header: self.header,
                key,
            })))
        } else {
            Ok(Some(BinaryRequest::DeleteQuiet(binary::DeleteRequest {
                header: self.header,
                key,
            })))
        }
    }

    fn parse_header_only_request(
        &self,
        src: &mut BytesMut,
    ) -> Result<Option<BinaryRequest>, io::Error> {
        if !self.request_valid(src, false) {
            return Err(Error::new(
                ErrorKind::InvalidData,
                "Incorrect header only request",
            ));
        }
        if self.header.opcode == binary::Command::Noop as u8 {
            Ok(Some(BinaryRequest::Noop(binary::NoopRequest {
                header: self.header,
            })))
        } else if self.header.opcode == binary::Command::Quit as u8 {
            Ok(Some(BinaryRequest::Quit(binary::QuitRequest {
                header: self.header,
            })))
        } else if self.header.opcode == binary::Command::QuitQuiet as u8 {
            Ok(Some(BinaryRequest::QuitQuietly(binary::QuitRequest {
                header: self.header,
            })))
        } else {
            Ok(Some(BinaryRequest::Version(binary::VersionRequest {
                header: self.header,
            })))
        }
    }

    fn parse_flush_request(&self, src: &mut BytesMut) -> Result<Option<BinaryRequest>, io::Error> {
        if !self.request_valid(src, false) {
            return Err(Error::new(
                ErrorKind::InvalidData,
                "Incorrect Flush request",
            ));
        }
        let mut expiration: u32 = 0;
        if self.header.extras_length == 4 {
            expiration = src.get_u32();
        }
        if self.header.opcode == binary::Command::Flush as u8 {
            Ok(Some(BinaryRequest::Flush(binary::FlushRequest {
                header: self.header,
                expiration,
            })))
        } else {
            Ok(Some(BinaryRequest::FlushQuietly(binary::FlushRequest {
                header: self.header,
                expiration,
            })))
        }
    }

    fn parse_append_prepend_request(
        &self,
        src: &mut BytesMut,
    ) -> Result<Option<BinaryRequest>, io::Error> {
        if !self.request_valid(src, true) {
            return Err(Error::new(
                ErrorKind::InvalidData,
                "Incorrect append/prepend request",
            ));
        }
        let value_len = self.get_value_len();
        let append_request = binary::AppendRequest {
            header: self.header,
            key: src.split_to(self.header.key_length as usize).to_vec(),
            value: src.split_to(value_len as usize).freeze(),
        };

        let response = if self.header.opcode == binary::Command::Append as u8 {
            Ok(Some(BinaryRequest::Append(append_request)))
        } else if self.header.opcode == binary::Command::AppendQuiet as u8 {
            Ok(Some(BinaryRequest::AppendQuietly(append_request)))
        } else if self.header.opcode == binary::Command::Prepend as u8 {
            Ok(Some(BinaryRequest::Prepend(append_request)))
        } else {
            Ok(Some(BinaryRequest::PrependQuietly(append_request)))
        };
        response
    }

    fn parse_inc_dec_request(
        &self,
        src: &mut BytesMut,
    ) -> Result<Option<BinaryRequest>, io::Error> {
        if !self.request_valid(src, true) {
            return Err(Error::new(
                ErrorKind::InvalidData,
                "Incorrect inc/dec request",
            ));
        }

        let required_len = 2 * std::mem::size_of::<u64>()
            + std::mem::size_of::<u32>()
            + self.header.key_length as usize;
        if src.len() < required_len {
            error!(
                "[Invalid data]: Buffer length({:?}) smaller than requied length({:?})",
                src.len(),
                required_len
            );
            return Err(Error::new(
                ErrorKind::InvalidData,
                format!(
                    "[Invalid data]: Buffer length({:?}) smaller than requied length({:?})",
                    self.header.body_length, required_len
                ),
            ));
        }

        let request = binary::IncrementRequest {
            header: self.header,
            delta: src.get_u64(),
            initial: src.get_u64(),
            expiration: src.get_u32(),
            key: src.split_to(self.header.key_length as usize).to_vec(),
        };

        let response = if self.header.opcode == binary::Command::Increment as u8 {
            Ok(Some(BinaryRequest::Increment(request)))
        } else if self.header.opcode == binary::Command::IncrementQuiet as u8 {
            Ok(Some(BinaryRequest::IncrementQuiet(request)))
        } else if self.header.opcode == binary::Command::Decrement as u8 {
            Ok(Some(BinaryRequest::Decrement(request)))
        } else {
            Ok(Some(BinaryRequest::DecrementQuiet(request)))
        };
        response
    }

    fn parse_item_too_large(
        &self,
        _src: &mut BytesMut,
    ) -> Result<Option<BinaryRequest>, io::Error> {
        let set_request = binary::SetRequest {
            header: self.header,
            flags: 0,
            expiration: 0,
            key: Vec::new(),
            value: BytesMut::new().freeze(),
        };
        Ok(Some(BinaryRequest::ItemTooLarge(set_request)))
    }

    fn parse_set_request(&self, src: &mut BytesMut) -> Result<Option<BinaryRequest>, io::Error> {
        if !self.request_valid(src, true) {
            return Err(Error::new(ErrorKind::InvalidData, "Incorrect set request"));
        }

        let value_len = self.get_value_len();
        // flags u32 +expiration u32
        let required_len =
            2 * std::mem::size_of::<u32>() + self.header.key_length as usize + value_len;

        if src.len() < required_len {
            error!(
                "[Invalid data]: Buffer length({:?}) smaller than requied length({:?})",
                src.len(),
                required_len
            );
            return Err(Error::new(
                ErrorKind::InvalidData,
                format!(
                    "[Invalid data]: Buffer length({:?}) smaller than requied length({:?})",
                    self.header.body_length, required_len
                ),
            ));
        }

        let set_request = binary::SetRequest {
            header: self.header,
            flags: src.get_u32(),
            expiration: src.get_u32(),
            key: src.split_to(self.header.key_length as usize).to_vec(),
            value: src.split_to(value_len as usize).freeze(),
        };

        let result = match FromPrimitive::from_u8(self.header.opcode) {
            Some(binary::Command::Set) => Ok(Some(BinaryRequest::Set(set_request))),
            Some(binary::Command::SetQuiet) => Ok(Some(BinaryRequest::SetQuietly(set_request))),
            Some(binary::Command::Add) => Ok(Some(BinaryRequest::Add(set_request))),
            Some(binary::Command::AddQuiet) => Ok(Some(BinaryRequest::AddQuietly(set_request))),
            Some(binary::Command::Replace) => Ok(Some(BinaryRequest::Replace(set_request))),
            Some(binary::Command::ReplaceQuiet) => {
                Ok(Some(BinaryRequest::ReplaceQuietly(set_request)))
            }
            None => {
                // println!("Cannot parse command opcode {:?}", self.header);
                error!("Cannot parse set command opcode: {:?}", self.header.opcode);
                Err(Error::new(ErrorKind::InvalidData, "Incorrect op code"))
            }
            _ => {
                error!("Cannot parse set command opcode: {:?}", self.header.opcode);
                Err(Error::new(ErrorKind::InvalidData, "Incorrect op code"))
            }
        };
        result
    }

    fn request_valid(&self, _src: &mut BytesMut, key_required: bool) -> bool {
        if self.header.extras_length > 20 {
            return false;
        }

        if self.header.key_length > 250 {
            return false;
        }

        if key_required && self.header.key_length == 0 {
            return false;
        }

        if self.header.body_length
            < (self.header.key_length + self.header.extras_length as u16) as u32
        {
            return false;
        }

        true
    }
}

impl MemcacheBinaryCodec {
    const HEADER_LEN: usize = 24;
}

impl Decoder for MemcacheBinaryCodec {
    type Item = BinaryRequest;
    type Error = io::Error;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<BinaryRequest>, io::Error> {
        if self.state == RequestParserState::None {
            if src.len() < MemcacheBinaryCodec::HEADER_LEN {
                return Ok(None);
            }
            let result = self.parse_header(src);
            match result {
                Err(error) => return Err(error),
                Ok(()) => {}
            }
        }

        if self.header.body_length > self.item_size_limit {
            let result = self.parse_item_too_large(src);
            self.init_parser();
            return result;
        }

        if (self.header.body_length as usize) > src.len() {
            return Ok(None);
        }
        self.parse_request(src)
    }
}

pub struct ResponseMessage {
    // header+key?+value?
    pub(crate) data: Bytes,
}

impl MemcacheBinaryCodec {
    const RESPONSE_HEADER_LEN: usize = 24;

    pub fn get_length(&self, msg: &BinaryResponse) -> usize {
        self.get_len_from_header(self.get_header(msg))
    }

    fn get_header<'a>(&self, msg: &'a BinaryResponse) -> &'a binary::ResponseHeader {
        msg.get_header()
    }

    fn get_len_from_header(&self, header: &binary::ResponseHeader) -> usize {
        MemcacheBinaryCodec::RESPONSE_HEADER_LEN
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
                if response.key.len() > 0 {
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

    fn write_msg(&self, msg: &BinaryResponse, dst: &mut BytesMut) {
        self.write_header_impl(self.get_header(msg), dst);
        self.write_data(msg, dst)
    }

    pub fn write_header(&self, msg: &BinaryResponse, dst: &mut BytesMut) {
        self.write_header_impl(self.get_header(msg), dst)
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

    fn write_data(&self, msg: &BinaryResponse, dst: &mut BytesMut) {
        match msg {
            BinaryResponse::Error(response) => {
                dst.put(response.error.as_bytes());
            }
            BinaryResponse::Get(response)
            | BinaryResponse::GetKey(response)
            | BinaryResponse::GetKeyQuietly(response)
            | BinaryResponse::GetQuietly(response) => {
                dst.put_u32(response.flags);
                dst.put_slice(&response.key[..]);
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
    }
}

impl Encoder<BinaryResponse> for MemcacheBinaryCodec {
    //type Item = BinaryResponse;
    type Error = io::Error;

    fn encode(&mut self, msg: BinaryResponse, dst: &mut BytesMut) -> Result<(), Self::Error> {
        dst.reserve(self.get_length(&msg));
        self.write_msg(&msg, dst);
        Ok(())
    }
}

#[cfg(test)]
mod binary_decoder_tests;
mod binary_encoder_tests;
