use crate::cache::error::CacheError;
use crate::memcache::store;
use crate::protocol::binary::encoder::storage_error_to_response;
use crate::protocol::binary::{binary, decoder, encoder};
use crate::version::MEMCRS_VERSION;
use bytes::Bytes;
use std::sync::Arc;

const EXTRAS_LENGTH: u8 = 4;

fn into_record_meta(request_header: &binary::RequestHeader, expiration: u32) -> store::Meta {
    store::Meta::new(request_header.cas, request_header.opaque, expiration)
}

fn into_quiet_get(response: encoder::BinaryResponse) -> Option<encoder::BinaryResponse> {
    if let encoder::BinaryResponse::Error(response) = &response {
        if response.header.status == CacheError::NotFound as u16 {
            return None;
        }
    }
    Some(response)
}

fn into_quiet_mutation(
    response: encoder::BinaryResponse,
) -> Option<encoder::BinaryResponse> {
    if let encoder::BinaryResponse::Error(_resp) = &response {
        return Some(response);
    }
    None
}

pub struct BinaryHandler {
    storage: Arc<store::MemcStore>,
}

impl BinaryHandler {
    pub fn new(store: Arc<store::MemcStore>) -> BinaryHandler {
        BinaryHandler { storage: store }
    }

    pub fn handle_request(
        &self,
        req: decoder::BinaryRequest,
    ) -> Option<encoder::BinaryResponse> {
        let request_header = req.get_header();
        let mut response_header =
            binary::ResponseHeader::new(request_header.opcode, request_header.opaque);

        match req {
            decoder::BinaryRequest::Delete(delete_request) => {
                Some(self.delete(delete_request, &mut response_header))
            }
            decoder::BinaryRequest::DeleteQuiet(delete_request) => {
                into_quiet_mutation(self.delete(delete_request, &mut response_header))
            }
            decoder::BinaryRequest::Flush(flush_request) => {
                Some(self.flush(flush_request, &mut response_header))
            }
            decoder::BinaryRequest::FlushQuietly(flush_request) => {
                into_quiet_mutation(self.flush(flush_request, &mut response_header))
            }
            decoder::BinaryRequest::Get(get_request)
            | decoder::BinaryRequest::GetKey(get_request) => {
                Some(self.get(get_request, &mut response_header))
            }
            decoder::BinaryRequest::GetQuietly(get_quiet_req)
            | decoder::BinaryRequest::GetKeyQuietly(get_quiet_req) => {
                into_quiet_get(self.get(get_quiet_req, &mut response_header))
            }
            decoder::BinaryRequest::Increment(inc_request) => {
                Some(self.increment(inc_request, &mut response_header))
            }
            decoder::BinaryRequest::IncrementQuiet(inc_request) => {
                into_quiet_mutation(self.increment(inc_request, &mut response_header))
            }
            decoder::BinaryRequest::Decrement(dec_request) => {
                Some(self.decrement(dec_request, &mut response_header))
            }
            decoder::BinaryRequest::DecrementQuiet(dec_request) => {
                into_quiet_mutation(self.decrement(dec_request, &mut response_header))
            }
            decoder::BinaryRequest::Noop(_noop_request) => {
                Some(encoder::BinaryResponse::Noop(binary::NoopResponse {
                    header: response_header,
                }))
            }
            decoder::BinaryRequest::Stats(_stat_request) => {
                Some(encoder::BinaryResponse::Stats(binary::StatsResponse {
                    header: response_header,
                }))
            }
            decoder::BinaryRequest::Quit(_quit_req) => {
                Some(encoder::BinaryResponse::Quit(binary::QuitResponse {
                    header: response_header,
                }))
            }
            decoder::BinaryRequest::QuitQuietly(_quit_req) => {
                into_quiet_mutation(encoder::BinaryResponse::Quit(binary::QuitResponse {
                    header: response_header,
                }))
            }
            decoder::BinaryRequest::Set(set_req) => {
                let response = self.set(set_req, &mut response_header);
                Some(response)
            }
            decoder::BinaryRequest::SetQuietly(set_req) => {
                let response = self.set(set_req, &mut response_header);
                into_quiet_mutation(response)
            }
            decoder::BinaryRequest::Add(req) | decoder::BinaryRequest::Replace(req) => {
                Some(self.add_replace(req, &mut response_header))
            }
            decoder::BinaryRequest::AddQuietly(req)
            | decoder::BinaryRequest::ReplaceQuietly(req) => {
                into_quiet_mutation(self.add_replace(req, &mut response_header))
            }
            decoder::BinaryRequest::Append(append_req)
            | decoder::BinaryRequest::Prepend(append_req) => {
                let response = self.append_prepend(append_req, &mut response_header);
                Some(response)
            }
            decoder::BinaryRequest::AppendQuietly(append_req)
            | decoder::BinaryRequest::PrependQuietly(append_req) => {
                let response = self.append_prepend(append_req, &mut response_header);
                into_quiet_mutation(response)
            }
            decoder::BinaryRequest::Version(_version_request) => {
                response_header.body_length = MEMCRS_VERSION.len() as u32;
                Some(encoder::BinaryResponse::Version(
                    binary::VersionResponse {
                        header: response_header,
                        version: String::from(MEMCRS_VERSION),
                    },
                ))
            }
            decoder::BinaryRequest::ItemTooLarge(_set_request) => Some(
                storage_error_to_response(CacheError::ValueTooLarge, &mut response_header),
            ),
        }
    }

