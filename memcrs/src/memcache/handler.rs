use super::storage;
use crate::memcache::error;
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
            binary_codec::BinaryRequest::Delete(_delete_request) 
            | binary_codec::BinaryRequest::DeleteQuiet(_delete_request) => {
                None
            },
            binary_codec::BinaryRequest::Get(get_request)
            | binary_codec::BinaryRequest::GetKey(get_request) => {
                Some(self.get(get_request, &mut response_header))
            }
            binary_codec::BinaryRequest::GetQuietly(get_quiet_req)
            | binary_codec::BinaryRequest::GetKeyQuietly(get_quiet_req) => {
                self.get_quiet(get_quiet_req, &mut response_header)
            }
            binary_codec::BinaryRequest::Increment(_incr_request) 
            | binary_codec::BinaryRequest::IncrementQuiet(_incr_request) 
            | binary_codec::BinaryRequest::Decrement(_incr_request) 
            | binary_codec::BinaryRequest::DecrementQuiet(_incr_request) => {
                None
            },
            binary_codec::BinaryRequest::Set(set_req) => {
                let response = self.set(set_req, &mut response_header);
                Some(binary_codec::BinaryResponse::Set(response))
            }
            binary_codec::BinaryRequest::Add(req) | binary_codec::BinaryRequest::Replace(req) => {
                Some(self.add_replace(req, &mut response_header))
            }
            binary_codec::BinaryRequest::Append(append_req)
            | binary_codec::BinaryRequest::Prepend(append_req) => {
                let response = self.append_prepend(append_req, &mut response_header);
                Some(response)
            }
        }
    }

    fn add_replace(
        &self,
        request: binary::SetRequest,
        response_header: &mut binary::ResponseHeader,
    ) -> binary_codec::BinaryResponse {
        let record = storage::Record::new(
            request.value,
            request.header.cas,
            request.flags,
            request.expiration,
        );
        let result = if self.is_add_command(request.header.opcode) {
            self.storage.add(request.key, record)
        } else {
            self.storage.replace(request.key, record)
        };

        match result {
            Ok(command_status) => response_header.cas = command_status.cas,
            Err(err) => response_header.status = err as u16,
        };

        binary_codec::BinaryResponse::Set(binary::SetResponse {
            header: *response_header,
        })
    }

    fn is_add_command(&self, opcode: u8) -> bool {
        opcode == binary::Command::Add as u8 || opcode == binary::Command::AddQuiet as u8
    }

    fn append_prepend(
        &self,
        append_req: binary::AppendRequest,
        response_header: &mut binary::ResponseHeader,
    ) -> binary_codec::BinaryResponse {
        let record = storage::Record::new(append_req.value, append_req.header.cas, 0, 0);
        let result = if self.is_append(append_req.header.opcode) {
            self.storage.append(append_req.key, record)
        } else {
            self.storage.prepend(append_req.key, record)
        };

        match result {
            Ok(command_status) => response_header.cas = command_status.cas,
            Err(err) => response_header.status = err as u16,
        }
        binary_codec::BinaryResponse::Append(binary::AppendResponse {
            header: *response_header,
        })
    }

    fn is_append(&self, opcode: u8) -> bool {
        opcode == binary::Command::Append as u8 || opcode == binary::Command::AppendQuiet as u8
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

    fn get(
        &self,
        get_request: binary::GetRequest,
        response_header: &mut binary::ResponseHeader,
    ) -> binary_codec::BinaryResponse {
        let result = self.storage.get(&get_request.key);

        match result {
            Ok(record) => {
                let include_key = self.is_get_key_command(get_request.header.opcode);
                let mut key: Vec<u8> = Vec::new();
                if include_key {
                    key = get_request.key
                }
                response_header.body_length =
                    record.value.len() as u32 + EXTRAS_LENGTH as u32 + key.len() as u32;
                response_header.key_length = key.len() as u16;
                response_header.extras_length = EXTRAS_LENGTH;
                response_header.cas = record.header.cas;
                binary_codec::BinaryResponse::Get(binary::GetResponse {
                    header: *response_header,
                    flags: record.header.flags,
                    key,
                    value: record.value,
                })
            }
            Err(err) => {
                let message = err.to_string();
                response_header.status = err as u16;
                binary_codec::BinaryResponse::Error(binary::ErrorResponse {
                    header: *response_header,
                    error: message,
                })
            }
        }
    }

    fn is_get_key_command(&self, opcode: u8) -> bool {
        opcode == binary::Command::GetKey as u8 || opcode == binary::Command::GetKeyQuiet as u8
    }

    fn get_quiet(
        &self,
        get_quiet_request: binary::GetRequest,
        response_header: &mut binary::ResponseHeader,
    ) -> Option<binary_codec::BinaryResponse> {
        let resp = self.get(get_quiet_request, response_header);
        if let binary_codec::BinaryResponse::Error(response) = &resp {
            if response.header.status == error::StorageError::NotFound as u16 {
                return None;
            }
        }
        Some(resp)
    }
}

