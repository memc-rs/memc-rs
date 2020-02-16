use std::collections::HashMap;

pub struct Value {
    cas: u64,
    flags: u32,
    expiration: u32,
    key: Vec<u8>,
    value: Vec<u8>,
}

pub struct Storage {
    memory: HashMap<u64, Value>,
}


impl Storage {
    pub fn new() -> Storage {
        Storage {
            memory: std::collections::HashMap::new()
        }
    }
}