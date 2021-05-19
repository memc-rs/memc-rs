use super::error::{StorageError, StorageResult};
use super::timer;
use bytes::Bytes;
use dashmap::DashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
#[derive(Clone, Debug)]
pub struct Meta {
    pub(self) timestamp: u64,
    pub(crate) cas: u64,
    pub(crate) flags: u32,
    pub(self) expiration: u32,
}

impl Meta {
    pub fn new(cas: u64, flags: u32, expiration: u32) -> Meta {
        Meta {
            timestamp: 0,
            cas,
            flags,
            expiration,
        }
    }

    pub fn get_expiration(&self) -> u32 {
        self.expiration
    }
}

pub type ValueType = Bytes;

#[derive(Clone, Debug)]
pub struct Record {
    pub(crate) header: Meta,
    pub(crate) value: ValueType,
}

impl Record {
    pub fn new(value: Bytes, cas: u64, flags: u32, expiration: u32) -> Record {
        let header = Meta::new(cas, flags, expiration);
        Record { header, value }
    }
}

impl PartialEq for Record {
    fn eq(&self, other: &Self) -> bool {
        self.value == other.value
    }
}

#[derive(Debug)]
pub struct SetStatus {
    pub cas: u64,
}

pub type KeyType = Vec<u8>;

pub trait KVStore {
    fn get(&self, key: &KeyType) -> StorageResult<Record>;
    fn set(&self, key: KeyType, record: Record) -> StorageResult<SetStatus>;
    fn delete(&self, key: KeyType, header: Meta) -> StorageResult<()>;
    fn flush(&self, header: Meta);
}

pub struct KeyValueStore {
    memory: DashMap<KeyType, Record>,
    timer: Arc<dyn timer::Timer + Send + Sync>,
    cas_id: AtomicU64,
}



impl KeyValueStore {
    pub fn new(timer: Arc<dyn timer::Timer + Send + Sync>) -> KeyValueStore {
        KeyValueStore {
            memory: DashMap::new(),
            timer,
            cas_id: AtomicU64::new(1),
        }
    }

    fn get_by_key(&self, key: &KeyType) -> StorageResult<Record> {
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

    fn check_if_expired(&self, key: &KeyType, record: &Record) -> bool {
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

    fn get_cas_id(&self) -> u64 {
        self.cas_id.fetch_add(1, Ordering::SeqCst) as u64
    }
}

impl KVStore for KeyValueStore {


    fn get(&self, key: &KeyType) -> StorageResult<Record> {
        //trace!("Get: {:?}", str::from_utf8(key));
        self.get_by_key(key)
    }

    
    fn set(&self, key: KeyType, mut record: Record) -> StorageResult<SetStatus> {
        //trace!("Set: {:?}", &record.header);

        if record.header.cas > 0 {
            match self.memory.get_mut(&key) {
                Some(mut key_value) => {
                    if key_value.header.cas != record.header.cas {
                        Err(StorageError::KeyExists)
                    } else {
                        record.header.cas += 1;
                        record.header.timestamp = self.timer.secs();
                        let cas = record.header.cas;
                        *key_value = record;
                        Ok(SetStatus { cas })
                    }
                }
                None => {
                    record.header.cas += 1;
                    record.header.timestamp = self.timer.secs();
                    let cas = record.header.cas;
                    self.memory.insert(key, record);
                    Ok(SetStatus { cas })
                }
            }
        } else {
            let cas = self.get_cas_id();
            record.header.cas = cas;
            record.header.timestamp = self.timer.secs();
            self.memory.insert(key, record);
            Ok(SetStatus { cas })
        }
    }



    fn delete(&self, key: KeyType, header: Meta) -> StorageResult<()> {
        let mut cas_match: Option<bool> = None;
        match self.memory.remove_if(&key, |_key, record| -> bool {
            let result = header.cas == 0 || record.header.cas == header.cas;
            cas_match = Some(result);
            result
        }) {
            Some(_key_value) => Ok(()),
            None => match cas_match {
                Some(_value) => Err(StorageError::KeyExists),
                None => Err(StorageError::NotFound),
            },
        }
    }

    fn flush(&self, header: Meta) {
        if header.expiration > 0 {
            self.memory.alter_all(|_key, mut value| {
                value.header.expiration = header.expiration;
                value
            });
        } else {
            self.memory.clear();
        }
    }
}
