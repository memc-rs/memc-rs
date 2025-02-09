use crate::memcache_server::handler::BinaryHandler;
use crate::mock::mock_server::create_dash_map_storage;
use crate::mock::mock_server::create_moka_storage;
use crate::protocol::binary::decoder;
use crate::protocol::binary::decoder::BinaryRequest;
use crate::protocol::binary::encoder;
use crate::protocol::binary::network;

use bytes::Bytes;
use std::sync::Arc;

use super::mock_server::MockSystemTimer;
const OPAQUE_VALUE: u32 = 0xABAD_CAFE;

pub struct BinaryHandlerWithTimer {
    pub handler: BinaryHandler,
    pub timer: Arc<MockSystemTimer>,
}

impl BinaryHandlerWithTimer {
    pub fn new(handler: BinaryHandler, timer: Arc<MockSystemTimer>) -> BinaryHandlerWithTimer {
        BinaryHandlerWithTimer { handler, timer }
    }

    pub fn handle_request(&self, req: decoder::BinaryRequest) -> Option<encoder::BinaryResponse> {
        self.handler.handle_request(req)
    }
}

pub fn create_dash_map_handler() -> BinaryHandlerWithTimer {
    let store_with_timer = create_dash_map_storage();
    BinaryHandlerWithTimer::new(
        BinaryHandler::new(store_with_timer.memc_store),
        store_with_timer.timer,
    )
}

pub fn create_moka_handler() -> BinaryHandlerWithTimer {
    let store_with_timer = create_moka_storage();
    BinaryHandlerWithTimer::new(
        BinaryHandler::new(store_with_timer.memc_store),
        store_with_timer.timer,
    )
}

pub fn create_get_request(header: network::RequestHeader, key: Bytes) -> BinaryRequest {
    decoder::BinaryRequest::Get(network::GetRequest {
        header,
        key: key.clone(),
    })
}

pub fn create_header(opcode: network::Command, key: &[u8]) -> network::RequestHeader {
    network::RequestHeader {
        magic: network::Magic::Request as u8,
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

pub fn get_value(handler: &BinaryHandlerWithTimer, key: Bytes) -> Option<Bytes> {
    let header = create_header(network::Command::Get, &key);
    let request = decoder::BinaryRequest::Get(network::GetRequest { header, key });

    let result = handler.handle_request(request);
    match result {
        Some(resp) => {
            if let encoder::BinaryResponse::Get(response) = resp {
                assert_ne!(response.header.cas, 0);
                return Some(response.value);
            } else {
                return None;
            }
        }
        None => return None,
    }
}

pub fn create_set_request(key: Bytes, value: Bytes) -> decoder::BinaryRequest {
    let header = create_header(network::Command::Set, &key);
    const FLAGS: u32 = 0xDEAD_BEEF;
    decoder::BinaryRequest::Set(network::SetRequest {
        header,
        key,
        flags: FLAGS,
        expiration: 0,
        value: value.clone(),
    })
}

pub fn create_get_request_by_key(key: &Bytes) -> BinaryRequest {
    let header = create_header(network::Command::Get, &key);
    decoder::BinaryRequest::Get(network::GetRequest {
        header,
        key: key.clone(),
    })
}

pub fn insert_value(handler: &BinaryHandlerWithTimer, key: Bytes, value: Bytes) {
    insert_value_with_expire(handler, key, value, 0)
}

pub fn insert_value_with_expire(
    handler: &BinaryHandlerWithTimer,
    key: Bytes,
    value: Bytes,
    expiration: u32,
) {
    let header = create_header(network::Command::Set, &key);
    const FLAGS: u32 = 0xDEAD_BEEF;
    let request = decoder::BinaryRequest::SetQuietly(network::SetRequest {
        header,
        key,
        flags: FLAGS,
        expiration: expiration,
        value: value.clone(),
    });

    let result = handler.handle_request(request);
    assert!(result.is_none());
}
#[allow(clippy::too_many_arguments)]
pub fn check_header(
    response: &network::ResponseHeader,
    opcode: network::Command,
    key_length: u16,
    extras_length: u8,
    data_type: u8,
    status: u16,
    body_length: u32,
) {
    assert_eq!(response.magic, network::Magic::Response as u8);
    assert_eq!(response.opcode, opcode as u8);
    assert_eq!(response.key_length, key_length);
    assert_eq!(response.extras_length, extras_length);
    assert_eq!(response.data_type, data_type);
    assert_eq!(response.status, status);
    assert_eq!(response.body_length, body_length);
    assert_eq!(response.opaque, OPAQUE_VALUE);
}
