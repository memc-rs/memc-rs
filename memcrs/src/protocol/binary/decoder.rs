use crate::protocol::binary::network;
use bytes::{Buf, BytesMut};
use num_traits::FromPrimitive;
use std::io;
use std::io::{Error, ErrorKind};
use tokio_util::codec::Decoder;

/// Client request
#[derive(Debug)]
pub enum BinaryRequest {
    Delete(network::DeleteRequest),
    DeleteQuiet(network::DeleteRequest),
    Get(network::GetRequest),
    GetQuietly(network::GetQuietRequest),
    GetKey(network::GetKeyRequest),
    GetKeyQuietly(network::GetKeyQuietRequest),
    Set(network::SetRequest),
    SetQuietly(network::SetRequest),
    Append(network::AppendRequest),
    AppendQuietly(network::AppendRequest),
    Prepend(network::PrependRequest),
    PrependQuietly(network::PrependRequest),
    Add(network::AddRequest),
    AddQuietly(network::AddRequest),
    Replace(network::ReplaceRequest),
    ReplaceQuietly(network::ReplaceRequest),
    Increment(network::IncrementRequest),
    IncrementQuiet(network::IncrementRequest),
    Decrement(network::DecrementRequest),
    DecrementQuiet(network::DecrementRequest),
    Noop(network::NoopRequest),
    Flush(network::FlushRequest),
    FlushQuietly(network::FlushRequest),
    Version(network::VersionRequest),
    Quit(network::QuitRequest),
    QuitQuietly(network::QuitRequest),
    ItemTooLarge(network::SetRequest),
    Stats(network::StatsRequest),
}

