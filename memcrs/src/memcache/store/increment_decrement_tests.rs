use super::test_utils::*;
use test_case::test_case;

// INCREMENT TESTS

#[test_case(create_moka_server() ; "moka_backend")]
#[test_case(create_dash_map_server() ; "dash_map_backend")]
fn increment_if_counter_doesnt_exists_it_should_created(server: MockServer) {
    const COUNTER_INITIAL_VALUE: u64 = 5;
    let key = Bytes::from("counter1");
    let counter = IncrementParam {
        delta: 0,
        value: COUNTER_INITIAL_VALUE,
    };
    let header = Meta::new(0, 0, 0);
    let result = server.storage.increment(header, key, counter);
    match result {
        Ok(delta_result) => {
            assert_eq!(COUNTER_INITIAL_VALUE, delta_result.value);
        }
        Err(_) => {
            unreachable!();
        }
    }
}

#[test_case(create_moka_server() ; "moka_backend")]
#[test_case(create_dash_map_server() ; "dash_map_backend")]
fn increment_should_fail_on_cas_mismatch(server: MockServer) {
    const COUNTER_INITIAL_VALUE: u64 = 5;
    let key = Bytes::from("counter1");
    let counter = IncrementParam {
        delta: 0,
        value: COUNTER_INITIAL_VALUE,
    };
    let header = Meta::new(0, 0, 0);
    let result = server
        .storage
        .increment(header, key.clone(), counter.clone());
    assert!(result.is_ok());
    let header = Meta::new(result.unwrap().cas + 1, 0, 0);
    let result = server.storage.increment(header, key, counter);
    match result {
        Ok(_) => {
            unreachable!();
        }
        Err(err) => {
            assert_eq!(err, CacheError::KeyExists);
        }
    }
}

#[test_case(create_moka_server() ; "moka_backend")]
#[test_case(create_dash_map_server() ; "dash_map_backend")]
fn increment_if_expire_equals_ffffffff_counter_should_not_be_created(server: MockServer) {
    let key = Bytes::from("counter1");
    let counter = IncrementParam { delta: 0, value: 0 };
    let header = Meta::new(0, 0, 0xffffffff);
    let result = server.storage.increment(header, key, counter);
    match result {
        Ok(_) => {
            unreachable!();
        }
        Err(err) => {
            assert_eq!(err, CacheError::NotFound);
        }
    }
}

#[test_case(create_moka_server() ; "moka_backend")]
#[test_case(create_dash_map_server() ; "dash_map_backend")]
fn increment_value_should_be_incremented(server: MockServer) {
    const DELTA: u64 = 6;
    const EXPECTED_RESULT: u64 = 5 + DELTA;
    let key = Bytes::from("counter1");
    let record = Record::new(from_string("5"), 0, 0, 0);
    let result = server.storage.set(key.clone(), record);
    assert!(result.is_ok());
    let cas = result.unwrap().cas;

    let counter = IncrementParam {
        delta: DELTA,
        value: 0,
    };
    let header = Meta::new(0, 0, 0);
    let result = server.storage.increment(header, key, counter);
    match result {
        Ok(counter_value) => {
            assert_eq!(counter_value.value, EXPECTED_RESULT);
            assert_eq!(counter_value.cas, cas + 1);
        }
        Err(_) => {
            unreachable!();
        }
    }
}

#[test_case(create_moka_server() ; "moka_backend")]
#[test_case(create_dash_map_server() ; "dash_map_backend")]
fn increment_if_value_is_not_number_it_should_be_error(server: MockServer) {
    const DELTA: u64 = 5;
    let key = Bytes::from("counter1");
    let record = Record::new(from_string("asdas5"), 0, 0, 0);
    let result = server.storage.set(key.clone(), record);
    assert!(result.is_ok());

    let counter = IncrementParam {
        delta: DELTA,
        value: 0,
    };
    let header = Meta::new(0, 0, 0);
    let result = server.storage.increment(header, key, counter);
    match result {
        Ok(_) => {
            unreachable!();
        }
        Err(err) => {
            assert_eq!(err, CacheError::ArithOnNonNumeric);
        }
    }
}

#[test_case(create_moka_server() ; "moka_backend")]
#[test_case(create_dash_map_server() ; "dash_map_backend")]
fn increment_if_value_cannot_be_parsed_it_should_be_error(server: MockServer) {
    const DELTA: u64 = 5;
    let key = Bytes::from("counter1");
    let record = Record::new(from_slice(&[0xc3, 0x28]), 0, 0, 0);
    let result = server.storage.set(key.clone(), record);
    assert!(result.is_ok());

    let counter = IncrementParam {
        delta: DELTA,
        value: 0,
    };
    let header = Meta::new(0, 0, 0);
    let result = server.storage.increment(header, key, counter);
    match result {
        Ok(_) => {
            unreachable!();
        }
        Err(err) => {
            assert_eq!(err, CacheError::ArithOnNonNumeric);
        }
    }
}

// DECREMENT TESTS

#[test_case(create_moka_server() ; "moka_backend")]
#[test_case(create_dash_map_server() ; "dash_map_backend")]
fn decrement_should_fail_on_cas_mismatch(server: MockServer) {
    const COUNTER_INITIAL_VALUE: u64 = 5;
    let key = Bytes::from("counter1");
    let counter = DecrementParam {
        delta: 0,
        value: COUNTER_INITIAL_VALUE,
    };
    let header = Meta::new(0, 0, 0);
    let result = server
        .storage
        .decrement(header, key.clone(), counter.clone());
    assert!(result.is_ok());
    let header = Meta::new(result.unwrap().cas + 1, 0, 0);
    let result = server.storage.decrement(header, key, counter);
    match result {
        Ok(_) => {
            unreachable!();
        }
        Err(err) => {
            assert_eq!(err, CacheError::KeyExists);
        }
    }
}

#[test_case(create_moka_server() ; "moka_backend")]
#[test_case(create_dash_map_server() ; "dash_map_backend")]
fn decrement_should_not_result_in_negative_value(server: MockServer) {
    const DELTA: u64 = 1;
    let key = Bytes::from("counter1");
    let record = Record::new(from_string("0"), 0, 0, 0);
    let result = server.storage.set(key.clone(), record);
    assert!(result.is_ok());
    let cas = result.unwrap().cas;

    let counter = IncrementParam {
        delta: DELTA,
        value: 0,
    };
    let header = Meta::new(0, 0, 0);
    let result = server.storage.decrement(header, key, counter);
    match result {
        Ok(counter_value) => {
            assert_eq!(counter_value.value, 0);
            assert_eq!(counter_value.cas, cas + 1);
        }
        Err(_) => {
            unreachable!();
        }
    }
}

#[test_case(create_moka_server() ; "moka_backend")]
#[test_case(create_dash_map_server() ; "dash_map_backend")]
fn decrement_value_should_be_decremented(server: MockServer) {
    const DELTA: u64 = 1;
    const EXPECTED_RESULT: u64 = 4;
    let key = Bytes::from("counter1");
    let record = Record::new(from_string("5"), 0, 0, 0);
    let result = server.storage.set(key.clone(), record);
    assert!(result.is_ok());
    let cas = result.unwrap().cas;

    let counter = IncrementParam {
        delta: DELTA,
        value: 0,
    };
    let header = Meta::new(0, 0, 0);
    let result = server.storage.decrement(header, key, counter);
    match result {
        Ok(counter_value) => {
            assert_eq!(counter_value.value, EXPECTED_RESULT);
            assert_eq!(counter_value.cas, cas + 1);
        }
        Err(_) => {
            unreachable!();
        }
    }
}
