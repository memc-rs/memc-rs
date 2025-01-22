use super::*;
use crate::mock::mock_server::{create_server, SetableTimer};
use crate::mock::value::{from_slice, from_string};
use bytes::{BufMut, BytesMut};

#[test]
fn if_not_defined_cas_should_be_1() {
    let server = create_server();

    let key = Bytes::from("key");
    let record = Record::new(from_string("Test data"), 0, 0, 0);
    let result: std::result::Result<CacheSetStatus, CacheError> = server.storage.set(key.clone(), record.clone());
    assert!(result.is_ok());
    let found = server.storage.get(&key);
    assert!(found.is_ok());
    match found {
        Ok(r) => {
            assert_eq!(r, record);
            assert_eq!(r.header.cas, 1)
        }
        Err(_er) => unreachable!(),
    }
}

#[test]
fn should_override_value_if_cas_is_0() {
    let server = create_server();

    let key = Bytes::from("key");
    let record = Record::new(from_string("Test data"), 0, 0, 0);
    let result = server.storage.set(key.clone(), record.clone());
    assert!(result.is_ok());

    let new_record = Record::new(from_string("new test data"), 0, 0, 0);
    let result = server.storage.set(key.clone(), new_record.clone());
    assert!(result.is_ok());
    let found = server.storage.get(&key);

    assert!(found.is_ok());
    match found {
        Ok(r) => {
            assert_eq!(r, new_record);
        }
        Err(_er) => unreachable!(),
    }
}

#[test]
fn if_cas_defined_it_should_be_returned() {
    let storage = create_server().storage;
    let cas: u64 = 0xDEAD_BEEF;
    let key = Bytes::from("key");
    let record = Record::new(from_string("test data"), cas, 0, 0);
    info!("Record {:?}", &record.header);
    let result = storage.set(key.clone(), record.clone());
    assert!(result.is_ok());
    let found = storage.get(&key);
    assert!(found.is_ok());
    match found {
        Ok(r) => {
            assert_eq!(r, record);
            assert_eq!(r.header.cas, cas + 1)
        }
        Err(_er) => unreachable!(),
    }
}

#[test]
fn insert_should_fail_on_cas_mismatch() {
    let storage = create_server().storage;
    let cas: u64 = 0xDEAD_BEEF;
    let key = Bytes::from("key");
    let mut record = Record::new(from_string("test data"), cas, 0, 0);
    let result = storage.set(key.clone(), record.clone());
    assert!(result.is_ok());
    record.header.cas = 1;
    let result = storage.set(key, record);
    match result {
        Ok(_) => unreachable!(),
        Err(err) => assert_eq!(err, CacheError::KeyExists),
    }
}

#[test]
fn record_should_expire_in_given_time() {
    let server = create_server();
    let cas: u64 = 0xDEAD_BEEF;
    let key = Bytes::from("key");
    let record = Record::new(from_string("test data"), cas, 0, 123);
    let result = server.storage.set(key.clone(), record);
    assert!(result.is_ok());
    println!("{:?}", result);
    let found = server.storage.get(&key);
    assert!(found.is_ok());

    server.timer.set(123);
    let found = server.storage.get(&key);
    assert!(found.is_err());
    match found {
        Ok(_r) => unreachable!(),
        Err(err) => assert_eq!(err, CacheError::NotFound),
    }
}

#[test]
fn delete_record() {
    let server = create_server();
    let key = Bytes::from("key");
    let record = Record::new(from_string("test data"), 0, 0, 0);
    let result = server.storage.set(key.clone(), record);
    assert!(result.is_ok());
    let found = server.storage.get(&key);
    assert!(found.is_ok());
    let header = Meta::new(0, 0, 0);
    let deleted = server.storage.delete(key.clone(), header);
    match deleted {
        Ok(_) => match server.storage.get(&key) {
            Ok(_) => unreachable!(),
            Err(err) => assert_eq!(err, CacheError::NotFound),
        },
        Err(_err) => unreachable!(),
    }
}

#[test]
fn delete_should_return_not_exists() {
    let server = create_server();
    let key = Bytes::from("key");
    let record = Record::new(from_string("test data"), 0, 0, 0);
    let result = server.storage.set(key.clone(), record);
    assert!(result.is_ok());
    let found = server.storage.get(&key);
    assert!(found.is_ok());
    let header = Meta::new(0, 0, 0);
    let deleted = server.storage.delete(Bytes::from("bad key"), header);
    match deleted {
        Ok(_) => unreachable!(),
        Err(err) => assert_eq!(err, CacheError::NotFound),
    }
}

#[test]
fn delete_if_cas_doesnt_match_should_not_delete() {
    let server = create_server();
    let key = Bytes::from("key");
    let record = Record::new(from_string("test data"), 1, 0, 0);
    let result = server.storage.set(key.clone(), record);
    assert!(result.is_ok());
    let found = server.storage.get(&key);
    assert!(found.is_ok());
    let header = Meta::new(6, 0, 0);
    let deleted = server.storage.delete(Bytes::from("key"), header);
    match deleted {
        Ok(_) => unreachable!(),
        Err(err) => assert_eq!(err, CacheError::KeyExists),
    }
}

#[test]
fn delete_if_cas_match_should_succeed() {
    let server = create_server();
    let key = Bytes::from("key");
    let record = Record::new(from_string("test data"), 5, 0, 0);
    let result = server.storage.set(key.clone(), record);
    assert!(result.is_ok());
    let found = server.storage.get(&key);
    assert!(found.is_ok());
    let header = Meta::new(found.unwrap().header.cas, 0, 0);
    let deleted = server.storage.delete(Bytes::from("key"), header);
    assert!(deleted.is_ok());
}

