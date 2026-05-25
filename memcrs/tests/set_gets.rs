//procspawn::enable_test_support!();
mod common;
use memcrs::memory_store::StoreEngine;
use test_case::test_case;

#[test_case(common::create_moka_engine() ; "moka_backend")]
#[test_case(common::create_dashmap_engine() ; "dash_map_backend")]
fn set_gets_check(engine: StoreEngine) {
    let params_builder: common::MemcrsdServerParamsBuilder =
        common::MemcrsdServerParamsBuilder::new(engine);
    let server_handle = common::spawn_server(params_builder);
    let client = memcache::connect(server_handle.get_connection_string()).unwrap();
    // flush the database
    client.flush().unwrap();

    // set a string value
    client.set("foo1", "bar1", 0).unwrap();
    client.set("foo2", "bar2", 0).unwrap();
    client.set("foo3", "bar3", 0).unwrap();

    // retrieve from memcached:
    let result: std::collections::HashMap<String, String> =
        client.gets(&["foo1", "foo2", "foo3"]).unwrap();
    assert_eq!(result.len(), 3);
    assert_eq!(result["foo1"], "bar1");
    assert_eq!(result["foo2"], "bar2");
    assert_eq!(result["foo3"], "bar3");
}
