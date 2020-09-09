use super::storage;
use crate::protocol::{binary, binary_codec};
use std::sync::Arc;

const EXTRAS_LENGTH: u8 = 4;

pub struct BinaryHandler {
    storage: Arc<storage::Storage>,
}

impl BinaryHandler {
    pub fn new(store: Arc<storage::Storage>) -> BinaryHandler {
        BinaryHandler { storage: store }
    }

    pub fn handle_request(
        &self,
        req: binary_codec::BinaryRequest,
    ) -> Option<binary_codec::BinaryResponse> {
        let request_header = req.get_header();
        let mut response_header =
            binary::ResponseHeader::new(request_header.opcode, request_header.opaque);

        match req {
            binary_codec::BinaryRequest::Get(get_request) => {
                let result = self.storage.get(&get_request.key);
                match result {
                    Ok(record) => {
                        response_header.body_length = record.value.len() as u32 + EXTRAS_LENGTH as u32;
                        response_header.extras_length = EXTRAS_LENGTH;
                        response_header.cas = record.header.cas;
                        Some(binary_codec::BinaryResponse::Get(binary::GetResponse {
                            header: response_header,
                            flags: record.header.flags,
                            key: Vec::new(),
                            value: record.value,
                        }))
                    }
                    Err(err) => {
                        let message = err.to_string();
                        response_header.status = err as u16;
                        Some(binary_codec::BinaryResponse::Error(binary::ErrorResponse {
                            header: response_header,
                            error: message,
                        }))
                    }
                }
            }
            binary_codec::BinaryRequest::GetQuietly(get_quiet_req) => None,
            binary_codec::BinaryRequest::GetKey(get_key_req) => None,
            binary_codec::BinaryRequest::GetKeyQuietly(get_key_quiet_req) => None,
            binary_codec::BinaryRequest::Set(mut set_req) => {
                let response = self.set(set_req, &mut response_header);
                Some(binary_codec::BinaryResponse::Set(response))
            }
            binary_codec::BinaryRequest::Add(add_req) => None,
            binary_codec::BinaryRequest::Replace(replace_req) => None,
        }
    }

    fn set(
        &self,
        set_req: binary::SetRequest,
        response_header: &mut binary::ResponseHeader,
    ) -> binary::SetResponse {
        let record = storage::Record::new(
            set_req.value,
            set_req.header.cas,
            set_req.flags,
            set_req.expiration,
        );
        match self.storage.set(set_req.key, record) {
            Ok(set_status) => response_header.cas = set_status.cas,
            Err(err) => response_header.status = err as u16,
        }
        binary::SetResponse {
            header: *response_header,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::binary;
    use super::binary_codec;
    use super::*;
    use crate::memcache::error;
    use crate::memcache::mock::mock_server::create_storage;

    fn create_handler() -> BinaryHandler {
        BinaryHandler::new(create_storage())
    }

    fn create_header(opcode: binary::Command, key: &[u8]) -> binary::RequestHeader {
        binary::RequestHeader {
            magic: binary::Magic::Request as u8,
            opcode: opcode as u8,
            key_length: key.len() as u16,
            extras_length: 0,
            data_type: 0,
            reserved: 0,
            body_length: 0,
            opaque: 0,
            cas: 0,
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn check_header(
        response: &binary::ResponseHeader,
        opcode: binary::Command,
        key_length: u16,
        extras_length: u8,
        data_type: u8,
        status: u16,
        body_length: u32,
        opaque: u32       
    ) {
        assert_eq!(response.magic, binary::Magic::Response as u8);
        assert_eq!(response.opcode, opcode as u8);
        assert_eq!(response.key_length, key_length);
        assert_eq!(response.extras_length, extras_length);
        assert_eq!(response.data_type, data_type);
        assert_eq!(response.status, status);
        assert_eq!(response.body_length, body_length);
        assert_eq!(response.opaque, opaque);
        assert_ne!(response.cas, 0);
    }

    #[test]
    fn get_request_should_return_not_found_when_not_exists() {
        let handler = create_handler();
        let key = String::from("key").into_bytes();
        let header = create_header(binary::Command::Get, &key);

        let request = binary_codec::BinaryRequest::Get(binary::GetRequest {
            header: header,
            key: key,
        });

        let result = handler.handle_request(request);
        match result {
            Some(resp) => {
                if let binary_codec::BinaryResponse::Error(response) = resp {
                    assert_eq!(response.header.status, error::StorageError::NotFound as u16);
                    assert_eq!(response.error, "Key not found");
                } else {
                    unreachable!();
                }
            }
            None => unreachable!(),
        }
    }

    #[test]
    fn get_request_should_return_record() {
        let handler = create_handler();
        let key = String::from("key").into_bytes();
        let header = create_header(binary::Command::Get, &key);
        const FLAGS: u32 = 0xDEAD_BEEF;
        let value = String::from("value").into_bytes();
        let record = storage::Record::new(value.clone(), 0, FLAGS, 0);

        let set_result = handler.storage.set(key.clone(), record);
        assert!(set_result.is_ok());

        let request = binary_codec::BinaryRequest::Get(binary::GetRequest { header, key });

        let result = handler.handle_request(request);
        match result {
            Some(resp) => {
                if let binary_codec::BinaryResponse::Get(response) = resp {
                    assert_eq!(response.flags, FLAGS);
                    assert_ne!(response.header.cas, 0);
                    check_header(&response.header, binary::Command::Get, 0, EXTRAS_LENGTH, 0, 0, value.len() as u32 + EXTRAS_LENGTH as u32, 0);
                } else {
                    unreachable!();
                }
            }
            None => unreachable!(),
        }
    }
}
