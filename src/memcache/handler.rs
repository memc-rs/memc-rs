use super::storage;
use crate::protocol::{binary, binary_codec};
use std::sync::Arc;

pub struct BinaryHandler {
    storage: Arc<storage::Storage>,
}

impl BinaryHandler {
    pub fn new(store: Arc<storage::Storage>) -> BinaryHandler {
        BinaryHandler { storage: store }
    }

    pub fn handle_request(
        &mut self,
        req: binary_codec::BinaryRequest,
    ) -> Option<binary_codec::BinaryResponse> {
        let request_header = req.get_header();
        let mut response_header = BinaryHandler::create_header(request_header.opcode);

        let result = match req {
            binary_codec::BinaryRequest::Get(get_request) => {
                let record = self.storage.get(&get_request.key);
                let r = match record {
                    Some(storage::Record::Value(data)) => {
                        response_header.body_length = data.value.len() as u32 + 4;
                        response_header.cas = 1;
                        Some(binary_codec::BinaryResponse::Get(binary::GetResponse {
                            header: response_header,
                            flags: data.header.flags,
                            key: Vec::new(),
                            value: data.value,
                        }))
                    }
                    Some(storage::Record::Counter(counter)) => None,
                    None => None,
                };
                r
            }
            binary_codec::BinaryRequest::GetQuietly(get_quiet_req) => None,
            binary_codec::BinaryRequest::GetKey(get_key_req) => None,
            binary_codec::BinaryRequest::GetKeyQuietly(get_key_quiet_req) => None,
            binary_codec::BinaryRequest::Set(set_req) => {
                let record = storage::ValueData::new(
                    set_req.key,
                    set_req.value,
                    set_req.header.cas,
                    set_req.flags,
                    set_req.expiration,
                );
                self.storage.set(storage::Record::Value(record));
                response_header.cas = 1;
                Some(binary_codec::BinaryResponse::Set(binary::SetResponse {
                    header: response_header,
                }))
            }
            binary_codec::BinaryRequest::Add(add_req) => None,
            binary_codec::BinaryRequest::Replace(replace_req) => None,
        };
        result
    }

    fn create_header(cmd: u8) -> binary::ResponseHeader {
        binary::ResponseHeader {
            magic: binary::Magic::Response as u8,
            opcode: cmd,
            ..binary::ResponseHeader::default()
        }
    }
}