#[cfg(test)]
mod tests {
    use super::binary;
    use super::binary_codec;
    use super::*;
    use crate::memcache::error;
    use crate::memcache::mock::mock_server::create_storage;
    const OPAQUE_VALUE: u32 = 0xABAD_CAFE;

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
            vbucket_id: 0,
            body_length: 0,
            opaque: OPAQUE_VALUE,
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
    ) {
        assert_eq!(response.magic, binary::Magic::Response as u8);
        assert_eq!(response.opcode, opcode as u8);
        assert_eq!(response.key_length, key_length);
        assert_eq!(response.extras_length, extras_length);
        assert_eq!(response.data_type, data_type);
        assert_eq!(response.status, status);
        assert_eq!(response.body_length, body_length);
        assert_eq!(response.opaque, OPAQUE_VALUE);
    }

    #[test]
    fn get_request_should_return_not_found_when_not_exists() {
        let handler = create_handler();
        let key = String::from("key").into_bytes();
        let header = create_header(binary::Command::Get, &key);

        let request = binary_codec::BinaryRequest::Get(binary::GetRequest { header, key });

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
                    check_header(
                        &response.header,
                        binary::Command::Get,
                        0,
                        EXTRAS_LENGTH,
                        0,
                        0,
                        value.len() as u32 + EXTRAS_LENGTH as u32,
                    );
                } else {
                    unreachable!();
                }
            }
            None => unreachable!(),
        }
    }

    #[test]
    fn set_request_should_succeed() {
        let handler = create_handler();
        let key = String::from("key").into_bytes();
        let header = create_header(binary::Command::Set, &key);
        const FLAGS: u32 = 0xDEAD_BEEF;
        let value = String::from("value").into_bytes();
        let request = binary_codec::BinaryRequest::Set(binary::SetRequest {
            header,
            flags: FLAGS,
            expiration: 0,
            key,
            value,
        });
        let result = handler.handle_request(request);
        match result {
            Some(resp) => {
                if let binary_codec::BinaryResponse::Set(response) = resp {
                    assert_ne!(response.header.cas, 0);
                    check_header(&response.header, binary::Command::Set, 0, 0, 0, 0, 0);
                } else {
                    unreachable!();
                }
            }
            None => unreachable!(),
        }
    }

    #[test]
    fn set_request_on_cas_mismatch_should_return_key_exists() {
        let handler = create_handler();
        let key = String::from("key").into_bytes();
        let mut header = create_header(binary::Command::Set, &key);
        const FLAGS: u32 = 0xDEAD_BEEF;
        let value = String::from("value").into_bytes();
        let request = binary_codec::BinaryRequest::Set(binary::SetRequest {
            header,
            flags: FLAGS,
            expiration: 0,
            key: key.clone(),
            value: value.clone(),
        });

        let result = handler.handle_request(request);
        match result {
            Some(resp) => {
                if let binary_codec::BinaryResponse::Set(response) = resp {
                    assert_ne!(response.header.cas, 0);
                    check_header(&response.header, binary::Command::Set, 0, 0, 0, 0, 0);
                } else {
                    unreachable!();
                }
            }
            None => unreachable!(),
        }
        header.cas = 100;
        let request = binary_codec::BinaryRequest::Set(binary::SetRequest {
            header,
            flags: FLAGS,
            expiration: 0,
            key,
            value,
        });

        let result = handler.handle_request(request);
        match result {
            Some(resp) => {
                if let binary_codec::BinaryResponse::Set(response) = resp {
                    assert_eq!(response.header.cas, 0);
                    check_header(
                        &response.header,
                        binary::Command::Set,
                        0,
                        0,
                        0,
                        error::StorageError::KeyExists as u16,
                        0,
                    );
                } else {
                    unreachable!();
                }
            }
            None => unreachable!(),
        }
    }
}
