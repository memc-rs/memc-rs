procspawn::enable_test_support!();
use common::create_value_with_size;
mod common;


#[test]
fn max_item_check() {
    let params_builder: common::MemcrsdServerParamsBuilder =
        common::MemcrsdServerParamsBuilder::new();
    let server_handle = common::spawn_server(params_builder);
    let client = memcache::connect(server_handle.get_connection_string()).unwrap();

    // flush the database
    client.flush().unwrap();

    let max_item_size = 1048565; // 3 characters reserved for key "foo" and binary protocol data
    let value = create_value_with_size(max_item_size);

    // set a string value
    client.set("foo", &value, 0).unwrap();

    // retrieve from memcached:
    let server_value: Option<String> = client.get("foo").unwrap();
    assert_eq!(server_value, Some(value.clone()));
    assert_eq!(server_value.unwrap(), value);
}
