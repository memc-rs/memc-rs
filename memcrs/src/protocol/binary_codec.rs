use std::io;

use crate::protocol::binary;
use bytes::{Buf, BufMut, BytesMut};
use num_traits::FromPrimitive;
use serde_derive::{Deserialize, Serialize};
use std::io::{Error, ErrorKind};
use tokio_util::codec::{Decoder, Encoder};

/// Client request
#[derive(Serialize, Deserialize, Debug)]
pub enum BinaryRequest {
    Get(binary::GetRequest),
    GetQuietly(binary::GetQuietRequest),
    GetKey(binary::GetKeyRequest),
    GetKeyQuietly(binary::GetKeyQuietRequest),
    Set(binary::SetRequest),
    Append(binary::AppendRequest),
    Prepend(binary::PrependRequest),
    Add(binary::AddRequest),
    Replace(binary::ReplaceRequest),
}

impl BinaryRequest {
    pub fn get_header(&'_ self) -> &'_ binary::RequestHeader {
        match self {
            BinaryRequest::Get(request) => &request.header,
            BinaryRequest::GetKey(request) => &request.header,
            BinaryRequest::GetKeyQuietly(request) => &request.header,
            BinaryRequest::GetQuietly(request) => &request.header,
            BinaryRequest::Set(request) => &request.header,
            BinaryRequest::Replace(request) => &request.header,
            BinaryRequest::Add(request) => &request.header,
            BinaryRequest::Prepend(request) => &request.header,
            BinaryRequest::Append(request) => &request.header,
        }
    }
}

/// Server response
#[derive(Serialize, Deserialize, Debug)]
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
        }
    }
}

#[derive(PartialEq, Debug)]
enum RequestParserState {
    None,
    HeaderParsed,
}

pub struct MemcacheBinaryCodec {
    header: binary::RequestHeader,
    state: RequestParserState,
}

impl Default for MemcacheBinaryCodec {
    fn default() -> Self {
        Self::new()
    }
}

