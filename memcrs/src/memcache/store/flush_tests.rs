use super::test_utils::*;
use test_case::test_case;

#[test_case(create_moka_server() ; "moka_backend")]
#[test_case(create_dash_map_server() ; "dash_map_backend")]
fn flush_should_remove_all_elements_in_cache(server: MockServer) {
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
