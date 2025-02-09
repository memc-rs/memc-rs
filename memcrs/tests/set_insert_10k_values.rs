procspawn::enable_test_support!();
use memcrs::mock::key_value::generate_random_with_max_size;
use std::time::Instant;
mod common;

#[test]
fn insert_10k_values() {
    let params_builder: common::MemcrsdServerParamsBuilder =
        common::MemcrsdServerParamsBuilder::new();
    let server_handle = common::spawn_server(params_builder);
    let connection_str =
        std::sync::Arc::new(std::sync::Mutex::new(server_handle.get_connection_string()));
    let mut thread_handles: Vec<std::thread::JoinHandle<()>> = Vec::new();

    for _i in 0..4 {
        let cs = std::sync::Arc::clone(&connection_str);
        let handle = std::thread::spawn(move || {
            let conn_str = cs.lock().unwrap().clone();
            let client = memcache::connect(conn_str).unwrap();

            let key_values = generate_random_with_max_size(10 * 1000, 200, 1024 * 5);
            let start = Instant::now();
            key_values.iter().for_each(|key_value| {
                let key = std::str::from_utf8(&key_value.key).unwrap();
                let value = std::str::from_utf8(&key_value.value).unwrap();
                client.set(key, value, 0).unwrap();
            });
            let end = start.elapsed();
            println!(
                "[{:?}]: Time taken: {}",
                std::thread::current().id(),
                end.as_millis()
            );
        });
        thread_handles.push(handle);
    }

    thread_handles.into_iter().for_each(|handler| {
        handler.join().unwrap();
    });
}
