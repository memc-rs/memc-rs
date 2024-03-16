procspawn::enable_test_support!();
mod common;

#[test]
fn set_get_check() {
    let _server_handle = common::spawn_server();
    let client =
        memcache::connect("memcache://127.0.0.1:11211?timeout=10&tcp_nodelay=true&protocol=binary")
            .unwrap();
    // flush the database
    client.flush().unwrap();

    // set a string value
    client.set("foo", "bar", 0).unwrap();

    // retrieve from memcached:
    let value: Option<String> = client.get("foo").unwrap();
    assert_eq!(value, Some(String::from("bar")));
    assert_eq!(value.unwrap(), "bar");
}
