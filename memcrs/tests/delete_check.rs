procspawn::enable_test_support!();
mod common;

#[test]
fn delete_check() {
    let _server_handle = common::spawn_server();
    let client =
        memcache::connect(common::get_connection_string())
            .unwrap();
    // flush the database
    client.flush().unwrap();

    // set a string value
    client.set("foo", "bar", 0).unwrap();

    // retrieve from memcached:
    let value: Option<String> = client.get("foo").unwrap();
    assert_eq!(value, Some(String::from("bar")));
    assert_eq!(value.unwrap(), "bar");

    match client.delete("foo") {
        Ok(removed) => {
            assert!(removed);
        }
        Err(_err) => {
            unreachable!();
        }
    }

    match client.delete("bar") {
        Ok(removed) => {
            assert!(removed == false);
        }
        Err(_err) => {
            unreachable!()
        }
    }
}
