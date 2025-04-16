use super::*;

#[cfg(test)]
mod tests {
    use super::decoder;
    use super::network;
    use crate::cache::error;
    use crate::memcache::store;
    use crate::memcache_server::handler::EXTRAS_LENGTH;
    use crate::mock::handler::*;
    use crate::mock::mock_server::SetableTimer;
    use crate::mock::value::from_string;
    use crate::protocol::binary::encoder;
    use crate::version::MEMCRS_VERSION;
    use test_case::test_case;

    use bytes::Bytes;

    #[test_case(create_moka_handler() ; "moka_backend")]
    #[test_case(create_dash_map_handler() ; "dash_map_backend")]
    fn get_request_should_return_not_found_when_not_exists(handler: BinaryHandlerWithTimer) {
        let key = Bytes::from("key");
        let header = create_header(network::Command::Get, &key);

        let request = decoder::BinaryRequest::Get(network::GetRequest { header, key });

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
    fn get_quiet_request_should_return_none_when_not_exists(handler: BinaryHandlerWithTimer) {
        let key = Bytes::from("key");
        let header = create_header(network::Command::GetQuiet, &key);

        let request = decoder::BinaryRequest::GetQuietly(network::GetQuietRequest { header, key });

        let result = handler.handle_request(request);
        assert!(result.is_none());
    }

    #[test_case(create_moka_handler() ; "moka_backend")]
    #[test_case(create_dash_map_handler() ; "dash_map_backend")]
    fn get_quiet_key_request_should_return_none_when_not_exists(handler: BinaryHandlerWithTimer) {
        let key = Bytes::from("key");
        let header = create_header(network::Command::GetQuiet, &key);

        let request =
            decoder::BinaryRequest::GetKeyQuietly(network::GetKeyQuietRequest { header, key });

        let result = handler.handle_request(request);
        assert!(result.is_none());
    }

    #[test_case(create_moka_handler() ; "moka_backend")]
    #[test_case(create_dash_map_handler() ; "dash_map_backend")]
    fn get_key_request_should_return_key_and_record(handler: BinaryHandlerWithTimer) {
        let key = Bytes::from("test_key");
        let value = from_string("test value");

        insert_value(&handler, key.clone(), value.clone());

        let header = create_header(network::Command::GetKey, &key);
        let request = decoder::BinaryRequest::GetKey(network::GetKeyRequest {
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
                        network::Command::GetKey,
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
    fn get_quiet_key_request_should_return_key_and_record(handler: BinaryHandlerWithTimer) {
        let key = Bytes::from("test_key");
        let value = from_string("test value");

        insert_value(&handler, key.clone(), value.clone());

        let header = create_header(network::Command::GetKeyQuiet, &key);
        let request = decoder::BinaryRequest::GetKeyQuietly(network::GetKeyQuietRequest {
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
                        network::Command::GetKeyQuiet,
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
    fn get_request_should_return_record(handler: BinaryHandlerWithTimer) {
        let key = Bytes::from("key");
        let header = create_header(network::Command::Get, &key);
        const FLAGS: u32 = 0xDEAD_BEEF;
        let value = from_string("value");
        let record = store::Record::new(value.clone(), 0, FLAGS, 0);

        let set_result = handler.handler.storage.set(key.clone(), record);
        assert!(set_result.is_ok());

        let request = decoder::BinaryRequest::Get(network::GetRequest { header, key });

        let result = handler.handle_request(request);
        match result {
            Some(resp) => {
                if let encoder::BinaryResponse::Get(response) = resp {
                    assert_eq!(response.flags, FLAGS);
                    assert_ne!(response.header.cas, 0);
                    check_header(
                        &response.header,
                        network::Command::Get,
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
    fn get_request_should_not_return_expired_record(handler: BinaryHandlerWithTimer) {
        let key = Bytes::from("key");
        let header = create_header(network::Command::Get, &key);
        let value = from_string("value");
        // insert value with TTL = 2
        insert_value_with_expire(&handler, key.clone(), value.clone(), 2);
        // add 3 seconds
        handler.timer.add_seconds(3);

        let request = decoder::BinaryRequest::Get(network::GetRequest { header, key });
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

    fn get_request_should_return_not_expired_record(handler: BinaryHandlerWithTimer) {
        let key = Bytes::from("key");
        let header = create_header(network::Command::Get, &key);
        let value = from_string("value");
        // set timer some time in the future
        handler.timer.set(100);
        // insert value with TTL = 2
        insert_value_with_expire(&handler, key.clone(), value.clone(), 2);
        // add 3 seconds
        handler.timer.add_seconds(1);

        let request = decoder::BinaryRequest::Get(network::GetRequest { header, key });
        let result = handler.handle_request(request);
        match result {
            Some(resp) => {
                if let encoder::BinaryResponse::Get(response) = resp {
                    assert_ne!(response.header.cas, 0);
                    check_header(
                        &response.header,
                        network::Command::Get,
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
    fn set_request_should_succeed(handler: BinaryHandlerWithTimer) {
        let key = Bytes::from("key");
        let header = create_header(network::Command::Set, &key);
        const FLAGS: u32 = 0xDEAD_BEEF;
        let value = from_string("value");
        let request = decoder::BinaryRequest::Set(network::SetRequest {
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
                    check_header(&response.header, network::Command::Set, 0, 0, 0, 0, 0);
                } else {
                    unreachable!();
                }
            }
            None => unreachable!(),
        }
    }

    #[test_case(create_moka_handler() ; "moka_backend")]
    #[test_case(create_dash_map_handler() ; "dash_map_backend")]
    fn set_request_should_return_item_too_large_(handler: BinaryHandlerWithTimer) {
        let key = Bytes::from("key");
        let header = create_header(network::Command::Set, &key);
        const FLAGS: u32 = 0xDEAD_BEEF;
        let value = from_string("value");
        let request = decoder::BinaryRequest::ItemTooLarge(network::SetRequest {
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
                        network::Command::Set,
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
    fn set_request_on_cas_mismatch_should_return_key_exists(handler: BinaryHandlerWithTimer) {
        let key = Bytes::from("key");
        let mut header = create_header(network::Command::Set, &key);
        const FLAGS: u32 = 0xDEAD_BEEF;
        let value = from_string("value");

        let request = decoder::BinaryRequest::Set(network::SetRequest {
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
                    check_header(&response.header, network::Command::Set, 0, 0, 0, 0, 0);
                } else {
                    unreachable!();
                }
            }
            None => unreachable!(),
        }
        header.cas = 100;
        let request = decoder::BinaryRequest::Set(network::SetRequest {
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
                        network::Command::Set,
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
    fn version_request_should_return_version(handler: BinaryHandlerWithTimer) {
        let key = String::from("").into_bytes();
        let header = create_header(network::Command::Version, &key);
        let request = decoder::BinaryRequest::Version(network::VersionRequest { header });

        let result = handler.handle_request(request);
        match result {
            Some(resp) => {
                if let encoder::BinaryResponse::Version(response) = resp {
                    check_header(
                        &response.header,
                        network::Command::Version,
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
    fn increment_request_should_return_cas(handler: BinaryHandlerWithTimer) {
        const EXPECTED_VALUE: u64 = 1;
        let key = Bytes::from("counter");
        let header = create_header(network::Command::Increment, &key);
        let request = decoder::BinaryRequest::Increment(network::IncrementRequest {
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
                        network::Command::Increment,
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
    fn increment_request_should_increment_value(handler: BinaryHandlerWithTimer) {
        const EXPECTED_VALUE: u64 = 101;
        let key = Bytes::from("counter");
        let value = from_string("100");
        insert_value(&handler, key.clone(), value);

        let header = create_header(network::Command::Increment, &key);
        let request = decoder::BinaryRequest::Increment(network::IncrementRequest {
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
                        network::Command::Increment,
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
    fn increment_quiet_should_increment_value(handler: BinaryHandlerWithTimer) {
        let key = Bytes::from("counter");
        let value = from_string("100");
        insert_value(&handler, key.clone(), value);

        let header = create_header(network::Command::IncrementQuiet, &key);
        let request = decoder::BinaryRequest::IncrementQuiet(network::IncrementRequest {
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
        let incremented_value = get_value(&handler, key.clone()).unwrap();
        let expected_value = from_string("101");
        assert_eq!(incremented_value[..], expected_value[..]);
    }

    #[test_case(create_moka_handler() ; "moka_backend")]
    #[test_case(create_dash_map_handler() ; "dash_map_backend")]
    fn decrement_request_should_return_cas(handler: BinaryHandlerWithTimer) {
        const EXPECTED_VALUE: u64 = 1;
        let key = Bytes::from("counter");
        let header = create_header(network::Command::Decrement, &key);
        let request = decoder::BinaryRequest::Decrement(network::DecrementRequest {
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
                        network::Command::Decrement,
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
    fn decrement_request_should_decrement_value(handler: BinaryHandlerWithTimer) {
        const EXPECTED_VALUE: u64 = 99;
        let key = Bytes::from("counter");
        let value = from_string("100");
        insert_value(&handler, key.clone(), value);

        let header = create_header(network::Command::Decrement, &key);
        let request = decoder::BinaryRequest::Decrement(network::DecrementRequest {
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
                        network::Command::Decrement,
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
    fn decrement_quiet_should_increment_value(handler: BinaryHandlerWithTimer) {
        let key = Bytes::from("counter");
        let value = from_string("100");
        insert_value(&handler, key.clone(), value);

        let header = create_header(network::Command::DecrementQuiet, &key);
        let request = decoder::BinaryRequest::DecrementQuiet(network::DecrementRequest {
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
        let dec_value = get_value(&handler, key.clone()).unwrap();
        let expected_value = from_string("99");
        assert_eq!(dec_value[..], expected_value[..]);
    }

    #[test_case(create_moka_handler() ; "moka_backend")]
    #[test_case(create_dash_map_handler() ; "dash_map_backend")]
    fn increment_request_should_error_when_expiration_is_ffffffff(handler: BinaryHandlerWithTimer) {
        let key = Bytes::from("counter");
        let header = create_header(network::Command::Increment, &key);
        let request = decoder::BinaryRequest::Increment(network::IncrementRequest {
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
                        network::Command::Increment,
                        0,
                        0,
                        0,
                        network::ResponseStatus::KeyNotExists as u16,
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
    fn decrement_request_should_error_when_expiration_is_ffffffff(handler: BinaryHandlerWithTimer) {
        let key = Bytes::from("counter");
        let header = create_header(network::Command::Decrement, &key);
        let request = decoder::BinaryRequest::Decrement(network::DecrementRequest {
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
                        network::Command::Decrement,
                        0,
                        0,
                        0,
                        network::ResponseStatus::KeyNotExists as u16,
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
    fn flush_should_remove_all(handler: BinaryHandlerWithTimer) {
        let value = from_string("test value");
        for key_suffix in 0..100 {
            let key = Bytes::from(String::from("test_key") + &key_suffix.to_string());
            insert_value(&handler, key.clone(), value.clone());
        }

        for key_suffix in 0..100 {
            let key = Bytes::from(String::from("test_key") + &key_suffix.to_string());
            let server_value = get_value(&handler, key.clone()).unwrap();
            assert_eq!(server_value, value.clone());
        }

        let key = String::from("").into_bytes();
        let header = create_header(network::Command::Flush, &key);
        let request = decoder::BinaryRequest::Flush(network::FlushRequest {
            header,
            expiration: 0,
        });

        let result = handler.handle_request(request);
        match result {
            Some(resp) => {
                if let encoder::BinaryResponse::Flush(response) = resp {
                    check_header(&response.header, network::Command::Flush, 0, 0, 0, 0, 0);
                } else {
                    unreachable!();
                }
            }
            None => unreachable!(),
        }
        for key_suffix in 0..100 {
            let key = Bytes::from(String::from("test_key") + &key_suffix.to_string());
            let server_value = get_value(&handler, key.clone());
            assert_eq!(server_value, None);
        }
    }

    #[test_case(create_moka_handler() ; "moka_backend")]
    #[test_case(create_dash_map_handler() ; "dash_map_backend")]
    fn flush_quiet_should_remove_all(handler: BinaryHandlerWithTimer) {
        let value = from_string("test value");
        for key_suffix in 0..100 {
            let key = Bytes::from(String::from("test_key") + &key_suffix.to_string());
            insert_value(&handler, key.clone(), value.clone());
        }

        for key_suffix in 0..100 {
            let key = Bytes::from(String::from("test_key") + &key_suffix.to_string());
            let server_value = get_value(&handler, key.clone()).unwrap();
            assert_eq!(server_value, value.clone());
        }

        let key = String::from("").into_bytes();
        let header = create_header(network::Command::FlushQuiet, &key);
        let request = decoder::BinaryRequest::FlushQuietly(network::FlushRequest {
            header,
            expiration: 0,
        });

        let result = handler.handle_request(request);
        match result {
            Some(_resp) => {
                unreachable!();
            }
            None => {}
        }

        for key_suffix in 0..100 {
            let key = Bytes::from(String::from("test_key") + &key_suffix.to_string());
            let server_value = get_value(&handler, key.clone());
            assert_eq!(server_value, None);
        }
    }

    #[test_case(create_moka_handler() ; "moka_backend")]
    #[test_case(create_dash_map_handler() ; "dash_map_backend")]
    fn delete_should_remove_from_store(handler: BinaryHandlerWithTimer) {
        let value = from_string("test value");
        let key = Bytes::from("test_key");
        insert_value(&handler, key.clone(), value.clone());

        let header = create_header(network::Command::Delete, &key);
        let request = decoder::BinaryRequest::Delete(network::DeleteRequest {
            header,
            key: key.clone(),
        });
        let result = handler.handle_request(request);
        match result {
            Some(resp) => {
                if let encoder::BinaryResponse::Delete(response) = resp {
                    check_header(&response.header, network::Command::Delete, 0, 0, 0, 0, 0);
                } else {
                    unreachable!();
                }
            }
            None => unreachable!(),
        }
    }

    #[test_case(create_moka_handler() ; "moka_backend")]
    #[test_case(create_dash_map_handler() ; "dash_map_backend")]
    fn delete_should_return_error_if_not_exists(handler: BinaryHandlerWithTimer) {
        let key = Bytes::from("test_key");

        let header = create_header(network::Command::DeleteQuiet, &key);
        let request = decoder::BinaryRequest::DeleteQuiet(network::DeleteRequest {
            header,
            key: key.clone(),
        });
        let result = handler.handle_request(request);
        match result {
            Some(resp) => {
                if let encoder::BinaryResponse::Error(response) = resp {
                    check_header(
                        &response.header,
                        network::Command::DeleteQuiet,
                        0,
                        0,
                        0,
                        network::ResponseStatus::KeyNotExists as u16,
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
    fn noop_request(handler: BinaryHandlerWithTimer) {
        let key = String::from("").into_bytes();

        let header = create_header(network::Command::Noop, &key);
        let request = decoder::BinaryRequest::Noop(network::NoopRequest { header });
        let result = handler.handle_request(request);
        match result {
            Some(resp) => {
                if let encoder::BinaryResponse::Noop(response) = resp {
                    check_header(&response.header, network::Command::Noop, 0, 0, 0, 0, 0);
                } else {
                    unreachable!();
                }
            }
            None => unreachable!(),
        }
    }

    #[test_case(create_moka_handler() ; "moka_backend")]
    #[test_case(create_dash_map_handler() ; "dash_map_backend")]
    fn quit_request(handler: BinaryHandlerWithTimer) {
        let key = String::from("").into_bytes();

        let header = create_header(network::Command::Quit, &key);
        let request = decoder::BinaryRequest::Quit(network::QuitRequest { header });
        let result = handler.handle_request(request);
        match result {
            Some(resp) => {
                if let encoder::BinaryResponse::Quit(response) = resp {
                    check_header(&response.header, network::Command::Quit, 0, 0, 0, 0, 0);
                } else {
                    unreachable!();
                }
            }
            None => unreachable!(),
        }
    }

    #[test_case(create_moka_handler() ; "moka_backend")]
    #[test_case(create_dash_map_handler() ; "dash_map_backend")]
    fn quit_quiet_request(handler: BinaryHandlerWithTimer) {
        let key = String::from("").into_bytes();

        let header = create_header(network::Command::QuitQuiet, &key);
        let request = decoder::BinaryRequest::QuitQuietly(network::QuitRequest { header });
        let result = handler.handle_request(request);
        match result {
            Some(_resp) => unreachable!(),
            None => {}
        }
    }

    #[test_case(create_moka_handler() ; "moka_backend")]
    #[test_case(create_dash_map_handler() ; "dash_map_backend")]
    fn add_request_should_succeed_if_item_not_exists_and_fail_if_exists(
        handler: BinaryHandlerWithTimer,
    ) {
        let key = Bytes::from("key");
        let mut header = create_header(network::Command::Add, &key);
        const FLAGS: u32 = 0xDEAD_BEEF;
        let value = from_string("value");

        let request = decoder::BinaryRequest::Add(network::AddRequest {
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
                    check_header(&response.header, network::Command::Add, 0, 0, 0, 0, 0);
                } else {
                    unreachable!();
                }
            }
            None => unreachable!(),
        }
        header.cas = 100;
        let request = decoder::BinaryRequest::Add(network::AddRequest {
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
                        network::Command::Add,
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
    fn add_quiet_request_should_succeed_if_item_does_not_exists(handler: BinaryHandlerWithTimer) {
        let key = Bytes::from("key");
        let mut header = create_header(network::Command::Add, &key);
        const FLAGS: u32 = 0xDEAD_BEEF;
        let value = from_string("value");

        let request = decoder::BinaryRequest::AddQuietly(network::AddRequest {
            header,
            flags: FLAGS,
            expiration: 0,
            key: key.clone(),
            value: value.clone(),
        });

        let result = handler.handle_request(request);

        match result {
            Some(_resp) => {
                unreachable!();
            }
            None => {}
        }
        header.cas = 100;
        let request = decoder::BinaryRequest::AddQuietly(network::AddRequest {
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
                        network::Command::Add,
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
    fn replace_request_should_fail_if_not_exists_and_succeed_if_exists(
        handler: BinaryHandlerWithTimer,
    ) {
        let key = Bytes::from("key");
        let header = create_header(network::Command::Replace, &key);
        const FLAGS: u32 = 0xDEAD_BEEF;
        let value = from_string("value");

        let request = decoder::BinaryRequest::Replace(network::ReplaceRequest {
            header,
            flags: FLAGS,
            expiration: 0,
            key: key.clone(),
            value: value.clone(),
        });

        let result = handler.handle_request(request);

        match result {
            Some(resp) => {
                if let encoder::BinaryResponse::Error(response) = resp {
                    assert_eq!(response.header.cas, 0);
                    check_header(
                        &response.header,
                        network::Command::Replace,
                        0,
                        0,
                        0,
                        error::CacheError::NotFound as u16,
                        response.error.len() as u32,
                    );
                } else {
                    unreachable!();
                }
            }
            None => {
                unreachable!()
            }
        }
        let orig_value = from_string("original_value");
        insert_value(&handler, key.clone(), orig_value.clone());
        let inserted_value = get_value(&handler, key.clone()).unwrap();
        assert_eq!(inserted_value, orig_value);

        let request = decoder::BinaryRequest::Replace(network::ReplaceRequest {
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
                    check_header(&response.header, network::Command::Replace, 0, 0, 0, 0, 0);
                } else {
                    unreachable!();
                }
            }
            None => unreachable!(),
        }
        let replaced_value = get_value(&handler, key.clone()).unwrap();
        assert_eq!(replaced_value, value);
    }

    #[test_case(create_moka_handler() ; "moka_backend")]
    #[test_case(create_dash_map_handler() ; "dash_map_backend")]
    fn replace_quiet_request_should_fail_if_not_exists_and_succeed_if_exists(
        handler: BinaryHandlerWithTimer,
    ) {
        let key = Bytes::from("key");
        let header = create_header(network::Command::ReplaceQuiet, &key);
        const FLAGS: u32 = 0xDEAD_BEEF;
        let value = from_string("value");

        let request = decoder::BinaryRequest::ReplaceQuietly(network::ReplaceRequest {
            header,
            flags: FLAGS,
            expiration: 0,
            key: key.clone(),
            value: value.clone(),
        });

        let result = handler.handle_request(request);

        match result {
            Some(resp) => {
                if let encoder::BinaryResponse::Error(response) = resp {
                    assert_eq!(response.header.cas, 0);
                    check_header(
                        &response.header,
                        network::Command::ReplaceQuiet,
                        0,
                        0,
                        0,
                        error::CacheError::NotFound as u16,
                        response.error.len() as u32,
                    );
                } else {
                    unreachable!();
                }
            }
            None => {
                unreachable!()
            }
        }
        let orig_value = from_string("original_value");
        insert_value(&handler, key.clone(), orig_value.clone());
        let inserted_value = get_value(&handler, key.clone()).unwrap();
        assert_eq!(inserted_value, orig_value);

        let request = decoder::BinaryRequest::ReplaceQuietly(network::ReplaceRequest {
            header,
            flags: FLAGS,
            expiration: 0,
            key: key.clone(),
            value: value.clone(),
        });

        let result = handler.handle_request(request);
        match result {
            Some(_resp) => {
                unreachable!();
            }
            None => {}
        }
        let replaced_value = get_value(&handler, key.clone()).unwrap();
        assert_eq!(replaced_value, value);
    }

    #[test_case(create_moka_handler() ; "moka_backend")]
    #[test_case(create_dash_map_handler() ; "dash_map_backend")]
    fn append_request_should_succeed_when_value_exists(handler: BinaryHandlerWithTimer) {
        let key = Bytes::from("hello");
        let value = from_string("hello ");
        insert_value(&handler, key.clone(), value.clone());

        let header = create_header(network::Command::Append, &key);
        let request = decoder::BinaryRequest::Append(network::AppendRequest {
            header,
            key: key.clone(),
            value: from_string("world!"),
        });
        let result = handler.handle_request(request);
        match result {
            Some(resp) => {
                if let encoder::BinaryResponse::Append(response) = resp {
                    assert_ne!(response.header.cas, 0);
                    check_header(&response.header, network::Command::Append, 0, 0, 0, 0, 0);
                } else {
                    unreachable!();
                }
            }
            None => unreachable!(),
        }
        let server_value = get_value(&handler, key).unwrap();
        assert_eq!(server_value, from_string("hello world!"));
    }

    #[test_case(create_moka_handler() ; "moka_backend")]
    #[test_case(create_dash_map_handler() ; "dash_map_backend")]
    fn append_quiet_request_should_succeed_when_value_exists(handler: BinaryHandlerWithTimer) {
        let key = Bytes::from("hello");
        let value = from_string("hello ");
        insert_value(&handler, key.clone(), value.clone());

        let header = create_header(network::Command::AppendQuiet, &key);
        let request = decoder::BinaryRequest::AppendQuietly(network::AppendRequest {
            header,
            key: key.clone(),
            value: from_string("world!"),
        });
        let result = handler.handle_request(request);
        match result {
            Some(_resp) => unreachable!(),
            None => {}
        }
        let server_value = get_value(&handler, key).unwrap();
        assert_eq!(server_value, from_string("hello world!"));
    }

    #[test_case(create_moka_handler() ; "moka_backend")]
    #[test_case(create_dash_map_handler() ; "dash_map_backend")]
    fn prepend_request_should_succeed_when_value_exists(handler: BinaryHandlerWithTimer) {
        let key = Bytes::from("hello");
        let value = from_string(" hello");
        insert_value(&handler, key.clone(), value.clone());

        let header = create_header(network::Command::Prepend, &key);
        let request = decoder::BinaryRequest::Prepend(network::PrependRequest {
            header,
            key: key.clone(),
            value: from_string("world!"),
        });
        let result = handler.handle_request(request);
        match result {
            Some(resp) => {
                if let encoder::BinaryResponse::Prepend(response) = resp {
                    assert_ne!(response.header.cas, 0);
                    check_header(&response.header, network::Command::Prepend, 0, 0, 0, 0, 0);
                } else {
                    unreachable!();
                }
            }
            None => unreachable!(),
        }
        let server_value = get_value(&handler, key).unwrap();
        assert_eq!(server_value, from_string("world! hello"));
    }

    #[test_case(create_moka_handler() ; "moka_backend")]
    #[test_case(create_dash_map_handler() ; "dash_map_backend")]
    fn prepend_quiet_request_should_succeed_when_value_exists(handler: BinaryHandlerWithTimer) {
        let key = Bytes::from("hello");
        let value = from_string("hello");
        insert_value(&handler, key.clone(), value.clone());

        let header = create_header(network::Command::PrependQuiet, &key);
        let request = decoder::BinaryRequest::PrependQuietly(network::PrependRequest {
            header,
            key: key.clone(),
            value: from_string("world! "),
        });
        let result = handler.handle_request(request);
        match result {
            Some(_resp) => unreachable!(),
            None => {}
        }
        let server_value = get_value(&handler, key).unwrap();
        assert_eq!(server_value, from_string("world! hello"));
    }
}
