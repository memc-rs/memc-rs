use dashmap::DashMap;
use std::collections::hash_map::DefaultHasher;
use std::hash::Hasher;
use std::str;

#[derive(Clone)]
pub struct Header {
    pub(crate) timestamp: u64,
    pub(crate) cas: u64,
    pub(crate) flags: u32,
    expiration: u32,
}

impl Header {
    pub fn new(cas: u64, flags: u32, expiration: u32) -> Header {
        Header {
            timestamp: 0,
            cas: cas,
            flags: flags,
            expiration: expiration,
        }
    }
}

#[derive(Clone)]
pub struct Record {
    pub(crate) header: Header,
    pub(crate) value: Vec<u8>,
}

impl Record {
    pub fn new(value: Vec<u8>, cas: u64, flags: u32, expiration: u32) -> Record {
        let header = Header::new(cas, flags, expiration);
        Record {
            header: header,
            value: value,
        }
    }
}

#[derive(Clone)]
pub struct IncrementParam {
    pub(crate) delta: u64,
    pub(crate) value: u64,
}
pub type DecrementParam = IncrementParam;

pub struct Storage {
    memory: dashmap::DashMap<Vec<u8>, Record>,
}

impl Default for Storage {
    fn default() -> Self {
        Storage {
            memory: dashmap::DashMap::new(),
        }
    }
}

impl Storage {
    pub fn new() -> Storage {
        Default::default()
    }

    pub fn get(&self, key: &Vec<u8>) -> Option<Record> {
        info!("Get: {:?}", str::from_utf8(key));
        self.get_by_key(key)
    }

    fn get_by_key(&self, key: &Vec<u8>) -> Option<Record> {
        match self.memory.get_mut(key) {
            Some(mut record) => {
                if self.check_if_expired(key, &mut record) {
                    None
                } else {
                    self.touch(&mut record);
                    Some(record.clone())
                }
            }
            None => None,
        }
    }

    fn check_if_expired(&self, key: &Vec<u8>, record: &mut Record) -> bool {
        false
    }

    fn touch(&self, record: &mut Record) {}

    pub fn set(&self, key: Vec<u8>, record: Record) {
        info!("Insert: {:?}", &key);
        self.memory.insert(key, record);
    }

    pub fn add(&self, key: Vec<u8>, record: Record) {}

    pub fn replace(&self, key: Vec<u8>, record: Record) {}

    pub fn append(&self, key: Vec<u8>, record: Record) {}

    pub fn prepend(&self, key: Vec<u8>, record: Record) {}

    pub fn cas(&self, key: Vec<u8>, record: Record) {}

    pub fn increment(&self, key: Vec<u8>, increment: IncrementParam) {}

    pub fn decrement(&self, key: Vec<u8>, decrement: DecrementParam) {}
}
