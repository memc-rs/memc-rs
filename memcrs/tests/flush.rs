use memcache::MemcacheError;
//procspawn::enable_test_support!();
mod common;
use memcrs::memory_store::StoreEngine;
use test_case::test_case;

#[test_case(common::create_moka_engine() ; "moka_backend")]
#[test_case(common::create_dashmap_engine() ; "dash_map_backend")]
fn flush_check(engine: StoreEngine) {
    let params_builder: common::MemcrsdServerParamsBuilder =
        common::MemcrsdServerParamsBuilder::new(engine);
    let server_handle = common::spawn_server(params_builder);
    let client = memcache::connect(server_handle.get_connection_string()).unwrap();

    // flush the database
    client.flush().unwrap();
    let mut keys = Vec::new();
    for n in 1..11 {
        // set a string value
        let key = format!("foo{}", n);
        let value = format!("bar{}", n);
        client.set(key.as_str(), value, 0).unwrap();
        keys.push(key);
    }

    // flush the database
    client.flush().unwrap();

    for n in 1..11 {
        // set a string value
        let key = format!("foo{}", n);
        let value: Result<Option<String>, MemcacheError> = client.get(key.as_str());
        match value {
            Ok(val) => {
                assert_eq!(val, None);
            }
            Err(_err) => {
                unreachable!();
            }
        }
    }
}
