procspawn::enable_test_support!();
mod common;

#[test]
fn counter_check() {
    let _server_handle = common::spawn_server();
    let client =
        memcache::connect(common::get_connection_string())
            .unwrap();
    // flush the database
    client.flush().unwrap();

    // using counter:
    client.set("counter", 40, 0).unwrap();
    client.increment("counter", 2).unwrap();
    let answer: i32 = client.get("counter").unwrap().unwrap();
    assert_eq!(answer, 42);
    client.decrement("counter", 4).unwrap();
    let answer: i32 = client.get("counter").unwrap().unwrap();
    assert_eq!(answer, 38);
}
