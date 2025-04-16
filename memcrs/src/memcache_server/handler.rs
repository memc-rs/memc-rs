use crate::cache::error::CacheError;
use crate::memcache::store;
use crate::protocol::binary::encoder::storage_error_to_response;
use crate::protocol::binary::{decoder, encoder, network};
use crate::version::MEMCRS_VERSION;
use bytes::Bytes;
use std::sync::Arc;

const EXTRAS_LENGTH: u8 = 4;

fn into_record_meta(request_header: &network::RequestHeader, expiration: u32) -> store::Meta {
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

fn into_quiet_mutation(response: encoder::BinaryResponse) -> Option<encoder::BinaryResponse> {
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

    pub fn handle_request(&self, req: decoder::BinaryRequest) -> Option<encoder::BinaryResponse> {
        let request_header = req.get_header();
        let mut response_header =
            network::ResponseHeader::new(request_header.opcode, request_header.opaque);

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
                Some(encoder::BinaryResponse::Noop(network::NoopResponse {
                    header: response_header,
                }))
            }
            decoder::BinaryRequest::Stats(_stat_request) => {
                Some(encoder::BinaryResponse::Stats(network::StatsResponse {
                    header: response_header,
                }))
            }
            decoder::BinaryRequest::Quit(_quit_req) => {
                Some(encoder::BinaryResponse::Quit(network::QuitResponse {
                    header: response_header,
                }))
            }
            decoder::BinaryRequest::QuitQuietly(_quit_req) => {
                into_quiet_mutation(encoder::BinaryResponse::Quit(network::QuitResponse {
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
                Some(encoder::BinaryResponse::Version(network::VersionResponse {
                    header: response_header,
                    version: String::from(MEMCRS_VERSION),
                }))
            }
            decoder::BinaryRequest::ItemTooLarge(_set_request) => Some(storage_error_to_response(
                CacheError::ValueTooLarge,
                &mut response_header,
            )),
        }
    }

    fn add_replace(
        &self,
        request: network::SetRequest,
        response_header: &mut network::ResponseHeader,
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
                encoder::BinaryResponse::Set(network::SetResponse {
                    header: *response_header,
                })
            }
            Err(err) => storage_error_to_response(err, response_header),
        }
    }

    fn is_add_command(&self, opcode: u8) -> bool {
        opcode == network::Command::Add as u8 || opcode == network::Command::AddQuiet as u8
    }

    fn append_prepend(
        &self,
        append_prepend_req: network::AppendRequest,
        response_header: &mut network::ResponseHeader,
    ) -> encoder::BinaryResponse {
        let record = store::Record::new(
            append_prepend_req.value,
            append_prepend_req.header.cas,
            0,
            0,
        );
        let is_append = self.is_append(append_prepend_req.header.opcode);
        let result = if is_append {
            self.storage.append(append_prepend_req.key, record)
        } else {
            self.storage.prepend(append_prepend_req.key, record)
        };

        match result {
            Ok(status) => {
                response_header.cas = status.cas;
                if is_append {
                    return encoder::BinaryResponse::Append(network::AppendResponse {
                        header: *response_header,
                    });
                }
                encoder::BinaryResponse::Prepend(network::PrependResponse {
                    header: *response_header,
                })
            }
            Err(err) => storage_error_to_response(err, response_header),
        }
    }

    fn is_append(&self, opcode: u8) -> bool {
        opcode == network::Command::Append as u8 || opcode == network::Command::AppendQuiet as u8
    }

    fn set(
        &self,
        set_req: network::SetRequest,
        response_header: &mut network::ResponseHeader,
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
                encoder::BinaryResponse::Set(network::SetResponse {
                    header: *response_header,
                })
            }
            Err(err) => storage_error_to_response(err, response_header),
        }
    }

    fn delete(
        &self,
        delete_request: network::DeleteRequest,
        response_header: &mut network::ResponseHeader,
    ) -> encoder::BinaryResponse {
        let result = self.storage.delete(
            delete_request.key,
            into_record_meta(&delete_request.header, 0),
        );
        match result {
            Ok(_record) => encoder::BinaryResponse::Delete(network::DeleteResponse {
                header: *response_header,
            }),
            Err(err) => storage_error_to_response(err, response_header),
        }
    }

    fn get(
        &self,
        get_request: network::GetRequest,
        response_header: &mut network::ResponseHeader,
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
                encoder::BinaryResponse::Get(network::GetResponse {
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
        opcode == network::Command::GetKey as u8 || opcode == network::Command::GetKeyQuiet as u8
    }

    fn flush(
        &self,
        flush_request: network::FlushRequest,
        response_header: &mut network::ResponseHeader,
    ) -> encoder::BinaryResponse {
        let meta: store::Meta = store::Meta::new(0, 0, flush_request.expiration);
        self.storage.flush(meta);
        encoder::BinaryResponse::Flush(network::FlushResponse {
            header: *response_header,
        })
    }

    fn increment(
        &self,
        inc_request: network::IncrementRequest,
        response_header: &mut network::ResponseHeader,
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
                encoder::BinaryResponse::Increment(network::IncrementResponse {
                    header: *response_header,
                    value: delta_result.value,
                })
            }
            Err(err) => storage_error_to_response(err, response_header),
        }
    }

    fn decrement(
        &self,
        dec_request: network::IncrementRequest,
        response_header: &mut network::ResponseHeader,
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
                encoder::BinaryResponse::Decrement(network::DecrementResponse {
                    header: *response_header,
                    value: delta_result.value,
                })
            }
            Err(err) => storage_error_to_response(err, response_header),
        }
    }
}

#[cfg(test)]
mod handler_tests;
