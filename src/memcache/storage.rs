use dashmap::DashMap;
use std::str;
use super::timer;
use std::sync::{Arc, Mutex};
use super::error::{StorageResult};

#[derive(Clone)]
pub struct Header {
    pub(self) timestamp: u64,
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
    timer: Arc<Box<dyn timer::Timer+Send+Sync>>,
}

impl Storage {
    pub fn new(timer: Arc<Box<dyn timer::Timer+Send+Sync>>) -> Storage {
        Storage {
            memory: dashmap::DashMap::new(),
            timer,
        }
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
                    Some(record.clone())
                }
            }
            None => None,
        }
    }

    fn check_if_expired(&self, key: &Vec<u8>, record: &mut Record) -> bool {
        false
    }

    fn touch_record(&self, record: &mut Record) {
        record.header.timestamp = self.timer.secs();
    }

    pub fn set(&self, key: Vec<u8>, mut record: Record) -> StorageResult<()> {
        info!("Insert: {:?}", &key);
        self.touch_record(&mut record);
        self.memory.insert(key, record);
        Ok(())
    }

    pub fn add(&self, key: Vec<u8>, record: Record) {}

    pub fn replace(&self, key: Vec<u8>, record: Record) {}

    pub fn append(&self, key: Vec<u8>, record: Record) {}

    pub fn prepend(&self, key: Vec<u8>, record: Record) {}

    pub fn cas(&self, key: Vec<u8>, record: Record) {}

    pub fn increment(&self, key: Vec<u8>, increment: IncrementParam) {}

    pub fn decrement(&self, key: Vec<u8>, decrement: DecrementParam) {}

    pub fn delete(&self, key: Vec<u8>, header: Header) {}

    pub fn flush(&self) {}

    pub fn touch(&self, key: Vec<u8>) {}


}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn insert() {
        let timer: Arc<Box<dyn timer::Timer+Send+Sync>> = Arc::new(Box::new(timer::SystemTimer::new()));
        let storage = Storage::new(timer);
        let record = Record::new(String::from("Test data").into_bytes(), 0, 0, 0);
        let result = storage.set(String::from("key1").into_bytes(), record);
        assert!(result.is_ok());
    }
}