impl BinaryRequest {
    pub fn get_header(&'_ self) -> &'_ network::RequestHeader {
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

#[derive(PartialEq, Debug)]
enum RequestParserState {
    None,
    HeaderParsed,
}

pub struct MemcacheBinaryDecoder {
    header: network::RequestHeader,
    state: RequestParserState,
    item_size_limit: u32,
}

impl MemcacheBinaryDecoder {
    pub fn new(item_size_limit: u32) -> MemcacheBinaryDecoder {
        MemcacheBinaryDecoder {
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
        if src.len() < MemcacheBinaryDecoder::HEADER_LEN {
            error!("Buffer len is less than MemcacheBinaryCodec::HEADER_LEN");
            return Err(Error::new(
                ErrorKind::InvalidData,
                "Buffer too small cannot parse header",
            ));
        }

        // println!("Header parsed: {:?} ", self.header);
        self.header = network::RequestHeader {
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
        if self.header.magic != network::Magic::Request as u8 {
            error!("Invalid header: magic != binary::Magic::Request");
            return false;
        }

        if self.header.opcode >= network::Command::OpCodeMax as u8 {
            error!("Invalid header: opcode >= OpCodeMax");
            return false;
        }

        if self.header.data_type != network::DataTypes::RawBytes as u8 {
            error!("Invalid header: data_type != binary::DataTypes::RawBytes");
            return false;
        }
        true
    }

    fn parse_request(&mut self, src: &mut BytesMut) -> Result<Option<BinaryRequest>, io::Error> {
        if self.state != RequestParserState::HeaderParsed {
            error!("Incorrect parser state ({:?})", self.state);
            return Err(std::io::Error::other(
                "Incorrect parser state, header not parsed".to_string(),
            ));
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
            return Err(std::io::Error::other(
                "Header body length too large".to_string(),
            ));
        }

        let result = match FromPrimitive::from_u8(self.header.opcode) {
            Some(network::Command::Get)
            | Some(network::Command::GetQuiet)
            | Some(network::Command::GetKeyQuiet)
            | Some(network::Command::GetKey) => self.parse_get_request(src),

            Some(network::Command::Append)
            | Some(network::Command::AppendQuiet)
            | Some(network::Command::Prepend)
            | Some(network::Command::PrependQuiet) => self.parse_append_prepend_request(src),

            Some(network::Command::Set)
            | Some(network::Command::SetQuiet)
            | Some(network::Command::Add)
            | Some(network::Command::Replace)
            | Some(network::Command::AddQuiet)
            | Some(network::Command::ReplaceQuiet) => self.parse_set_request(src),

            Some(network::Command::Delete) | Some(network::Command::DeleteQuiet) => {
                self.parse_delete_request(src)
            }

            Some(network::Command::Increment)
            | Some(network::Command::Decrement)
            | Some(network::Command::IncrementQuiet)
            | Some(network::Command::DecrementQuiet) => self.parse_inc_dec_request(src),

            Some(network::Command::Noop)
            | Some(network::Command::Quit)
            | Some(network::Command::QuitQuiet)
            | Some(network::Command::Stat)
            | Some(network::Command::Version) => self.parse_header_only_request(src),

            Some(network::Command::Flush) | Some(network::Command::FlushQuiet) => {
                self.parse_flush_request(src)
            }

            Some(network::Command::Touch)
            | Some(network::Command::GetAndTouch)
            | Some(network::Command::GetAndTouchQuiet)
            | Some(network::Command::GetAndTouchKey)
            | Some(network::Command::GetAndTouchKeyQuiet)
            | Some(network::Command::SaslAuth)
            | Some(network::Command::SaslListMechs)
            | Some(network::Command::SaslStep) => {
                error!("Command not supported, opcode: {:?}", self.header.opcode);
                Ok(None)
            }

            Some(network::Command::OpCodeMax) => {
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
        let key = buf.freeze();
        if self.header.opcode == network::Command::Get as u8 {
            Ok(Some(BinaryRequest::Get(network::GetRequest {
                header: self.header,
                key,
            })))
        } else if self.header.opcode == network::Command::GetQuiet as u8 {
            Ok(Some(BinaryRequest::GetQuietly(network::GetQuietRequest {
                header: self.header,
                key,
            })))
        } else if self.header.opcode == network::Command::GetKey as u8 {
            Ok(Some(BinaryRequest::GetKey(network::GetKeyRequest {
                header: self.header,
                key,
            })))
        } else {
            Ok(Some(BinaryRequest::GetKeyQuietly(
                network::GetKeyQuietRequest {
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
        let key = buf.freeze();
        if self.header.opcode == network::Command::Delete as u8 {
            Ok(Some(BinaryRequest::Delete(network::DeleteRequest {
                header: self.header,
                key,
            })))
        } else {
            Ok(Some(BinaryRequest::DeleteQuiet(network::DeleteRequest {
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
        if self.header.opcode == network::Command::Noop as u8 {
            Ok(Some(BinaryRequest::Noop(network::NoopRequest {
                header: self.header,
            })))
        } else if self.header.opcode == network::Command::Quit as u8 {
            Ok(Some(BinaryRequest::Quit(network::QuitRequest {
                header: self.header,
            })))
        } else if self.header.opcode == network::Command::QuitQuiet as u8 {
            Ok(Some(BinaryRequest::QuitQuietly(network::QuitRequest {
                header: self.header,
            })))
        } else {
            Ok(Some(BinaryRequest::Version(network::VersionRequest {
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
        if self.header.opcode == network::Command::Flush as u8 {
            Ok(Some(BinaryRequest::Flush(network::FlushRequest {
                header: self.header,
                expiration,
            })))
        } else {
            Ok(Some(BinaryRequest::FlushQuietly(network::FlushRequest {
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
        let append_request = network::AppendRequest {
            header: self.header,
            key: src.split_to(self.header.key_length as usize).freeze(),
            value: src.split_to(value_len).freeze(),
        };

        if self.header.opcode == network::Command::Append as u8 {
            Ok(Some(BinaryRequest::Append(append_request)))
        } else if self.header.opcode == network::Command::AppendQuiet as u8 {
            Ok(Some(BinaryRequest::AppendQuietly(append_request)))
        } else if self.header.opcode == network::Command::Prepend as u8 {
            Ok(Some(BinaryRequest::Prepend(append_request)))
        } else {
            Ok(Some(BinaryRequest::PrependQuietly(append_request)))
        }
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

        let request = network::IncrementRequest {
            header: self.header,
            delta: src.get_u64(),
            initial: src.get_u64(),
            expiration: src.get_u32(),
            key: src.split_to(self.header.key_length as usize).freeze(),
        };

        if self.header.opcode == network::Command::Increment as u8 {
            Ok(Some(BinaryRequest::Increment(request)))
        } else if self.header.opcode == network::Command::IncrementQuiet as u8 {
            Ok(Some(BinaryRequest::IncrementQuiet(request)))
        } else if self.header.opcode == network::Command::Decrement as u8 {
            Ok(Some(BinaryRequest::Decrement(request)))
        } else {
            Ok(Some(BinaryRequest::DecrementQuiet(request)))
        }
    }

    fn parse_item_too_large(
        &self,
        _src: &mut BytesMut,
    ) -> Result<Option<BinaryRequest>, io::Error> {
        let set_request = network::SetRequest {
            header: self.header,
            flags: 0,
            expiration: 0,
            key: BytesMut::new().freeze(),
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

        let set_request = network::SetRequest {
            header: self.header,
            flags: src.get_u32(),
            expiration: src.get_u32(),
            key: src.split_to(self.header.key_length as usize).freeze(),
            value: src.split_to(value_len).freeze(),
        };

        match FromPrimitive::from_u8(self.header.opcode) {
            Some(network::Command::Set) => Ok(Some(BinaryRequest::Set(set_request))),
            Some(network::Command::SetQuiet) => Ok(Some(BinaryRequest::SetQuietly(set_request))),
            Some(network::Command::Add) => Ok(Some(BinaryRequest::Add(set_request))),
            Some(network::Command::AddQuiet) => Ok(Some(BinaryRequest::AddQuietly(set_request))),
            Some(network::Command::Replace) => Ok(Some(BinaryRequest::Replace(set_request))),
            Some(network::Command::ReplaceQuiet) => {
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
        }
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

impl MemcacheBinaryDecoder {
    const HEADER_LEN: usize = 24;
}

impl Decoder for MemcacheBinaryDecoder {
    type Item = BinaryRequest;
    type Error = io::Error;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<BinaryRequest>, io::Error> {
        if self.state == RequestParserState::None {
            if src.len() < MemcacheBinaryDecoder::HEADER_LEN {
                return Ok(None);
            }
            let result = self.parse_header(src);
            result?
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

#[cfg(test)]
mod binary_decoder_tests;
