
use rand::{self, Rng};


pub struct MemcacheValue {
    cas: u64,
    flags: u32,
    expiration: u32,
    key: Vec<u8>,
    value: Vec<u8>
}


pub struct MemcacheServer {
    cache: HashMap<u64, MemcacheValue>
}
