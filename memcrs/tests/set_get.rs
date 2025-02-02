procspawn::enable_test_support!();
use std::time::{Instant};

use common::create_value_with_size;
use memcrs::{mock::key_value::generate_random_with_max_size};
mod common;

#[test]
fn set_get_check() {
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
}

#[test]
fn set_gets_check() {
    let params_builder: common::MemcrsdServerParamsBuilder =
        common::MemcrsdServerParamsBuilder::new();
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

#[test]
fn insert_10k_values() {
    let params_builder: common::MemcrsdServerParamsBuilder =
    common::MemcrsdServerParamsBuilder::new();
    let server_handle = common::spawn_server(params_builder);
    let connection_str = std::sync::Arc::new(std::sync::Mutex::new(server_handle.get_connection_string()));
    let mut thread_handles: Vec<std::thread::JoinHandle<()>> = Vec::new();
    
    for _i in 0..4 {
        let cs = std::sync::Arc::clone(&connection_str);
        let handle = std::thread::spawn(move || {
            let conn_str = cs.lock().unwrap().clone();
            let client = memcache::connect(conn_str).unwrap();

            let key_values = generate_random_with_max_size(10*1000, 200, 1024*5);
            let start = Instant::now();
            key_values.iter().for_each(|key_value| {
                let key = std::str::from_utf8(&key_value.key).unwrap();
                let value = std::str::from_utf8(&key_value.value).unwrap();
                client.set(key, value, 0).unwrap();
            });
            let end = start.elapsed();
            println!("[{:?}]: Time taken: {}", std::thread::current().id(), end.as_millis());
        });
        thread_handles.push(handle);
    }

    thread_handles.into_iter().for_each(|handler| {
        handler.join().unwrap();
    });

}
