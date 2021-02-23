use memcache::{MemcacheError};
use std::collections::HashMap;
use std::str;

fn main() {
    let client = memcache::Client::connect(
        "memcache://127.0.0.1:11211?timeout=120&tcp_nodelay=true&protocol=binary",
    )
    .unwrap();
    client.set("foo", "test", 600).unwrap();

    let result: HashMap<String, (Vec<u8>, u32, Option<u64>)> = client.gets(&["foo"]).unwrap();

    let (key, val, cas) = result.get("foo").unwrap();

    println!(
        "Foo: {:?} {} {}",
        str::from_utf8(key).unwrap(),
        val,
        cas.unwrap()
    );

    client.append("foo", "bas").unwrap();

    client.prepend("foo", "bis").unwrap();

    let result: HashMap<String, (Vec<u8>, u32, Option<u64>)> = client.gets(&["foo"]).unwrap();
    let (key, val, cas) = result.get("foo").unwrap();

    println!(
        "Foo: {:?} {} {}",
        str::from_utf8(key).unwrap(),
        val,
        cas.unwrap()
    );
    client.replace("foo", "3000", 80).unwrap();

    client.increment("foo", 100).unwrap();

    client.decrement("foo", 50).unwrap();

    let result: Result<Option<String>, MemcacheError> = client.get("foo");
    match result {
        Ok(val) => match val {
            Some(value) => println!("Server returned: {}", value),
            None => println!("Server none"),
        },
        Err(err) => {
            println!("Error returned: {:?}", err);
        }
    }

    client.delete("foo").unwrap();

    client.flush_with_delay(100).unwrap();

    client.flush().unwrap();
    let version = client.version().unwrap();
    println!("Version: {:?}", version);
}
