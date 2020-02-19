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

impl ValueHeader {
    pub fn new(key: Vec<u8>, cas: u64, flags: u32, expiration: u32) -> ValueHeader {
        ValueHeader {
            timestamp: 0,
            cas: cas,
            flags: flags,
            expiration: expiration,
            key: key
        }
    }
}

#[derive(Clone)]
pub struct ValueData {
    pub(crate) header: ValueHeader,
    pub(crate) value: Vec<u8>
}


impl ValueData {
    pub fn new(key: Vec<u8>, value: Vec<u8>, cas: u64, flags: u32, expiration: u32) -> ValueData {
        let header = ValueHeader::new(key, cas, flags, expiration);
        ValueData {
            header: header,
            value: value
        }
    }
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
    memory: Mutex<HashMap<u64, Record>>
}


impl Storage {
    pub fn new() -> Storage {
        Storage {
            memory: Mutex::new(std::collections::HashMap::new())
        }
    }

    pub fn get(&self, key: &Vec<u8>) -> Option<Record> {
        let hash = self.get_hash(key);
        self.get_by_hash(hash)
    }

    fn get_by_hash(&self, hash: u64) -> Option<Record> {
        let storage = self.memory.lock().unwrap();
        let value = match storage.get(&hash) {
            Some(record) => {
                if self.check_if_expired(record) {                    
                    None
                }                             
                else {
                    self.touch(record);
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

    fn touch(&self, record: &Record) {        
        
        
    }

    pub fn set(&self, record: Record)  {
        let header = self.get_header(&record);
        let hash = self.get_hash(&header.key);
        let mut storage = self.memory.lock().unwrap();
        storage.insert(hash, record);        
    }

    fn get_header<'a>(&self, record: &'a Record) -> &'a ValueHeader {
        match(record) {
            Record::Value(data) => &data.header,
            Record::Counter(counter) => &counter.header
        }
    }

    fn get_hash(&self, key: &Vec<u8>) -> u64 {
        let mut hasher = DefaultHasher::new();
        hasher.write(key);
        hasher.finish()
    }
}