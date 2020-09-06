use super::error::{StorageError, StorageResult};
use super::timer;
use dashmap::DashMap;
use std::str;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

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
            cas,
            flags,
            expiration,
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
    cas_id: AtomicU64,
}
#[derive(Debug)]
pub struct SetStatus {
    pub cas: u64,
}

impl Storage {
    pub fn new(timer: Arc<dyn timer::Timer + Send + Sync>) -> Storage {
        Storage {
            memory: dashmap::DashMap::new(),
            timer,
            cas_id: AtomicU64::new(1),
        }
    }

    pub fn get(&self, key: &Vec<u8>) -> StorageResult<Record> {
        info!("Get: {:?}", str::from_utf8(key));
        self.get_by_key(key)
    }

    fn get_by_key(&self, key: &Vec<u8>) -> StorageResult<Record> {
        let result = match self.memory.get(key) {
            Some(record) => Ok(record.clone()),
            None => Err(StorageError::NotFound),
        };

        match result {
            Ok(record) => {
                if self.check_if_expired(key, &record) {
                    return Err(StorageError::NotFound);
                }
                Ok(record)
            }
            Err(err) => Err(err),
        }
    }

    fn check_if_expired(&self, key: &Vec<u8>, record: &Record) -> bool {
        let current_time = self.timer.secs();

        if record.header.expiration == 0 {
            return false;
        }

        if record.header.timestamp + (record.header.expiration as u64) > current_time {
            return false;
        }
        match self.memory.remove(key) {
            Some(_) => true,
            None => true,
        }
    }

    fn touch_record(&self, _record: &mut Record) {
        let _timer = self.timer.secs();
    }

    pub fn set(&self, key: Vec<u8>, mut record: Record) -> StorageResult<SetStatus> {
        info!("Set: {:?}", &record.header);
        match self.check_cas(&key, &record.header) {
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

    fn check_cas(&self, key: &Vec<u8>, header: &Header) -> StorageResult<u64> {
        if header.cas > 0 {
            if let Some(existing_record) = self.memory.get(key) {
                if existing_record.header.cas != header.cas {
                    return Err(StorageError::KeyExists);
                }
            }
            return Ok(header.cas);
        }
        Ok(self.get_cas_id())
    }

    fn get_cas_id(&self) -> u64 {
        self.cas_id.fetch_add(1, Ordering::SeqCst) as u64
    }

    pub fn add(&self, key: Vec<u8>, record: Record) -> StorageResult<SetStatus> {
        match self.get_by_key(&key) {
            Ok(_record) => Err(StorageError::KeyExists),
            Err(_err) => self.set(key, record),
        }
    }

    pub fn replace(&self, key: Vec<u8>, record: Record) -> StorageResult<SetStatus> {
        match self.get_by_key(&key) {
            Ok(_record) => self.set(key, record),
            Err(_err) => Err(StorageError::NotFound),
        }
    }

    pub fn append(&self, key: Vec<u8>, mut new_record: Record) -> StorageResult<SetStatus> {
        match self.get_by_key(&key) {
            Ok(mut record) => {
                record.header = new_record.header;
                record.value.reserve(new_record.value.len());
                record.value.append(&mut new_record.value);
                self.set(key, record)
            }
            Err(_err) => Err(StorageError::NotFound),
        }
    }

    pub fn prepend(&self, key: Vec<u8>, mut new_record: Record) -> StorageResult<SetStatus> {
        match self.get_by_key(&key) {
            Ok(mut record) => {
                new_record.value.reserve(record.value.len());
                new_record.value.append(&mut record.value);
                self.set(key, new_record)
            }
            Err(_err) => Err(StorageError::NotFound),
        }
    }

    pub fn increment(&self, _key: Vec<u8>, _increment: IncrementParam) {}

    pub fn decrement(&self, _key: Vec<u8>, _decrement: DecrementParam) {}

    pub fn delete(&self, key: Vec<u8>, header: Header) -> StorageResult<()> {
        match self.get_by_key(&key) {
            Ok(_record) => match self.check_cas(&key, &header) {
                Ok(_cas) => match self.memory.remove(&key) {
                    Some(_record) => Ok(()),
                    None => Err(StorageError::NotFound),
                },
                Err(err) => Err(err),
            },
            Err(_err) => Err(StorageError::NotFound),
        }
    }

    pub fn flush(&self, header: Header) {
        self.memory.alter_all(|_key, mut value| {
            value.header.expiration = header.expiration;
            value
        });
    }

    pub fn touch(&self, _key: Vec<u8>) {}
}

#[cfg(test)]
mod storage_tests;
