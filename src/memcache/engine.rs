use std::collections::HashMap;

pub struct Value {
    cas: u64,
    flags: u32,
    expiration: u32,
    key: Vec<u8>,
    value: Vec<u8>,
}

pub struct Engine {
    cache: HashMap<u64, Value>,
}


impl Engine {
    pub fn new() -> Engine {
        Engine {
            cache: std::collections::HashMap::new()
        }
    }
}