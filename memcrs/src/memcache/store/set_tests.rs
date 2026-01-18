use super::test_utils::*;
use test_case::test_case;

#[test_case(create_moka_server() ; "moka_backend")]
#[test_case(create_dash_map_server() ; "dash_map_backend")]
fn set_if_not_defined_cas_should_be_1(server: MockServer) {
    let key = Bytes::from("key");
    let record = Record::new(from_string("Test data"), 0, 0, 0);
    let result: std::result::Result<CacheSetStatus, CacheError> =
        server.storage.set(key.clone(), record.clone());
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

#[test_case(create_moka_server() ; "moka_backend")]
#[test_case(create_dash_map_server() ; "dash_map_backend")]
fn set_should_override_value_if_cas_is_0(server: MockServer) {
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

#[test_case(create_moka_server() ; "moka_backend")]
#[test_case(create_dash_map_server() ; "dash_map_backend")]
fn set_if_cas_defined_it_should_be_returned(server: MockServer) {
    let storage = server.storage;
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

#[test_case(create_moka_server() ; "moka_backend")]
#[test_case(create_dash_map_server() ; "dash_map_backend")]
fn set_insert_should_fail_on_cas_mismatch(server: MockServer) {
    let storage = server.storage;
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

#[test_case(create_moka_server() ; "moka_backend")]
#[test_case(create_dash_map_server() ; "dash_map_backend")]
fn set_insert_should_not_fail_on_cas_max(server: MockServer) {
    let storage = server.storage;
    let cas: u64 = u64::MAX;
    let key = Bytes::from("key");
    let record = Record::new(from_string("test data"), cas, 0, 0);
    let result = storage.set(key.clone(), record.clone());
    assert!(result.is_ok());

    match result {
        Ok(set_status) => {
            assert!(set_status.cas != cas);
        }
        Err(_err) => unreachable!(),
    }
}

#[test_case(create_moka_server() ; "moka_backend")]
#[test_case(create_dash_map_server() ; "dash_map_backend")]
fn set_record_should_expire_in_given_time(server: MockServer) {
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

#[test_case(create_moka_server() ; "moka_backend")]
#[test_case(create_dash_map_server() ; "dash_map_backend")]
fn set_should_not_fail_on_cas_mismatch(server: MockServer) {
    let cas: u64 = 0xDEAD_BEEF;
    let key = Bytes::from("key");
    let record = Record::new(from_string("test data"), cas, 0, 0);
    let result = server.storage.set(key.clone(), record);
    assert!(result.is_ok());

    let found = server.storage.get(&key);
    assert!(found.is_ok());
    let cas = found.unwrap().header.cas;
    let record = Record::new(from_string("test data 1 "), cas, 0, 0);
    let result = server.storage.set(key.clone(), record);
    assert!(result.is_ok());
    match result {
        Ok(set_status) => {
            assert!(set_status.cas != cas);
        }
        Err(_err) => unreachable!(),
    }
}
