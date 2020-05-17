use super::error::{StorageError, StorageResult};
use super::timer;
use dashmap::DashMap;
use std::str;
use std::sync::{Arc, RwLock};

#[derive(Clone, Debug)]
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

#[derive(Clone, Debug)]
pub struct Record {
    pub(crate) header: Header,
    pub(crate) value: Vec<u8>,
}

impl Record {
    pub fn new(value: Vec<u8>, cas: u64, flags: u32, expiration: u32) -> Record {
        let header = Header::new(cas, flags, expiration);
        Record { header, value }
    }
}

impl PartialEq for Record {
    fn eq(&self, other: &Self) -> bool {
        self.value == other.value
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
    timer: Arc<dyn timer::Timer + Send + Sync>,
}

pub struct SetStatus {
    pub cas: u64,
}

impl Storage {
    pub fn new(timer: Arc<dyn timer::Timer + Send + Sync>) -> Storage {
        Storage {
            memory: dashmap::DashMap::new(),
            timer,
        }
    }

    pub fn get(&self, key: &Vec<u8>) -> StorageResult<Record> {
        info!("Get: {:?}", str::from_utf8(key));
        self.get_by_key(key)
    }

    fn get_by_key(&self, key: &Vec<u8>) -> StorageResult<Record> {
        match self.memory.get_mut(key) {
            Some(mut record) => {
                if self.check_if_expired(key, &mut record) {
                    Err(StorageError::NotFound)
                } else {
                    Ok(record.clone())
                }
            }
            None => Err(StorageError::NotFound),
        }
    }

    fn check_if_expired(&self, key: &Vec<u8>, record: &mut Record) -> bool {
        let current_time = self.timer.secs();

        if record.header.expiration == 0 {
            return false;
        }

        if record.header.timestamp + (record.header.expiration as u64) > current_time {
            return false;
        }
        self.memory.remove(key);
        true
    }

    fn touch_record(&self, record: &mut Record) {
        let timer = self.timer.secs();        
    }

    pub fn set(&self, key: Vec<u8>, mut record: Record) -> StorageResult<SetStatus> {
        info!("Header: {:?}", &record.header);
        match self.check_cas(&key, &record) {
            Ok(cas) => {
                record.header.cas = cas;
                self.touch_record(&mut record);
                info!("Insert: {:?}, {:?}", &key, &record.header);
                self.memory.insert(key, record);
                Ok(SetStatus { cas })
            }
            Err(err) => Err(err),
        }
    }

    fn check_cas(&self, key: &Vec<u8>, record: &Record) -> StorageResult<u64> {
        if record.header.cas > 0 {
            if let Some(existing_record) = self.memory.get(key) {
                if existing_record.header.cas != record.header.cas {
                    return Err(StorageError::KeyExists);
                }
            }
            return Ok(record.header.cas);
        }
        Ok(1)
    }

    pub fn add(&self, _key: Vec<u8>, _record: Record) {}

    pub fn replace(&self, _key: Vec<u8>, _record: Record) {}

    pub fn append(&self, _key: Vec<u8>, _record: Record) {}

    pub fn prepend(&self, _key: Vec<u8>, _record: Record) {}

    pub fn cas(&self, _key: Vec<u8>, _record: Record) {}

    pub fn increment(&self, _key: Vec<u8>, _increment: IncrementParam) {}

    pub fn decrement(&self, _key: Vec<u8>, _decrement: DecrementParam) {}

    pub fn delete(&self, _key: Vec<u8>, _header: Header) {}

    pub fn flush(&self) {}

    pub fn touch(&self, _key: Vec<u8>) {}
}

#[cfg(test)]
mod tests {
    use super::*;
    use simplelog::{CombinedLogger, Config, LevelFilter, WriteLogger};
    use std::fs::File;

    struct MockSystemTimer {
        current_time: u64
    }

    trait SetableTimer : timer::Timer {
        fn set(&mut self, time: u64);    
    }

    impl MockSystemTimer {
        pub fn new() -> Self {
            MockSystemTimer {
                current_time: 0
            }
        }
    }

    impl timer::Timer for MockSystemTimer {
        fn secs(&self) -> u64 {
            self.current_time
        }
    }

    impl SetableTimer for MockSystemTimer {
        fn set(&mut self, time: u64) {
            self.current_time = time
        }
    }
    
    struct MockServer {
        timer: Arc<dyn SetableTimer + Send + Sync>,
        storage: Storage
    }

    impl MockServer {
        pub fn new() -> Self {
            let timer: Arc<dyn SetableTimer + Send + Sync> = Arc::new(MockSystemTimer::new());
            MockServer {
                timer: timer.clone(),
                storage: Storage::new(timer as Arc<dyn timer::Timer+Send+Sync>)
            }
        }
    }
    fn create_timer() -> Arc<dyn timer::Timer + Send + Sync> {
        Arc::new(MockSystemTimer::new())
    }

    fn create_storage() -> Storage {
        let timer: Arc<dyn timer::Timer + Send + Sync> = create_timer();
        Storage::new(timer)
    }

    

    #[test]
    fn if_not_defined_cas_should_be_1() {
        let storage = create_storage();
        let key = String::from("key").into_bytes();
        let record = Record::new(String::from("Test data").into_bytes(), 0, 0, 0);
        let result = storage.set(key.clone(), record.clone());
        assert!(result.is_ok());
        let found = storage.get(&key);
        assert!(found.is_ok());
        match found {
            Ok(r) => {
                assert_eq!(r, record);
                assert_eq!(r.header.cas, 1)
            }
            Err(_er) => unreachable!(),
        }
    }

    #[test]
    fn if_cas_defined_it_should_be_returned() {
        let storage = create_storage();
        let cas: u64 = 0xDEAD_BEEF;
        let key = String::from("key").into_bytes();
        let record = Record::new(String::from("Test data").into_bytes(), cas, 0, 0);
        info!("Record {:?}", &record.header);
        let result = storage.set(key.clone(), record.clone());
        assert!(result.is_ok());
        let found = storage.get(&key);
        assert!(found.is_ok());
        match found {
            Ok(r) => {
                assert_eq!(r, record);
                assert_eq!(r.header.cas, cas)
            }
            Err(_er) => unreachable!(),
        }
    }

    #[test]
    fn insert_should_fail_on_cas_mismatch() {
        let storage = create_storage();
        let cas: u64 = 0xDEAD_BEEF;
        let key = String::from("key").into_bytes();
        let mut record = Record::new(String::from("Test data").into_bytes(), cas, 0, 0);
        let result = storage.set(key.clone(), record.clone());
        assert!(result.is_ok());
        record.header.cas = 1;
        let result = storage.set(key, record);
        match result {
            Ok(_) => unreachable!(),
            Err(err) => assert_eq!(err, StorageError::KeyExists),
        }
    }
}