    fn add_replace(
        &self,
        request: binary::SetRequest,
        response_header: &mut binary::ResponseHeader,
    ) -> encoder::BinaryResponse {
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
                encoder::BinaryResponse::Set(binary::SetResponse {
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
    ) -> encoder::BinaryResponse {
        let record = store::Record::new(append_req.value, append_req.header.cas, 0, 0);
        let result = if self.is_append(append_req.header.opcode) {
            self.storage.append(append_req.key, record)
        } else {
            self.storage.prepend(append_req.key, record)
        };

        match result {
            Ok(status) => {
                response_header.cas = status.cas;
                encoder::BinaryResponse::Append(binary::AppendResponse {
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
    ) -> encoder::BinaryResponse {
        let record = store::Record::new(
            set_req.value,
            set_req.header.cas,
            set_req.flags,
            set_req.expiration,
        );

        match self.storage.set(set_req.key, record) {
            Ok(status) => {
                response_header.cas = status.cas;
                encoder::BinaryResponse::Set(binary::SetResponse {
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
    ) -> encoder::BinaryResponse {
        let result = self.storage.delete(
            delete_request.key,
            into_record_meta(&delete_request.header, 0),
        );
        match result {
            Ok(_record) => encoder::BinaryResponse::Delete(binary::DeleteResponse {
                header: *response_header,
            }),
            Err(err) => storage_error_to_response(err, response_header),
        }
    }

    fn get(
        &self,
        get_request: binary::GetRequest,
        response_header: &mut binary::ResponseHeader,
    ) -> encoder::BinaryResponse {
        let result = self.storage.get(&get_request.key);

        match result {
            Ok(record) => {
                let include_key = self.is_get_key_command(get_request.header.opcode);
                let mut key: Bytes = Bytes::new();
                if include_key {
                    key = get_request.key
                }
                response_header.body_length =
                    record.value.len() as u32 + EXTRAS_LENGTH as u32 + key.len() as u32;
                response_header.key_length = key.len() as u16;
                response_header.extras_length = EXTRAS_LENGTH;
                response_header.cas = record.header.cas;
                encoder::BinaryResponse::Get(binary::GetResponse {
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
    ) -> encoder::BinaryResponse {
        let meta: store::Meta = store::Meta::new(0, 0, flush_request.expiration);
        self.storage.flush(meta);
        encoder::BinaryResponse::Flush(binary::FlushResponse {
            header: *response_header,
        })
    }

    fn increment(
        &self,
        inc_request: binary::IncrementRequest,
        response_header: &mut binary::ResponseHeader,
    ) -> encoder::BinaryResponse {
        let delta = store::IncrementParam {
            delta: inc_request.delta,
            value: inc_request.initial,
        };

        let result = self.storage.increment(
            into_record_meta(&inc_request.header, inc_request.expiration),
            inc_request.key,
            delta,
        );
        match result {
            Ok(delta_result) => {
                response_header.body_length =
                    std::mem::size_of::<store::DeltaResultValueType>() as u32;
                response_header.cas = delta_result.cas;
                encoder::BinaryResponse::Increment(binary::IncrementResponse {
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
    ) -> encoder::BinaryResponse {
        let delta = store::IncrementParam {
            delta: dec_request.delta,
            value: dec_request.initial,
        };

        let result = self.storage.decrement(
            into_record_meta(&dec_request.header, dec_request.expiration),
            dec_request.key,
            delta,
        );
        match result {
            Ok(delta_result) => {
                response_header.body_length =
                    std::mem::size_of::<store::DeltaResultValueType>() as u32;
                response_header.cas = delta_result.cas;
                encoder::BinaryResponse::Decrement(binary::DecrementResponse {
                    header: *response_header,
                    value: delta_result.value,
                })
            }
            Err(err) => storage_error_to_response(err, response_header),
        }
    }
}

#[cfg(any(test, feature = "criterion"))]
pub mod mock {
    use super::binary;
    use super::decoder;
    use super::*;
    use crate::mock::mock_server::create_dash_map_storage;
    use crate::mock::mock_server::create_moka_storage;
    use crate::protocol::binary::decoder::BinaryRequest;

    use bytes::Bytes;
    const OPAQUE_VALUE: u32 = 0xABAD_CAFE;

    pub fn create_dash_map_handler() -> BinaryHandler {
        BinaryHandler::new(create_dash_map_storage())
    }

    pub fn create_moka_handler() -> BinaryHandler {
        BinaryHandler::new(create_moka_storage())
    }

    pub fn create_get_request(header: binary::RequestHeader, key: Bytes) -> BinaryRequest {
        decoder::BinaryRequest::Get(binary::GetRequest { header, key: key.clone() })
    }

    pub fn create_header(opcode: binary::Command, key: &[u8]) -> binary::RequestHeader {
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

    pub fn get_value(handler: &BinaryHandler, key: Bytes) -> Bytes {
        let header = create_header(binary::Command::Get, &key);
        let request = decoder::BinaryRequest::Get(binary::GetRequest { header, key });

        let result = handler.handle_request(request);
        match result {
            Some(resp) => {
                if let encoder::BinaryResponse::Get(response) = resp {
                    assert_ne!(response.header.cas, 0);
                    return response.value;
                } else {
                    unreachable!();
                }
            }
            None => unreachable!(),
        }
    }

    pub fn create_set_request(key: Bytes, value: Bytes) -> decoder::BinaryRequest {
        let header = create_header(binary::Command::Set, &key);
        const FLAGS: u32 = 0xDEAD_BEEF;
        decoder::BinaryRequest::Set(binary::SetRequest {
            header,
            key,
            flags: FLAGS,
            expiration: 0,
            value: value.clone(),
        })
    }

    pub fn insert_value(handler: &BinaryHandler, key: Bytes, value: Bytes) {
        let header = create_header(binary::Command::Set, &key);
        const FLAGS: u32 = 0xDEAD_BEEF;
        let request = decoder::BinaryRequest::SetQuietly(binary::SetRequest {
            header,
            key,
            flags: FLAGS,
            expiration: 0,
            value: value.clone(),
        });

        let result = handler.handle_request(request);
        assert!(result.is_none());
    }

    #[allow(clippy::too_many_arguments)]
    pub fn check_header(
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
}
#[cfg(test)]
mod tests {
    use super::binary;
    use super::decoder;
    use super::*;
    use super::mock::*;
    use crate::cache::error;
    use crate::mock::value::from_string;
    use test_case::test_case;

    use bytes::Bytes;

    #[test_case(create_moka_handler() ; "moka_backend")]
    #[test_case(create_dash_map_handler() ; "dash_map_backend")]
    fn get_request_should_return_not_found_when_not_exists(handler: BinaryHandler) {
        let key = Bytes::from("key");
        let header = create_header(binary::Command::Get, &key);

        let request = decoder::BinaryRequest::Get(binary::GetRequest { header, key });

        let result = handler.handle_request(request);
        match result {
            Some(resp) => {
                if let encoder::BinaryResponse::Error(response) = resp {
                    assert_eq!(response.header.status, error::CacheError::NotFound as u16);
                    assert_eq!(response.error, "Not found");
                    assert_eq!(response.header.body_length, response.error.len() as u32);
                } else {
                    unreachable!();
                }
            }
            None => unreachable!(),
        }
    }

    #[test_case(create_moka_handler() ; "moka_backend")]
    #[test_case(create_dash_map_handler() ; "dash_map_backend")]
    fn get_quiet_request_should_return_none_when_not_exists(handler: BinaryHandler) {
        let key = Bytes::from("key");
        let header = create_header(binary::Command::GetQuiet, &key);

        let request =
            decoder::BinaryRequest::GetQuietly(binary::GetQuietRequest { header, key });

        let result = handler.handle_request(request);
        assert!(result.is_none());
    }

    #[test_case(create_moka_handler() ; "moka_backend")]
    #[test_case(create_dash_map_handler() ; "dash_map_backend")]
    fn get_quiet_key_request_should_return_none_when_not_exists(handler: BinaryHandler) {
        let key = Bytes::from("key");
        let header = create_header(binary::Command::GetQuiet, &key);

        let request =
            decoder::BinaryRequest::GetKeyQuietly(binary::GetKeyQuietRequest { header, key });

        let result = handler.handle_request(request);
        assert!(result.is_none());
    }

    #[test_case(create_moka_handler() ; "moka_backend")]
    #[test_case(create_dash_map_handler() ; "dash_map_backend")]
    fn get_key_request_should_return_key_and_record(handler: BinaryHandler) {
        let key = Bytes::from("test_key");
        let value = from_string("test value");

        insert_value(&handler, key.clone(), value.clone());

        let header = create_header(binary::Command::GetKey, &key);
        let request = decoder::BinaryRequest::GetKey(binary::GetKeyRequest {
            header,
            key: key.clone(),
        });

        let result = handler.handle_request(request);
        match result {
            Some(resp) => {
                if let encoder::BinaryResponse::Get(response) = resp {
                    assert_ne!(response.header.cas, 0);
                    check_header(
                        &response.header,
                        binary::Command::GetKey,
                        key.len() as u16,
                        EXTRAS_LENGTH,
                        0,
                        0,
                        key.len() as u32 + value.len() as u32 + EXTRAS_LENGTH as u32,
                    );
                    assert_eq!(response.key[..], key[..]);
                    assert_eq!(response.value[..], value[..]);
                } else {
                    unreachable!();
                }
            }
            None => unreachable!(),
        }
    }

    #[test_case(create_moka_handler() ; "moka_backend")]
    #[test_case(create_dash_map_handler() ; "dash_map_backend")]
    fn get_quiet_key_request_should_return_key_and_record(handler: BinaryHandler) {
        let key = Bytes::from("test_key");
        let value = from_string("test value");

        insert_value(&handler, key.clone(), value.clone());

        let header = create_header(binary::Command::GetKeyQuiet, &key);
        let request = decoder::BinaryRequest::GetKeyQuietly(binary::GetKeyQuietRequest {
            header,
            key: key.clone(),
        });

        let result = handler.handle_request(request);
        match result {
            Some(resp) => {
                if let encoder::BinaryResponse::Get(response) = resp {
                    assert_ne!(response.header.cas, 0);
                    check_header(
                        &response.header,
                        binary::Command::GetKeyQuiet,
                        key.len() as u16,
                        EXTRAS_LENGTH,
                        0,
                        0,
                        key.len() as u32 + value.len() as u32 + EXTRAS_LENGTH as u32,
                    );
                    assert_eq!(response.key[..], key[..]);
                    assert_eq!(response.value[..], value[..]);
                } else {
                    unreachable!();
                }
            }
            None => unreachable!(),
        }
    }

    #[test_case(create_moka_handler() ; "moka_backend")]
    #[test_case(create_dash_map_handler() ; "dash_map_backend")]
    fn get_request_should_return_record(handler: BinaryHandler) {
        let key = Bytes::from("key");
        let header = create_header(binary::Command::Get, &key);
        const FLAGS: u32 = 0xDEAD_BEEF;
        let value = from_string("value");
        let record = store::Record::new(value.clone(), 0, FLAGS, 0);

        let set_result = handler.storage.set(key.clone(), record);
        assert!(set_result.is_ok());

        let request = decoder::BinaryRequest::Get(binary::GetRequest { header, key });

        let result = handler.handle_request(request);
        match result {
            Some(resp) => {
                if let encoder::BinaryResponse::Get(response) = resp {
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

    #[test_case(create_moka_handler() ; "moka_backend")]
    #[test_case(create_dash_map_handler() ; "dash_map_backend")]
    fn set_request_should_succeed(handler: BinaryHandler) {
        let key = Bytes::from("key");
        let header = create_header(binary::Command::Set, &key);
        const FLAGS: u32 = 0xDEAD_BEEF;
        let value = from_string("value");
        let request = decoder::BinaryRequest::Set(binary::SetRequest {
            header,
            flags: FLAGS,
            expiration: 0,
            key,
            value,
        });
        let result = handler.handle_request(request);
        match result {
            Some(resp) => {
                if let encoder::BinaryResponse::Set(response) = resp {
                    assert_ne!(response.header.cas, 0);
                    check_header(&response.header, binary::Command::Set, 0, 0, 0, 0, 0);
                } else {
                    unreachable!();
                }
            }
            None => unreachable!(),
        }
    }

    #[test_case(create_moka_handler() ; "moka_backend")]
    #[test_case(create_dash_map_handler() ; "dash_map_backend")]
    fn set_request_should_return_item_too_large_(handler: BinaryHandler) {
        let key = Bytes::from("key");
        let header = create_header(binary::Command::Set, &key);
        const FLAGS: u32 = 0xDEAD_BEEF;
        let value = from_string("value");
        let request = decoder::BinaryRequest::ItemTooLarge(binary::SetRequest {
            header,
            flags: FLAGS,
            expiration: 0,
            key,
            value,
        });
        let result = handler.handle_request(request);
        match result {
            Some(resp) => {
                if let encoder::BinaryResponse::Error(response) = resp {
                    assert_eq!(response.header.cas, 0);
                    check_header(
                        &response.header,
                        binary::Command::Set,
                        0,
                        0,
                        0,
                        error::CacheError::ValueTooLarge as u16,
                        response.error.len() as u32,
                    );
                } else {
                    unreachable!();
                }
            }
            None => unreachable!(),
        }
    }

    #[test_case(create_moka_handler() ; "moka_backend")]
    #[test_case(create_dash_map_handler() ; "dash_map_backend")]
    fn set_request_on_cas_mismatch_should_return_key_exists(handler: BinaryHandler) {
        let key = Bytes::from("key");
        let mut header = create_header(binary::Command::Set, &key);
        const FLAGS: u32 = 0xDEAD_BEEF;
        let value = from_string("value");

        let request = decoder::BinaryRequest::Set(binary::SetRequest {
            header,
            flags: FLAGS,
            expiration: 0,
            key: key.clone(),
            value: value.clone(),
        });

        let result = handler.handle_request(request);

        match result {
            Some(resp) => {
                if let encoder::BinaryResponse::Set(response) = resp {
                    assert_ne!(response.header.cas, 0);
                    check_header(&response.header, binary::Command::Set, 0, 0, 0, 0, 0);
                } else {
                    unreachable!();
                }
            }
            None => unreachable!(),
        }
        header.cas = 100;
        let request = decoder::BinaryRequest::Set(binary::SetRequest {
            header,
            flags: FLAGS,
            expiration: 0,
            key,
            value,
        });

        let result = handler.handle_request(request);
        match result {
            Some(resp) => {
                if let encoder::BinaryResponse::Error(response) = resp {
                    assert_eq!(response.header.cas, 0);
                    check_header(
                        &response.header,
                        binary::Command::Set,
                        0,
                        0,
                        0,
                        error::CacheError::KeyExists as u16,
                        response.error.len() as u32,
                    );
                } else {
                    unreachable!();
                }
            }
            None => unreachable!(),
        }
    }

    #[test_case(create_moka_handler() ; "moka_backend")]
    #[test_case(create_dash_map_handler() ; "dash_map_backend")]
    fn version_request_should_return_version(handler: BinaryHandler) {
        let key = String::from("").into_bytes();
        let header = create_header(binary::Command::Version, &key);
        let request = decoder::BinaryRequest::Version(binary::VersionRequest { header });

        let result = handler.handle_request(request);
        match result {
            Some(resp) => {
                if let encoder::BinaryResponse::Version(response) = resp {
                    check_header(
                        &response.header,
                        binary::Command::Version,
                        0,
                        0,
                        0,
                        0,
                        MEMCRS_VERSION.len() as u32,
                    );
                } else {
                    unreachable!();
                }
            }
            None => unreachable!(),
        }
    }

    #[test_case(create_moka_handler() ; "moka_backend")]
    #[test_case(create_dash_map_handler() ; "dash_map_backend")]
    fn increment_request_should_return_cas(handler: BinaryHandler) {
        const EXPECTED_VALUE: u64 = 1;
        let key = Bytes::from("counter");
        let header = create_header(binary::Command::Increment, &key);
        let request = decoder::BinaryRequest::Increment(binary::IncrementRequest {
            header,
            delta: 1,
            initial: 1,
            expiration: 1,
            key,
        });

        let result = handler.handle_request(request);
        match result {
            Some(resp) => {
                if let encoder::BinaryResponse::Increment(response) = resp {
                    check_header(
                        &response.header,
                        binary::Command::Increment,
                        0,
                        0,
                        0,
                        0,
                        std::mem::size_of::<store::DeltaResultValueType>() as u32,
                    );
                    assert_eq!(response.value, EXPECTED_VALUE);
                    assert_ne!(response.header.cas, 0);
                } else {
                    unreachable!();
                }
            }
            None => unreachable!(),
        }
    }

    #[test_case(create_moka_handler() ; "moka_backend")]
    #[test_case(create_dash_map_handler() ; "dash_map_backend")]
    fn increment_request_should_increment_value(handler: BinaryHandler) {
        const EXPECTED_VALUE: u64 = 101;
        let key = Bytes::from("counter");
        let value = from_string("100");
        insert_value(&handler, key.clone(), value);

        let header = create_header(binary::Command::Increment, &key);
        let request = decoder::BinaryRequest::Increment(binary::IncrementRequest {
            header,
            delta: 1,
            initial: 1,
            expiration: 1,
            key,
        });

        let result = handler.handle_request(request);
        match result {
            Some(resp) => {
                if let encoder::BinaryResponse::Increment(response) = resp {
                    check_header(
                        &response.header,
                        binary::Command::Increment,
                        0,
                        0,
                        0,
                        0,
                        std::mem::size_of::<store::DeltaResultValueType>() as u32,
                    );
                    assert_eq!(response.value, EXPECTED_VALUE);
                    assert_ne!(response.header.cas, 0);
                } else {
                    unreachable!();
                }
            }
            None => unreachable!(),
        }
    }

    #[test_case(create_moka_handler() ; "moka_backend")]
    #[test_case(create_dash_map_handler() ; "dash_map_backend")]
    fn increment_quiet_should_increment_value(handler: BinaryHandler) {
        let key = Bytes::from("counter");
        let value = from_string("100");
        insert_value(&handler, key.clone(), value);

        let header = create_header(binary::Command::IncrementQuiet, &key);
        let request = decoder::BinaryRequest::IncrementQuiet(binary::IncrementRequest {
            header,
            delta: 1,
            initial: 1,
            expiration: 1,
            key: key.clone(),
        });

        let result = handler.handle_request(request);
        match result {
            Some(_resp) => unreachable!(),
            None => {}
        }
        let incremented_value = get_value(&handler, key.clone());
        let expected_value = from_string("101");
        assert_eq!(incremented_value[..], expected_value[..]);
    }

    #[test_case(create_moka_handler() ; "moka_backend")]
    #[test_case(create_dash_map_handler() ; "dash_map_backend")]
    fn decrement_request_should_return_cas(handler: BinaryHandler) {
        const EXPECTED_VALUE: u64 = 1;
        let key = Bytes::from("counter");
        let header = create_header(binary::Command::Decrement, &key);
        let request = decoder::BinaryRequest::Decrement(binary::DecrementRequest {
            header,
            delta: 1,
            initial: 1,
            expiration: 1,
            key,
        });

        let result = handler.handle_request(request);
        match result {
            Some(resp) => {
                if let encoder::BinaryResponse::Decrement(response) = resp {
                    check_header(
                        &response.header,
                        binary::Command::Decrement,
                        0,
                        0,
                        0,
                        0,
                        std::mem::size_of::<store::DeltaResultValueType>() as u32,
                    );
                    assert_eq!(response.value, EXPECTED_VALUE);
                    assert_ne!(response.header.cas, 0);
                } else {
                    unreachable!();
                }
            }
            None => unreachable!(),
        }
    }

    #[test_case(create_moka_handler() ; "moka_backend")]
    #[test_case(create_dash_map_handler() ; "dash_map_backend")]
    fn decrement_request_should_decrement_value(handler: BinaryHandler) {
        const EXPECTED_VALUE: u64 = 99;
        let key = Bytes::from("counter");
        let value = from_string("100");
        insert_value(&handler, key.clone(), value);

        let header = create_header(binary::Command::Decrement, &key);
        let request = decoder::BinaryRequest::Decrement(binary::DecrementRequest {
            header,
            delta: 1,
            initial: 1,
            expiration: 1,
            key,
        });

        let result = handler.handle_request(request);
        match result {
            Some(resp) => {
                if let encoder::BinaryResponse::Decrement(response) = resp {
                    check_header(
                        &response.header,
                        binary::Command::Decrement,
                        0,
                        0,
                        0,
                        0,
                        std::mem::size_of::<store::DeltaResultValueType>() as u32,
                    );
                    assert_eq!(response.value, EXPECTED_VALUE);
                    assert_ne!(response.header.cas, 0);
                } else {
                    unreachable!();
                }
            }
            None => unreachable!(),
        }
    }

    #[test_case(create_moka_handler() ; "moka_backend")]
    #[test_case(create_dash_map_handler() ; "dash_map_backend")]
    fn decrement_quiet_should_increment_value(handler: BinaryHandler) {
        let key = Bytes::from("counter");
        let value = from_string("100");
        insert_value(&handler, key.clone(), value);

        let header = create_header(binary::Command::DecrementQuiet, &key);
        let request = decoder::BinaryRequest::DecrementQuiet(binary::DecrementRequest {
            header,
            delta: 1,
            initial: 1,
            expiration: 1,
            key: key.clone(),
        });

        let result = handler.handle_request(request);
        match result {
            Some(_resp) => unreachable!(),
            None => {}
        }
        let dec_value = get_value(&handler, key.clone());
        let expected_value = from_string("99");
        assert_eq!(dec_value[..], expected_value[..]);
    }

    #[test_case(create_moka_handler() ; "moka_backend")]
    #[test_case(create_dash_map_handler() ; "dash_map_backend")]
    fn increment_request_should_error_when_expiration_is_ffffffff(handler: BinaryHandler) {
        let key = Bytes::from("counter");
        let header = create_header(binary::Command::Increment, &key);
        let request = decoder::BinaryRequest::Increment(binary::IncrementRequest {
            header,
            delta: 1,
            initial: 1,
            expiration: 0xffffffff,
            key,
        });

        let result = handler.handle_request(request);
        match result {
            Some(resp) => {
                if let encoder::BinaryResponse::Error(response) = resp {
                    check_header(
                        &response.header,
                        binary::Command::Increment,
                        0,
                        0,
                        0,
                        binary::ResponseStatus::KeyNotExists as u16,
                        response.error.len() as u32,
                    );
                    assert_eq!(response.header.cas, 0);
                } else {
                    unreachable!();
                }
            }
            None => unreachable!(),
        }
    }

    #[test_case(create_moka_handler() ; "moka_backend")]
    #[test_case(create_dash_map_handler() ; "dash_map_backend")]
    fn decrement_request_should_error_when_expiration_is_ffffffff(handler: BinaryHandler) {
        let key = Bytes::from("counter");
        let header = create_header(binary::Command::Decrement, &key);
        let request = decoder::BinaryRequest::Decrement(binary::DecrementRequest {
            header,
            delta: 1,
            initial: 1,
            expiration: 0xffffffff,
            key,
        });

        let result = handler.handle_request(request);
        match result {
            Some(resp) => {
                if let encoder::BinaryResponse::Error(response) = resp {
                    check_header(
                        &response.header,
                        binary::Command::Decrement,
                        0,
                        0,
                        0,
                        binary::ResponseStatus::KeyNotExists as u16,
                        response.error.len() as u32,
                    );
                    assert_eq!(response.header.cas, 0);
                } else {
                    unreachable!();
                }
            }
            None => unreachable!(),
        }
    }

    #[test_case(create_moka_handler() ; "moka_backend")]
    #[test_case(create_dash_map_handler() ; "dash_map_backend")]
    fn flush_should_remove_all(handler: BinaryHandler) {
        let value = from_string("test value");
        for key_suffix in 0..100 {
            let key = Bytes::from(String::from("test_key") + &key_suffix.to_string());
            insert_value(&handler, key.clone(), value.clone());
        }

        let key = String::from("").into_bytes();
        let header = create_header(binary::Command::Flush, &key);
        let request = decoder::BinaryRequest::Flush(binary::FlushRequest {
            header,
            expiration: 0,
        });

        let result = handler.handle_request(request);
        match result {
            Some(resp) => {
                if let encoder::BinaryResponse::Flush(response) = resp {
                    check_header(&response.header, binary::Command::Flush, 0, 0, 0, 0, 0);
                } else {
                    unreachable!();
                }
            }
            None => unreachable!(),
        }
    }

    #[test_case(create_moka_handler() ; "moka_backend")]
    #[test_case(create_dash_map_handler() ; "dash_map_backend")]
    fn delete_should_remove_from_store(handler: BinaryHandler) {
        let value = from_string("test value");
        let key = Bytes::from("test_key");
        insert_value(&handler, key.clone(), value.clone());

        let header = create_header(binary::Command::Delete, &key);
        let request = decoder::BinaryRequest::Delete(binary::DeleteRequest {
            header,
            key: key.clone(),
        });
        let result = handler.handle_request(request);
        match result {
            Some(resp) => {
                if let encoder::BinaryResponse::Delete(response) = resp {
                    check_header(&response.header, binary::Command::Delete, 0, 0, 0, 0, 0);
                } else {
                    unreachable!();
                }
            }
            None => unreachable!(),
        }
    }

    #[test_case(create_moka_handler() ; "moka_backend")]
    #[test_case(create_dash_map_handler() ; "dash_map_backend")]
    fn delete_should_return_error_if_not_exists(handler: BinaryHandler) {
        let key = Bytes::from("test_key");

        let header = create_header(binary::Command::DeleteQuiet, &key);
        let request = decoder::BinaryRequest::DeleteQuiet(binary::DeleteRequest {
            header,
            key: key.clone(),
        });
        let result = handler.handle_request(request);
        match result {
            Some(resp) => {
                if let encoder::BinaryResponse::Error(response) = resp {
                    check_header(
                        &response.header,
                        binary::Command::DeleteQuiet,
                        0,
                        0,
                        0,
                        binary::ResponseStatus::KeyNotExists as u16,
                        response.error.len() as u32,
                    );
                } else {
                    unreachable!();
                }
            }
            None => unreachable!(),
        }
    }

    #[test_case(create_moka_handler() ; "moka_backend")]
    #[test_case(create_dash_map_handler() ; "dash_map_backend")]
    fn noop_request(handler: BinaryHandler) {
        let key = String::from("").into_bytes();

        let header = create_header(binary::Command::Noop, &key);
        let request = decoder::BinaryRequest::Noop(binary::NoopRequest { header });
        let result = handler.handle_request(request);
        match result {
            Some(resp) => {
                if let encoder::BinaryResponse::Noop(response) = resp {
                    check_header(&response.header, binary::Command::Noop, 0, 0, 0, 0, 0);
                } else {
                    unreachable!();
                }
            }
            None => unreachable!(),
        }
    }

    #[test_case(create_moka_handler() ; "moka_backend")]
    #[test_case(create_dash_map_handler() ; "dash_map_backend")]
    fn quit_request(handler: BinaryHandler) {
        let key = String::from("").into_bytes();

        let header = create_header(binary::Command::Quit, &key);
        let request = decoder::BinaryRequest::Quit(binary::QuitRequest { header });
        let result = handler.handle_request(request);
        match result {
            Some(resp) => {
                if let encoder::BinaryResponse::Quit(response) = resp {
                    check_header(&response.header, binary::Command::Quit, 0, 0, 0, 0, 0);
                } else {
                    unreachable!();
                }
            }
            None => unreachable!(),
        }
    }

    #[test_case(create_moka_handler() ; "moka_backend")]
    #[test_case(create_dash_map_handler() ; "dash_map_backend")]
    fn quit_quiet_request(handler: BinaryHandler) {
        let key = String::from("").into_bytes();

        let header = create_header(binary::Command::QuitQuiet, &key);
        let request = decoder::BinaryRequest::QuitQuietly(binary::QuitRequest { header });
        let result = handler.handle_request(request);
        match result {
            Some(_resp) => unreachable!(),
            None => {}
        }
    }

    #[test_case(create_moka_handler() ; "moka_backend")]
    #[test_case(create_dash_map_handler() ; "dash_map_backend")]
    fn add_request(handler: BinaryHandler) {
        let key = Bytes::from("key");
        let mut header = create_header(binary::Command::Add, &key);
        const FLAGS: u32 = 0xDEAD_BEEF;
        let value = from_string("value");

        let request = decoder::BinaryRequest::Add(binary::AddRequest {
            header,
            flags: FLAGS,
            expiration: 0,
            key: key.clone(),
            value: value.clone(),
        });

        let result = handler.handle_request(request);

        match result {
            Some(resp) => {
                if let encoder::BinaryResponse::Set(response) = resp {
                    assert_ne!(response.header.cas, 0);
                    check_header(&response.header, binary::Command::Add, 0, 0, 0, 0, 0);
                } else {
                    unreachable!();
                }
            }
            None => unreachable!(),
        }
        header.cas = 100;
        let request = decoder::BinaryRequest::Add(binary::SetRequest {
            header,
            flags: FLAGS,
            expiration: 0,
            key,
            value,
        });

        let result = handler.handle_request(request);
        match result {
            Some(resp) => {
                if let encoder::BinaryResponse::Error(response) = resp {
                    assert_eq!(response.header.cas, 0);
                    check_header(
                        &response.header,
                        binary::Command::Add,
                        0,
                        0,
                        0,
                        error::CacheError::KeyExists as u16,
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
