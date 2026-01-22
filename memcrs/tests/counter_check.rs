//procspawn::enable_test_support!();
mod common;

#[test]
fn counter_check() {
    let params_builder: common::MemcrsdServerParamsBuilder =
        common::MemcrsdServerParamsBuilder::new();
    let server_handle = common::spawn_server(params_builder);
    let client = memcache::connect(server_handle.get_connection_string()).unwrap();
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