impl MemcacheBinaryCodec {
    pub fn new() -> MemcacheBinaryCodec {
        MemcacheBinaryCodec {
            header: Default::default(),
            state: RequestParserState::None,
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
            | Some(binary::Command::PrependQuiet)
            | Some(binary::Command::Prepend) => Ok(None),

            Some(binary::Command::Set)
            | Some(binary::Command::SetQuiet)
            | Some(binary::Command::Add)
            | Some(binary::Command::Replace)
            | Some(binary::Command::AddQuiet)
            | Some(binary::Command::ReplaceQuiet) => self.parse_set_request(src),

            Some(binary::Command::Delete) | Some(binary::Command::DeleteQuiet) => Ok(None),

            Some(binary::Command::Increment)
            | Some(binary::Command::Decrement)
            | Some(binary::Command::IncrementQuiet)
            | Some(binary::Command::DecrementQuiet) => Ok(None),

            Some(binary::Command::Quit) | Some(binary::Command::QuitQuiet) => Ok(None),

            Some(binary::Command::Noop) => Ok(None),
            Some(binary::Command::Version) => Ok(None),
            Some(binary::Command::Stat) => Ok(None),

            Some(binary::Command::Flush) | Some(binary::Command::FlushQuiet) => Ok(None),

            Some(binary::Command::Touch) => Ok(None),
            Some(binary::Command::GetAndTouch) => Ok(None),
            Some(binary::Command::GetAndTouchQuiet) => Ok(None),
            Some(binary::Command::GetAndTouchKey) => Ok(None),
            Some(binary::Command::GetAndTouchKeyQuiet) => Ok(None),

            Some(binary::Command::SaslAuth) => Ok(None),
            Some(binary::Command::SaslListMechs) => Ok(None),
            Some(binary::Command::SaslStep) => Ok(None),

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
        (self.header.body_length as usize) - ((self.header.key_length + 8) as usize)
    }

    fn parse_get_request(&self, src: &mut BytesMut) -> Result<Option<BinaryRequest>, io::Error> {
        if !self.request_valid(src) {
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
        } else {
            Ok(Some(BinaryRequest::Get(binary::GetQuietRequest {
                header: self.header,
                key,
            })))
        }
    }

    fn parse_set_request(&self, src: &mut BytesMut) -> Result<Option<BinaryRequest>, io::Error> {
        if !self.request_valid(src) {
            return Err(Error::new(ErrorKind::InvalidData, "Incorrect set request"));
        }

        let value_len = self.get_value_len();

        let set_request = binary::SetRequest {
            header: self.header,
            flags: src.get_u32(),
            expiration: src.get_u32(),
            key: src.split_to(self.header.key_length as usize).to_vec(),
            value: src.split_to(value_len as usize).to_vec(),
        };

        if self.header.opcode == binary::Command::Replace as u8
            || self.header.opcode == binary::Command::ReplaceQuiet as u8
        {
            Ok(Some(BinaryRequest::Replace(set_request)))
        } else if self.header.opcode == binary::Command::Add as u8
            || self.header.opcode == binary::Command::AddQuiet as u8
        {
            Ok(Some(BinaryRequest::Add(set_request)))
        } else {
            Ok(Some(BinaryRequest::Set(set_request)))
        }
    }

    fn request_valid(&self, src: &mut BytesMut) -> bool {
        if self.header.extras_length > 12 {
            return false;
        }

        if self.header.key_length == 0 || self.header.key_length > 250 {
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

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
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
        if (self.header.body_length as usize) > src.len() {
            return Ok(None);
        }
        self.parse_request(src)
    }
}

impl MemcacheBinaryCodec {
    const RESPONSE_HEADER_LEN: usize = 24;

    fn get_length(&self, msg: &BinaryResponse) -> usize {
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

    fn write_msg(&self, msg: &BinaryResponse, dst: &mut BytesMut) {
        self.write_header(self.get_header(msg), dst);
        self.write_data(msg, dst)
    }

    fn write_header(&self, header: &binary::ResponseHeader, dst: &mut BytesMut) {
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
            BinaryResponse::Get(response) => {
                dst.put_u32(response.flags);
                dst.put_slice(&response.key[..]);
                dst.put_slice(&response.value[..]);
            }
            BinaryResponse::GetKey(response) => {
                dst.put_u32(response.flags);
                dst.put_slice(&response.key[..]);
            }
            BinaryResponse::GetKeyQuietly(response) => {
                dst.put_u32(response.flags);
                dst.put_slice(&response.key[..]);
            }
            BinaryResponse::GetQuietly(response) => {
                dst.put_u32(response.flags);
                dst.put_slice(&response.key[..]);
                dst.put_slice(&response.value[..]);
            }
            BinaryResponse::Set(response)
            | BinaryResponse::Replace(response)
            | BinaryResponse::Add(response)
            | BinaryResponse::Append(response)
            | BinaryResponse::Prepend(response) => dst.put_u64(response.header.cas),
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
mod tests {

    use super::*;

    fn decode_packet(src: &[u8]) -> Result<Option<BinaryRequest>, io::Error> {
        let mut decoder = MemcacheBinaryCodec::new();
        let mut buf = BytesMut::with_capacity(64);
        buf.put_slice(&src);
        decoder.decode(&mut buf)
    }

    #[test]
    fn decode_set_request() {
        let set_request_packet: [u8; 39] = [
            0x80, // magic
            0x01, // opcode
            0x00, 0x03, // key length
            0x08, // extras length
            0x00, // data type
            0x00, 0x00, // vbucket id
            0x00, 0x00, 0x00, 0x0f, // total body length
            0xDE, 0xAD, 0xBE, 0xEF, // opaque
            0x00, 0x00, 0x00, 0x00, // cas
            0x00, 0x00, 0x00, 0x01, // cas
            0xAB, 0xAD, 0xCA, 0xFE, // flags
            0x00, 0x00, 0x00, 0x32, // expiration
            0x66, 0x6f, 0x6f, // key 'foo'
            0x74, 0x65, 0x73, 0x74, // value 'test'
        ];
        let decode_result = decode_packet(&set_request_packet);
        match decode_result {
            Ok(set_request) => {
                assert!(set_request.is_some());
                if let Some(request) = set_request {
                    let header = request.get_header();
                    assert_eq!(header.magic, binary::Magic::Request as u8);
                    assert_eq!(header.opcode, binary::Command::Set as u8);
                    assert_eq!(header.key_length, 0x03);
                    assert_eq!(header.extras_length, 0x08);
                    assert_eq!(header.data_type, binary::DataTypes::RawBytes as u8);
                    assert_eq!(header.vbucket_id, 0x00);
                    assert_eq!(header.body_length, 0x0f);
                    assert_eq!(header.opaque, 0xDEADBEEF);
                    assert_eq!(header.cas, 0x01);
                    //
                    match request {
                        BinaryRequest::Set(req) => {
                            assert_eq!(req.flags, 0xabadcafe);
                            assert_eq!(req.expiration, 0x32);
                            assert_eq!(req.key, [b'f', b'o', b'o']);
                            assert_eq!(req.value, [b't', b'e', b's', b't']);
                        }
                        _ => unreachable!(),
                    }
                }
            }
            Err(_) => unreachable!(),
        }
    }

    #[test]
    fn decode_replace_request() {
        let set_request_packet: [u8; 39] = [
            0x80, // magic
            0x03, // opcode
            0x00, 0x03, // key length
            0x08, // extras length
            0x00, // data type
            0x00, 0x00, // vbucket id
            0x00, 0x00, 0x00, 0x0f, // total body length
            0xDE, 0xAD, 0xBE, 0xEF, // opaque
            0x00, 0x00, 0x00, 0x00, // cas
            0x00, 0x00, 0x00, 0x01, // cas
            0xAB, 0xAD, 0xCA, 0xFE, // flags
            0x00, 0x00, 0x00, 0x32, // expiration
            0x66, 0x6f, 0x6f, // key 'foo'
            0x74, 0x65, 0x73, 0x74, // value 'test'
        ];
        let decode_result = decode_packet(&set_request_packet);
        match decode_result {
            Ok(set_request) => {
                assert!(set_request.is_some());
                if let Some(request) = set_request {
                    let header = request.get_header();
                    assert_eq!(header.magic, binary::Magic::Request as u8);
                    assert_eq!(header.opcode, binary::Command::Replace as u8);

                    match request {
                        BinaryRequest::Replace(req) => {
                            assert_eq!(req.flags, 0xabadcafe);
                            assert_eq!(req.expiration, 0x32);
                            assert_eq!(req.key, [b'f', b'o', b'o']);
                            assert_eq!(req.value, [b't', b'e', b's', b't']);
                        }
                        _ => unreachable!(),
                    }
                }
            }
            Err(_) => unreachable!(),
        }
    }

    #[test]
    fn decode_add_request() {
        let set_request_packet: [u8; 39] = [
            0x80, // magic
            0x02, // opcode
            0x00, 0x03, // key length
            0x08, // extras length
            0x00, // data type
            0x00, 0x00, // vbucket id
            0x00, 0x00, 0x00, 0x0f, // total body length
            0xDE, 0xAD, 0xBE, 0xEF, // opaque
            0x00, 0x00, 0x00, 0x00, // cas
            0x00, 0x00, 0x00, 0x01, // cas
            0xAB, 0xAD, 0xCA, 0xFE, // flags
            0x00, 0x00, 0x00, 0x32, // expiration
            0x66, 0x6f, 0x6f, // key 'foo'
            0x74, 0x65, 0x73, 0x74, // value 'test'
        ];
        let decode_result = decode_packet(&set_request_packet);
        match decode_result {
            Ok(set_request) => {
                assert!(set_request.is_some());
                if let Some(request) = set_request {
                    let header = request.get_header();
                    assert_eq!(header.magic, binary::Magic::Request as u8);
                    assert_eq!(header.opcode, binary::Command::Add as u8);

                    match request {
                        BinaryRequest::Add(req) => {
                            assert_eq!(req.flags, 0xabadcafe);
                            assert_eq!(req.expiration, 0x32);
                            assert_eq!(req.key, [b'f', b'o', b'o']);
                            assert_eq!(req.value, [b't', b'e', b's', b't']);
                        }
                        _ => unreachable!(),
                    }
                }
            }
            Err(_) => unreachable!(),
        }
    }

    #[test]
    fn decode_get_request() {
        let get_request_packet: [u8; 27] = [
            0x80, // magic
            0x00, // opcode
            0x00, 0x03, // key len
            0x00, // extras length
            0x00, // data type
            0x00, 0x00, // vbucket id
            0x00, 0x00, 0x00, 0x03, // total body length
            0x00, 0x00, 0x00, 0x00, // opaque
            0x00, 0x00, 0x00, 0x00, // cas
            0x00, 0x00, 0x00, 0x00, // cas
            0x66, 0x6f, 0x6f, // key 'foo'
        ];

        let decode_result = decode_packet(&get_request_packet);
        match decode_result {
            Ok(set_request) => {
                assert!(set_request.is_some());
                if let Some(request) = set_request {
                    let header = request.get_header();
                    assert_eq!(header.magic, binary::Magic::Request as u8);
                    assert_eq!(header.opcode, binary::Command::Get as u8);
                    assert_eq!(header.key_length, 0x03);
                    assert_eq!(header.extras_length, 0x00);
                    assert_eq!(header.data_type, binary::DataTypes::RawBytes as u8);
                    assert_eq!(header.vbucket_id, 0x00);
                    assert_eq!(header.body_length, 0x03);
                    assert_eq!(header.opaque, 0x00000000);
                    assert_eq!(header.cas, 0x00);
                    //
                    match request {
                        BinaryRequest::Get(req) => {
                            assert_eq!(req.key, [b'f', b'o', b'o']);
                        }
                        _ => unreachable!(),
                    }
                }
            }
            Err(_) => unreachable!(),
        }
    }
    #[test]
    fn decode_get_quiet_request() {
        let get_request_packet: [u8; 27] = [
            0x80, // magic
            0x09, // opcode
            0x00, 0x03, // key len
            0x00, // extras length
            0x00, // data type
            0x00, 0x00, // vbucket id
            0x00, 0x00, 0x00, 0x03, // total body length
            0x00, 0x00, 0x00, 0x00, // opaque
            0x00, 0x00, 0x00, 0x00, // cas
            0x00, 0x00, 0x00, 0x00, // cas
            0x66, 0x6f, 0x6f, // key 'foo'
        ];

        let decode_result = decode_packet(&get_request_packet);
        match decode_result {
            Ok(set_request) => {
                assert!(set_request.is_some());
                if let Some(request) = set_request {
                    let header = request.get_header();
                    assert_eq!(header.magic, binary::Magic::Request as u8);
                    assert_eq!(header.opcode, binary::Command::GetQuiet as u8);
                    assert_eq!(header.key_length, 0x03);
                    assert_eq!(header.extras_length, 0x00);
                    assert_eq!(header.data_type, binary::DataTypes::RawBytes as u8);
                    assert_eq!(header.vbucket_id, 0x00);
                    assert_eq!(header.body_length, 0x03);
                    assert_eq!(header.opaque, 0x00000000);
                    assert_eq!(header.cas, 0x00);
                    //
                    match request {
                        BinaryRequest::Get(req) => {
                            assert_eq!(req.key, [b'f', b'o', b'o']);
                        }
                        _ => unreachable!(),
                    }
                }
            }
            Err(_) => unreachable!(),
        }
    }

    #[test]
    fn decode_get_key_request() {
        let get_request_packet: [u8; 27] = [
            0x80, // magic
            0x0c, // opcode
            0x00, 0x03, // key len
            0x00, // extras length
            0x00, // data type
            0x00, 0x00, // vbucket id
            0x00, 0x00, 0x00, 0x03, // total body length
            0x00, 0x00, 0x00, 0x00, // opaque
            0x00, 0x00, 0x00, 0x00, // cas
            0x00, 0x00, 0x00, 0x00, // cas
            0x66, 0x6f, 0x6f, // key 'foo'
        ];

        let decode_result = decode_packet(&get_request_packet);
        match decode_result {
            Ok(set_request) => {
                assert!(set_request.is_some());
                if let Some(request) = set_request {
                    let header = request.get_header();
                    assert_eq!(header.magic, binary::Magic::Request as u8);
                    assert_eq!(header.opcode, binary::Command::GetKey as u8);
                    //
                    match request {
                        BinaryRequest::Get(req) => {
                            assert_eq!(req.key, [b'f', b'o', b'o']);
                        }
                        _ => unreachable!(),
                    }
                }
            }
            Err(_) => unreachable!(),
        }
    }

    #[test]
    fn decode_get_key_quiet_request() {
        let get_request_packet: [u8; 27] = [
            0x80, // magic
            0x0D, // opcode
            0x00, 0x03, // key len
            0x00, // extras length
            0x00, // data type
            0x00, 0x00, // vbucket id
            0x00, 0x00, 0x00, 0x03, // total body length
            0x00, 0x00, 0x00, 0x00, // opaque
            0x00, 0x00, 0x00, 0x00, // cas
            0x00, 0x00, 0x00, 0x00, // cas
            0x66, 0x6f, 0x6f, // key 'foo'
        ];

        let decode_result = decode_packet(&get_request_packet);
        match decode_result {
            Ok(set_request) => {
                assert!(set_request.is_some());
                if let Some(request) = set_request {
                    let header = request.get_header();
                    assert_eq!(header.magic, binary::Magic::Request as u8);
                    assert_eq!(header.opcode, binary::Command::GetKeyQuiet as u8);
                    //
                    match request {
                        BinaryRequest::Get(req) => {
                            assert_eq!(req.key, [b'f', b'o', b'o']);
                        }
                        _ => unreachable!(),
                    }
                }
            }
            Err(_) => unreachable!(),
        }
    }

    #[test]
    fn decode_if_buffer_doesnt_contain_full_header_none_should_be_returned() {
        let set_request_packet: [u8; 4] = [
            0x80, // magic
            0x01, // opcode
            0x00, 0x03, // key length
        ];
        let decode_result = decode_packet(&set_request_packet);
        match decode_result {
            Ok(set_request) => {
                assert!(set_request.is_none());
            }
            Err(_) => unreachable!(),
        }
    }

    #[test]
    fn decode_if_buffer_doesnt_contain_full_packet_none_should_be_returned() {
        let set_request_packet: [u8; 24] = [
            0x80, // magic
            0x01, // opcode
            0x00, 0x03, // key length
            0x08, // extras length
            0x00, // data type
            0x00, 0x00, // vbucket id
            0x00, 0x00, 0x00, 0x0f, // total body length
            0xDE, 0xAD, 0xBE, 0xEF, // opaque
            0x00, 0x00, 0x00, 0x00, // cas
            0x00, 0x00, 0x00, 0x01, // cas
        ];
        let decode_result = decode_packet(&set_request_packet);
        match decode_result {
            Ok(set_request) => {
                assert!(set_request.is_none());
            }
            Err(_) => unreachable!(),
        }
    }

    #[test]
    fn decode_check_if_error_on_incorrect_magic() {
        let set_request_packet: [u8; 24] = [
            0x81, // magic
            0x01, // opcode
            0x00, 0x03, // key length
            0x08, // extras length
            0x00, // data type
            0x00, 0x00, // vbucket id
            0x00, 0x00, 0x00, 0x0f, // total body length
            0xDE, 0xAD, 0xBE, 0xEF, // opaque
            0x00, 0x00, 0x00, 0x00, // cas
            0x00, 0x00, 0x00, 0x01, // cas
        ];
        let decode_result = decode_packet(&set_request_packet);
        match decode_result {
            Ok(_) => unreachable!(),
            Err(err) => {
                assert_eq!(err.kind(), io::ErrorKind::InvalidData);
            }
        }
    }

    #[test]
    fn decode_if_key_length_too_large_error_should_be_returned() {
        let get_request_packet: [u8; 27] = [
            0x80, // magic
            0x00, // opcode
            0xff, 0xff, // key len
            0x00, // extras length
            0x00, // data type
            0x00, 0x00, // vbucket id
            0x00, 0x00, 0x00, 0x03, // total body length
            0x00, 0x00, 0x00, 0x00, // opaque
            0x00, 0x00, 0x00, 0x00, // cas
            0x00, 0x00, 0x00, 0x00, // cas
            0x66, 0x6f, 0x6f, // key 'foo'
        ];
        let decode_result = decode_packet(&get_request_packet);
        match decode_result {
            Ok(_) => unreachable!(),
            Err(err) => {
                assert_eq!(err.kind(), io::ErrorKind::InvalidData);
            }
        }
    }

    #[test]
    fn decode_if_extras_length_too_large_error_should_be_returned() {
        let set_request_packet: [u8; 39] = [
            0x80, // magic
            0x01, // opcode
            0x00, 0x03, // key length
            0x0D, // extras length
            0x00, // data type
            0x00, 0x00, // vbucket id
            0x00, 0x00, 0x00, 0x0f, // total body length
            0xDE, 0xAD, 0xBE, 0xEF, // opaque
            0x00, 0x00, 0x00, 0x00, // cas
            0x00, 0x00, 0x00, 0x01, // cas
            0xAB, 0xAD, 0xCA, 0xFE, // flags
            0x00, 0x00, 0x00, 0x32, // expiration
            0x66, 0x6f, 0x6f, // key 'foo'
            0x74, 0x65, 0x73, 0x74, // value 'test'
        ];

        let decode_result = decode_packet(&set_request_packet);
        match decode_result {
            Ok(_) => unreachable!(),
            Err(err) => {
                assert_eq!(err.kind(), io::ErrorKind::InvalidData);
            }
        }
    }

    #[test]
    fn decode_body_length_should_be_greater_than_key_len_and_extras_len() {
        let set_request_packet: [u8; 39] = [
            0x80, // magic
            0x01, // opcode
            0x00, 0x03, // key length
            0x08, // extras length
            0x00, // data type
            0x00, 0x00, // vbucket id
            0x00, 0x00, 0x00, 0x0A, // total body length
            0xDE, 0xAD, 0xBE, 0xEF, // opaque
            0x00, 0x00, 0x00, 0x00, // cas
            0x00, 0x00, 0x00, 0x01, // cas
            0xAB, 0xAD, 0xCA, 0xFE, // flags
            0x00, 0x00, 0x00, 0x32, // expiration
            0x66, 0x6f, 0x6f, // key 'foo'
            0x74, 0x65, 0x73, 0x74, // value 'test'
        ];

        let decode_result = decode_packet(&set_request_packet);
        match decode_result {
            Ok(_) => unreachable!(),
            Err(err) => {
                assert_eq!(err.kind(), io::ErrorKind::InvalidData);
            }
        }
    }

    #[test]
    fn decode_if_opcode_is_greater_than_opcode_max_error_should_be_returned() {
        let set_request_packet: [u8; 39] = [
            0x80, // magic
            0x25, // opcode
            0x00, 0x03, // key length
            0x08, // extras length
            0x00, // data type
            0x00, 0x00, // vbucket id
            0x00, 0x00, 0x00, 0x0A, // total body length
            0xDE, 0xAD, 0xBE, 0xEF, // opaque
            0x00, 0x00, 0x00, 0x00, // cas
            0x00, 0x00, 0x00, 0x01, // cas
            0xAB, 0xAD, 0xCA, 0xFE, // flags
            0x00, 0x00, 0x00, 0x32, // expiration
            0x66, 0x6f, 0x6f, // key 'foo'
            0x74, 0x65, 0x73, 0x74, // value 'test'
        ];

        let decode_result = decode_packet(&set_request_packet);
        match decode_result {
            Ok(_) => unreachable!(),
            Err(err) => {
                assert_eq!(err.kind(), io::ErrorKind::InvalidData);
            }
        }
    }

    #[test]
    fn decode_data_type_should_be_0() {
        let set_request_packet: [u8; 39] = [
            0x80, // magic
            0x01, // opcode
            0x00, 0x03, // key length
            0x08, // extras length
            0xff, // data type
            0x00, 0x00, // vbucket id
            0x00, 0x00, 0x00, 0x0A, // total body length
            0xDE, 0xAD, 0xBE, 0xEF, // opaque
            0x00, 0x00, 0x00, 0x00, // cas
            0x00, 0x00, 0x00, 0x01, // cas
            0xAB, 0xAD, 0xCA, 0xFE, // flags
            0x00, 0x00, 0x00, 0x32, // expiration
            0x66, 0x6f, 0x6f, // key 'foo'
            0x74, 0x65, 0x73, 0x74, // value 'test'
        ];

        let decode_result = decode_packet(&set_request_packet);
        match decode_result {
            Ok(_) => unreachable!(),
            Err(err) => {
                assert_eq!(err.kind(), io::ErrorKind::InvalidData);
            }
        }
    }
}
