use crate::memcache_server::handler::BinaryHandler;
use crate::mock::mock_server::create_dash_map_storage;
use crate::mock::mock_server::create_moka_storage;
use crate::protocol::binary::decoder;
use crate::protocol::binary::decoder::BinaryRequest;
use crate::protocol::binary::encoder;
use crate::protocol::binary::network;

use bytes::Bytes;
const OPAQUE_VALUE: u32 = 0xABAD_CAFE;

pub fn create_dash_map_handler() -> BinaryHandler {
    BinaryHandler::new(create_dash_map_storage())
}

pub fn create_moka_handler() -> BinaryHandler {
    BinaryHandler::new(create_moka_storage())
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

pub fn get_value(handler: &BinaryHandler, key: Bytes) -> Bytes {
    let header = create_header(network::Command::Get, &key);
    let request = decoder::BinaryRequest::Get(network::GetRequest { header, key });

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

pub fn insert_value(handler: &BinaryHandler, key: Bytes, value: Bytes) {
    let header = create_header(network::Command::Set, &key);
    const FLAGS: u32 = 0xDEAD_BEEF;
    let request = decoder::BinaryRequest::SetQuietly(network::SetRequest {
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
