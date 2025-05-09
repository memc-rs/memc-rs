procspawn::enable_test_support!();
use common::create_value_with_size;
mod common;

#[test]
fn set_item_too_large() {
    let params_builder: common::MemcrsdServerParamsBuilder =
        common::MemcrsdServerParamsBuilder::new();
    let server_handle = common::spawn_server(params_builder);
    let client = memcache::connect(server_handle.get_connection_string()).unwrap();

    // flush the database
    client.flush().unwrap();

    let item_size = 1048565; // 3 characters reserved for key "foo" and binary protocol data
    let value = create_value_with_size(item_size);

    // set a string value
    client.set("foo", &value, 0).unwrap();

    // retrieve from memcached:
    let server_value: Option<String> = client.get("foo").unwrap();

    assert_eq!(server_value.unwrap(), value.clone());

    let item_size_too_large = 1024 + (1024 * 1024);
    let value_too_large = create_value_with_size(item_size_too_large);

    // set a string value
    let result = client.set("foo", &value_too_large, 0);
    match result {
        Ok(_res) => {
            unreachable!();
        }
        Err(err) => match err {
            memcache::MemcacheError::CommandError(cmd) => {
                assert_eq!(cmd, memcache::CommandError::ValueTooLarge);
            }
            _ => {
                assert_eq!(true, false);
            }
        },
    }

    // retrieve from memcached:
    let server_value: Option<String> = client.get("foo").unwrap();
    assert_eq!(server_value.unwrap(), value.clone());
}
