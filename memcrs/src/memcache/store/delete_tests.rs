use super::test_utils::*;
use test_case::test_case;

#[test_case(create_moka_server() ; "moka_backend")]
#[test_case(create_dash_map_server() ; "dash_map_backend")]
fn delete_record(server: MockServer) {
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

#[test_case(create_moka_server() ; "moka_backend")]
#[test_case(create_dash_map_server() ; "dash_map_backend")]
fn delete_should_return_not_exists(server: MockServer) {
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

#[test_case(create_moka_server() ; "moka_backend")]
#[test_case(create_dash_map_server() ; "dash_map_backend")]
fn delete_if_cas_doesnt_match_should_not_delete(server: MockServer) {
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

#[test_case(create_moka_server() ; "moka_backend")]
#[test_case(create_dash_map_server() ; "dash_map_backend")]
fn delete_if_cas_match_should_succeed(server: MockServer) {
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
