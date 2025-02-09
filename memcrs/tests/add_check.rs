use memcache::MemcacheError;

procspawn::enable_test_support!();
mod common;

#[test]
fn replace_check() {
    let params_builder: common::MemcrsdServerParamsBuilder =
        common::MemcrsdServerParamsBuilder::new();
    let server_handle = common::spawn_server(params_builder);
    let client = memcache::connect(server_handle.get_connection_string()).unwrap();
    // flush the database
    client.flush().unwrap();

    // add value
    client.add("foo", "foobar", 0).unwrap();

    let value: Option<String> = client.get("foo").unwrap();
    assert_eq!(value.unwrap(), "foobar");

    let result: Result<(), MemcacheError> = client.add("foo", "baz", 0);
    match result {
        Ok(_res) => {
            unreachable!();
        }
        Err(err) => match err {
            memcache::MemcacheError::CommandError(cmd) => {
                assert_eq!(cmd, memcache::CommandError::KeyExists);
            }
            _ => {
                unreachable!();
            }
        },
    }
}
