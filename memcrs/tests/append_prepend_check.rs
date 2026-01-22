//procspawn::enable_test_support!();
mod common;

#[test]
fn append_prepend_works() {
    let params_builder: common::MemcrsdServerParamsBuilder =
        common::MemcrsdServerParamsBuilder::new();
    let server_handle = common::spawn_server(params_builder);
    let client = memcache::connect(server_handle.get_connection_string()).unwrap();
    // flush the database
    client.flush().unwrap();

    // set a string value
    client.set("foo", "bar", 0).unwrap();

    // retrieve from memcached:
    let value: Option<String> = client.get("foo").unwrap();
    assert_eq!(value, Some(String::from("bar")));
    assert_eq!(value.unwrap(), "bar");

    // prepend, append:
    client.prepend("foo", "foo").unwrap();
    client.append("foo", "baz").unwrap();
    let value: String = client.get("foo").unwrap().unwrap();
    assert_eq!(value, "foobarbaz");
}
