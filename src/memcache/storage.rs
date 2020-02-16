use std::collections::HashMap;
use std::collections::hash_map::DefaultHasher;
use std::hash::Hasher;
use std::sync::Mutex;

#[derive(Clone)]
pub struct ValueHeader {
    pub(crate) timestamp: u64,
    pub(crate) cas: u64,
    pub(crate) flags: u32,
    expiration: u32,
    pub(crate) key: Vec<u8>,
}

#[derive(Clone)]
pub struct ValueData {
    pub(crate) header: ValueHeader,
    pub(crate) value: Vec<u8>
}

#[derive(Clone)]
pub struct ValueCounter {
    pub(crate) header: ValueHeader,
    pub(crate) delta: u64,
    pub(crate) value: u64,
}

#[derive(Clone)]
pub enum Record {
    Value(ValueData),
    Counter(ValueCounter)
}

pub struct Storage {
    memory: Mutex<HashMap<u64, Record>>,
}


impl Storage {
    pub fn new() -> Storage {
        Storage {
            memory: Mutex::new(std::collections::HashMap::new()),        
        }
    }

    pub fn get(&self, key: &Vec<u8>) -> Option<Record> {
        let hash = self.get_hash(key);
        self.get_by_hash(hash)
    }

    fn get_by_hash(&self, hash: u64) -> Option<Record> {
        let mut storage = self.memory.lock().unwrap();
        let value = match storage.get(&hash) {
            Some(record) => {
                if self.check_if_expired(record) {                    
                    None
                }                                
                else {
                    Some(record.clone())
                }                
            },
            None => None    
        };        
        value
    }

    fn check_if_expired(&self, record: &Record) -> bool {
        false
    }

    pub fn set(&self)  {

    }

    fn get_hash(&self, key: &Vec<u8>) -> u64 {
        let mut hasher = DefaultHasher::new();
        hasher.write(key);
        hasher.finish()
    }
}