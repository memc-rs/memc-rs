use super::test_utils::*;
use test_case::test_case;

// APPEND TESTS

#[test_case(create_moka_server() ; "moka_backend")]
#[test_case(create_dash_map_server() ; "dash_map_backend")]
fn append_should_fail_if_not_exist(server: MockServer) {
    let key = Bytes::from("key");
    let record = Record::new(from_string("test data"), 0, 0, 0);
    let result = server.storage.append(key, record);

    match result {
        Ok(_) => unreachable!(),
        Err(err) => assert_eq!(err, CacheError::NotFound),
    }
}

#[test_case(create_moka_server() ; "moka_backend")]
#[test_case(create_dash_map_server() ; "dash_map_backend")]
fn append_should_add_at_the_end(server: MockServer) {
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

#[test_case(create_moka_server() ; "moka_backend")]
#[test_case(create_dash_map_server() ; "dash_map_backend")]
fn append_should_fail_on_cas_mismatch(server: MockServer) {
    let key = Bytes::from("key");
    let record = Record::new(from_string("Foo"), 0, 0, 0);
    let result = server.storage.set(key.clone(), record);

    match result {
        Ok(status) => {
            let append_data = Record::new(from_string("bar"), status.cas + 1, 0, 0);
            let append_result = server.storage.append(key.clone(), append_data);
            assert!(append_result.is_err());
            match append_result {
                Ok(_) => {
                    unreachable!();
                }
                Err(err) => {
                    assert_eq!(err, CacheError::KeyExists);
                }
            }
        }
        Err(_) => unreachable!(),
    }
}

// PREPEND TESTS

#[test_case(create_moka_server() ; "moka_backend")]
#[test_case(create_dash_map_server() ; "dash_map_backend")]
fn prepend_should_fail_if_not_exist(server: MockServer) {
    let key = Bytes::from("key");
    let record = Record::new(from_string("test data"), 0, 0, 0);
    let result = server.storage.prepend(key, record);

    match result {
        Ok(_) => unreachable!(),
        Err(err) => assert_eq!(err, CacheError::NotFound),
    }
}

#[test_case(create_moka_server() ; "moka_backend")]
#[test_case(create_dash_map_server() ; "dash_map_backend")]
fn prepend_should_add_at_the_begining(server: MockServer) {
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

#[test_case(create_moka_server() ; "moka_backend")]
#[test_case(create_dash_map_server() ; "dash_map_backend")]
fn prepend_should_fail_on_cas_mismatch(server: MockServer) {
    let key = Bytes::from("key");
    let record = Record::new(from_string("Foo"), 0, 0, 0);
    let result = server.storage.set(key.clone(), record);

    match result {
        Ok(status) => {
            let prepend_data = Record::new(from_string("bar"), status.cas + 1, 0, 0);
            let prepend_result = server.storage.prepend(key.clone(), prepend_data);
            assert!(prepend_result.is_err());
            match prepend_result {
                Ok(_) => {
                    unreachable!();
                }
                Err(err) => {
                    assert_eq!(err, CacheError::KeyExists);
                }
            }
        }
        Err(_) => unreachable!(),
    }
}