#[test]
fn flush_should_remove_all_elements_in_cache() {
    let server = create_server();
    for key_suffix in 1..10 {
        let mut key_str = BytesMut::from("key");
        key_str.reserve(8);
        key_str.put_slice(&key_suffix.to_string().as_bytes());
        let key = key_str.freeze();
        let record = Record::new(from_string("test data"), 0, 0, 5);
        let result = server.storage.set(key.clone(), record);
        assert!(result.is_ok());
    }

    server.storage.flush(Meta::new(0, 0, 3));
    server.timer.set(10);

    for key_suffix in 1..10 {
        let mut key_str = BytesMut::from("key");
        key_str.reserve(8);
        key_str.put_slice(&key_suffix.to_string().as_bytes());
        let result = server.storage.get(&key_str.freeze());
        match result {
            Ok(_) => unreachable!(),
            Err(err) => assert_eq!(err, CacheError::NotFound),
        }
    }
}

#[test]
fn add_should_succeed_if_not_already_stored() {
    let server = create_server();
    let key = Bytes::from("key");
    let record = Record::new(from_string("test data"), 5, 0, 0);
    let result = server.storage.add(key, record);
    assert!(result.is_ok());
}

#[test]
fn add_should_fail_if_already_stored() {
    let server = create_server();
    let key = Bytes::from("key");
    let record = Record::new(from_string("test data"), 5, 0, 0);
    let result = server.storage.set(key.clone(), record.clone());
    assert!(result.is_ok());
    let add_result = server.storage.add(key, record);
    match add_result {
        Ok(_) => unreachable!(),
        Err(err) => assert_eq!(err, CacheError::KeyExists),
    }
}

#[test]
fn replace_should_fail_if_not_stored() {
    let server = create_server();
    let key = Bytes::from("key");
    let record = Record::new(from_string("test data"), 5, 0, 0);
    let result = server.storage.replace(key, record);
    match result {
        Ok(_) => unreachable!(),
        Err(err) => assert_eq!(err, CacheError::NotFound),
    }
}

#[test]
fn replace_should_succeed_if_stored() {
    let server = create_server();
    let key = Bytes::from("key");
    let record = Record::new(from_string("test data"), 0, 0, 0);
    let result = server.storage.set(key.clone(), record);
    assert!(result.is_ok());
    match result {
        Ok(status) => {
            let new_record = Record::new(from_string("New record"), status.cas, 0, 0);
            let replace_result = server.storage.replace(key, new_record);
            assert!(replace_result.is_ok());
        }
        Err(_) => unreachable!(),
    }
}

#[test]
fn append_should_fail_if_not_exist() {
    let server = create_server();
    let key = Bytes::from("key");
    let record = Record::new(from_string("test data"), 0, 0, 0);
    let result = server.storage.append(key, record);

    match result {
        Ok(_) => unreachable!(),
        Err(err) => assert_eq!(err, CacheError::NotFound),
    }
}

#[test]
fn prepend_should_fail_if_not_exist() {
    let server = create_server();
    let key = Bytes::from("key");
    let record = Record::new(from_string("test data"), 0, 0, 0);
    let result = server.storage.prepend(key, record);

    match result {
        Ok(_) => unreachable!(),
        Err(err) => assert_eq!(err, CacheError::NotFound),
    }
}

#[test]
fn append_should_add_at_the_end() {
    let server = create_server();
    let key = Bytes::from("key");
    let record = Record::new(from_string("Foo"), 0, 0, 0);
    let result = server.storage.set(key.clone(), record);

    match result {
        Ok(status) => {
            let append_data = Record::new(from_string("bar"), status.cas, 0, 0);
            let append_result = server.storage.append(key.clone(), append_data);
            assert!(append_result.is_ok());
            let get_result = server.storage.get(&key);
            match get_result {
                Ok(record) => {
                    let value = from_string("Foobar");
                    assert_eq!(record.value[..], value[..]);
                }
                Err(_) => unreachable!(),
            }
        }
        Err(_) => unreachable!(),
    }
}

#[test]
fn prepend_should_add_at_the_begining() {
    let server = create_server();
    let key = Bytes::from("key");
    let record = Record::new(from_string("Foo"), 0, 0, 0);
    let result = server.storage.set(key.clone(), record);

    match result {
        Ok(status) => {
            let append_data = Record::new(from_string("bar"), status.cas, 0, 0);
            let append_result = server.storage.prepend(key.clone(), append_data);
            assert!(append_result.is_ok());
            let get_result = server.storage.get(&key);
            match get_result {
                Ok(record) => {
                    let value = from_string("barFoo");
                    assert_eq!(record.value[..], value[..]);
                }
                Err(_) => unreachable!(),
            }
        }
        Err(_) => unreachable!(),
    }
}

#[test]
fn increment_if_counter_doesnt_exists_it_should_created() {
    const COUNTER_INITIAL_VALUE: u64 = 5;
    let server = create_server();
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

#[test]
fn increment_if_expire_equals_ffffffff_counter_should_not_be_created() {
    let server = create_server();
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

#[test]
fn increment_value_should_be_incremented() {
    const DELTA: u64 = 6;
    const EXPECTED_RESULT: u64 = 5 + DELTA;
    let server = create_server();
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

#[test]
fn increment_if_value_is_not_number_it_should_be_error() {
    const DELTA: u64 = 5;
    let server = create_server();
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

#[test]
fn increment_if_value_cannot_be_parsed_it_should_be_error() {
    const DELTA: u64 = 5;
    let server = create_server();
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

#[test]
fn decrement_should_not_result_in_negative_value() {
    const DELTA: u64 = 1;
    let server = create_server();
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

#[test]
fn decrement_value_should_be_decremented() {
    const DELTA: u64 = 1;
    const EXPECTED_RESULT: u64 = 4;
    let server = create_server();
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
