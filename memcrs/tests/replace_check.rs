use memcache::MemcacheError;
//procspawn::enable_test_support!();
mod common;
use memcrs::memory_store::StoreEngine;
use test_case::test_case;

#[test_case(common::create_moka_engine() ; "moka_backend")]
#[test_case(common::create_dashmap_engine() ; "dash_map_backend")]
fn replace_check(engine: StoreEngine) {
    let params_builder: common::MemcrsdServerParamsBuilder =
        common::MemcrsdServerParamsBuilder::new(engine);
    let server_handle = common::spawn_server(params_builder);
    let client = memcache::connect(server_handle.get_connection_string()).unwrap();
    // flush the database
    client.flush().unwrap();

    // set a string value
    client.set("foo", "bar", 0).unwrap();

    // retrieve from memcached:
    let value: Option<String> = client.get("foo").unwrap();
    assert_eq!(value.unwrap(), "bar");

    // replace original value
    client.replace("foo", "foobar", 0).unwrap();

    let value: Option<String> = client.get("foo").unwrap();
    assert_eq!(value.unwrap(), "foobar");

    let result: Result<(), MemcacheError> = client.replace("baz", "foo", 0);
    match result {
        Ok(_res) => {
            unreachable!();
        }
        Err(err) => match err {
            memcache::MemcacheError::CommandError(cmd) => {
                assert_eq!(cmd, memcache::CommandError::KeyNotFound);
            }
            _ => {
                unreachable!();
            }
        },
    }
}
