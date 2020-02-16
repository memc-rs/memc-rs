use std::sync::Arc;
use super::storage;
use crate::protocol::{binary, binary_codec};


pub struct BinaryHandler {
    storage: Arc<storage::Storage>
}

impl BinaryHandler {
    pub fn new(store: Arc<storage::Storage>) -> BinaryHandler {
        BinaryHandler{
            storage: store
        }
    }
    pub fn handle_request(&mut self, req: &binary_codec::BinaryRequest) -> Option<binary_codec::BinaryResponse> {
        let header = binary::ResponseHeader {
            magic: binary::Magic::Response as u8,
            opcode: binary::Command::Set as u8,
            cas: 0x01,
            ..binary::ResponseHeader::default()
        };
        Some(binary_codec::BinaryResponse::Set(binary::SetResponse { header: header }))
    }
}