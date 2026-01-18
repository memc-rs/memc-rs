use super::test_utils::*;
use test_case::test_case;

#[test_case(create_moka_server() ; "moka_backend")]
#[test_case(create_dash_map_server() ; "dash_map_backend")]
fn add_should_succeed_if_not_already_stored(server: MockServer) {
    let key = Bytes::from("key");
    let record = Record::new(from_string("test data"), 5, 0, 0);
    let result = server.storage.add(key, record);
    assert!(result.is_ok());
}

#[test_case(create_moka_server() ; "moka_backend")]
#[test_case(create_dash_map_server() ; "dash_map_backend")]
fn add_should_fail_if_already_stored(server: MockServer) {
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
