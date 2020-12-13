use crate::storage::store;
use crate::storage::error;
use crate::version::MEMCRS_VERSION;
use crate::protocol::{binary, binary_codec};
use std::sync::Arc;

const EXTRAS_LENGTH: u8 = 4;

impl Into<store::Meta> for binary::RequestHeader {
    fn into(self) -> store::Meta {
        store::Meta::new(self.cas, self.opaque, 0)
    }
}

fn storage_error_to_response(
    err: error::StorageError,
    response_header: &mut binary::ResponseHeader,
) -> binary_codec::BinaryResponse {
    let message = err.to_static_string();
    response_header.status = err as u16;
    response_header.body_length = message.len() as u32;
    binary_codec::BinaryResponse::Error(binary::ErrorResponse {
        header: *response_header,
        error: message,
    })
}

fn into_quiet(response: binary_codec::BinaryResponse) -> Option<binary_codec::BinaryResponse> {
    if let binary_codec::BinaryResponse::Error(response) = &response {
        if response.header.status == error::StorageError::NotFound as u16 {
            return None;
        }
    }
    Some(response)
}

pub struct BinaryHandler {
    storage: Arc<store::Storage>,
}

impl BinaryHandler {
    pub fn new(store: Arc<store::Storage>) -> BinaryHandler {
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
            binary_codec::BinaryRequest::Delete(delete_request) => {
                Some(self.delete(delete_request, &mut response_header))
            }
            binary_codec::BinaryRequest::DeleteQuiet(delete_request) => {
                into_quiet(self.delete(delete_request, &mut response_header))
            }
            binary_codec::BinaryRequest::Flush(flush_request) => {
                Some(self.flush(flush_request, &mut response_header))
            }
            binary_codec::BinaryRequest::Get(get_request)
            | binary_codec::BinaryRequest::GetKey(get_request) => {
                Some(self.get(get_request, &mut response_header))
            }
            binary_codec::BinaryRequest::GetQuietly(get_quiet_req)
            | binary_codec::BinaryRequest::GetKeyQuietly(get_quiet_req) => {
                into_quiet(self.get(get_quiet_req, &mut response_header))
            }
            binary_codec::BinaryRequest::Increment(inc_request) => {
                Some(self.increment(inc_request, &mut response_header))
            }
            binary_codec::BinaryRequest::IncrementQuiet(inc_request) => {
                into_quiet(self.increment(inc_request, &mut response_header))
            }
            binary_codec::BinaryRequest::Decrement(dec_request) => {
                Some(self.decrement(dec_request, &mut response_header))
            }
            binary_codec::BinaryRequest::DecrementQuiet(dec_request) => {
                into_quiet(self.decrement(dec_request, &mut response_header))
            }
            binary_codec::BinaryRequest::Noop(_noop_request) => {
                Some(binary_codec::BinaryResponse::Noop(binary::NoopResponse {
                    header: response_header,
                }))
            }
            binary_codec::BinaryRequest::Set(set_req) => {
                let response = self.set(set_req, &mut response_header);
                Some(response)
            }
            binary_codec::BinaryRequest::Add(req) | binary_codec::BinaryRequest::Replace(req) => {
                Some(self.add_replace(req, &mut response_header))
            }
            binary_codec::BinaryRequest::Append(append_req)
            | binary_codec::BinaryRequest::Prepend(append_req) => {
                let response = self.append_prepend(append_req, &mut response_header);
                Some(response)
            }
            binary_codec::BinaryRequest::Version(_version_request) => {
                response_header.body_length = MEMCRS_VERSION.len() as u32;
                Some(binary_codec::BinaryResponse::Version(
                    binary::VersionResponse {
                        header: response_header,
                        version: String::from(MEMCRS_VERSION),
                    },
                ))
            }
        }
    }

    fn add_replace(
        &self,
        request: binary::SetRequest,
        response_header: &mut binary::ResponseHeader,
    ) -> binary_codec::BinaryResponse {
        let record = store::Record::new(
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
            Ok(command_status) => {
                response_header.cas = command_status.cas;
                binary_codec::BinaryResponse::Set(binary::SetResponse {
                    header: *response_header,
                })
            }
            Err(err) => storage_error_to_response(err, response_header),
        }
    }

    fn is_add_command(&self, opcode: u8) -> bool {
        opcode == binary::Command::Add as u8 || opcode == binary::Command::AddQuiet as u8
    }

    fn append_prepend(
        &self,
        append_req: binary::AppendRequest,
        response_header: &mut binary::ResponseHeader,
    ) -> binary_codec::BinaryResponse {
        let record = store::Record::new(append_req.value, append_req.header.cas, 0, 0);
        let result = if self.is_append(append_req.header.opcode) {
            self.storage.append(append_req.key, record)
        } else {
            self.storage.prepend(append_req.key, record)
        };

        match result {
            Ok(status) => {
                response_header.cas = status.cas;
                binary_codec::BinaryResponse::Append(binary::AppendResponse {
                    header: *response_header,
                })
            }
            Err(err) => storage_error_to_response(err, response_header),
        }
    }

    fn is_append(&self, opcode: u8) -> bool {
        opcode == binary::Command::Append as u8 || opcode == binary::Command::AppendQuiet as u8
    }

    fn set(
        &self,
        set_req: binary::SetRequest,
        response_header: &mut binary::ResponseHeader,
    ) -> binary_codec::BinaryResponse {
        let record = store::Record::new(
            set_req.value,
            set_req.header.cas,
            set_req.flags,
            set_req.expiration,
        );

        match self.storage.set(set_req.key, record) {
            Ok(status) => {
                response_header.cas = status.cas;
                binary_codec::BinaryResponse::Set(binary::SetResponse {
                    header: *response_header,
                })
            }
            Err(err) => storage_error_to_response(err, response_header),
        }
    }

    fn delete(
        &self,
        delete_request: binary::DeleteRequest,
        response_header: &mut binary::ResponseHeader,
    ) -> binary_codec::BinaryResponse {
        let result = self
            .storage
            .delete(delete_request.key, delete_request.header.into());
        match result {
            Ok(_record) => binary_codec::BinaryResponse::Delete(binary::DeleteResponse {
                header: *response_header,
            }),
            Err(err) => storage_error_to_response(err, response_header),
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
            Err(err) => storage_error_to_response(err, response_header),
        }
    }

    fn is_get_key_command(&self, opcode: u8) -> bool {
        opcode == binary::Command::GetKey as u8 || opcode == binary::Command::GetKeyQuiet as u8
    }

    fn flush(
        &self,
        flush_request: binary::FlushRequest,
        response_header: &mut binary::ResponseHeader,
    ) -> binary_codec::BinaryResponse {
        let meta: store::Meta = store::Meta::new(0, 0, flush_request.expiration);
        self.storage.flush(meta);
        binary_codec::BinaryResponse::Flush(binary::FlushResponse {
            header: *response_header,
        })
    }

    fn increment(
        &self,
        inc_request: binary::IncrementRequest,
        response_header: &mut binary::ResponseHeader,
    ) -> binary_codec::BinaryResponse {
        let delta = store::IncrementParam {
            delta: inc_request.delta,
            value: inc_request.initial,
        };

        let result = self
            .storage
            .increment(inc_request.header.into(), inc_request.key, delta);
        match result {
            Ok(delta_result) => {
                binary_codec::BinaryResponse::Increment(binary::IncrementResponse {
                    header: *response_header,
                    value: delta_result.value,
                })
            }
            Err(err) => storage_error_to_response(err, response_header),
        }
    }

    fn decrement(
        &self,
        dec_request: binary::IncrementRequest,
        response_header: &mut binary::ResponseHeader,
    ) -> binary_codec::BinaryResponse {
        let delta = store::IncrementParam {
            delta: dec_request.delta,
            value: dec_request.initial,
        };

        let result = self
            .storage
            .decrement(dec_request.header.into(), dec_request.key, delta);
        match result {
            Ok(delta_result) => {
                binary_codec::BinaryResponse::Decrement(binary::DecrementResponse {
                    header: *response_header,
                    value: delta_result.value,
                })
            }
            Err(err) => storage_error_to_response(err, response_header),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::binary;
    use super::binary_codec;
    use super::*;
    use crate::storage::error;
    use crate::mock::mock_server::create_storage;
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
                    assert_eq!(response.header.body_length, response.error.len() as u32);
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
        let record = store::Record::new(value.clone(), 0, FLAGS, 0);

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
                if let binary_codec::BinaryResponse::Error(response) = resp {
                    assert_eq!(response.header.cas, 0);
                    check_header(
                        &response.header,
                        binary::Command::Set,
                        0,
                        0,
                        0,
                        error::StorageError::KeyExists as u16,
                        response.error.len() as u32,
                    );
                } else {
                    unreachable!();
                }
            }
            None => unreachable!(),
        }
    }
}
