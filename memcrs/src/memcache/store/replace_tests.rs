use super::test_utils::*;
use test_case::test_case;

#[test_case(create_moka_server() ; "moka_backend")]
#[test_case(create_dash_map_server() ; "dash_map_backend")]
fn replace_should_fail_if_not_stored(server: MockServer) {
    let key = Bytes::from("key");
    let record = Record::new(from_string("test data"), 5, 0, 0);
    let result = server.storage.replace(key, record);
    match result {
        Ok(_) => unreachable!(),
        Err(err) => assert_eq!(err, CacheError::NotFound),
    }
}

#[test_case(create_moka_server() ; "moka_backend")]
#[test_case(create_dash_map_server() ; "dash_map_backend")]
fn replace_should_succeed_if_stored(server: MockServer) {
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

#[test_case(create_moka_server() ; "moka_backend")]
#[test_case(create_dash_map_server() ; "dash_map_backend")]
fn replace_should_fail_on_cas_mismatch(server: MockServer) {
    let key = Bytes::from("key");
    let record = Record::new(from_string("test data"), 0, 0, 0);
    let result = server.storage.set(key.clone(), record);
    assert!(result.is_ok());
    let new_record = Record::new(from_string("New record"), result.unwrap().cas + 1, 0, 0);
    let replace_result = server.storage.replace(key, new_record);
    match replace_result {
        Ok(_) => unreachable!(),
        Err(err) => {
            assert_eq!(err, CacheError::KeyExists);
        }
    }
}
