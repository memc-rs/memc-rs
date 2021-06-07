use super::error::{StorageError, StorageResult};
use super::timer;
use bytes::Bytes;
use dashmap::mapref::multiple::RefMulti;
use dashmap::{DashMap, ReadOnlyView};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

#[derive(Clone, Debug)]
pub struct Meta {
    pub(self) timestamp: u64,
    pub(crate) cas: u64,
    pub(crate) flags: u32,
    pub(self) time_to_live: u32,
}

impl Meta {
    pub fn new(cas: u64, flags: u32, time_to_live: u32) -> Meta {
        Meta {
            timestamp: 0,
            cas,
            flags,
            time_to_live,
        }
    }

    pub fn get_expiration(&self) -> u32 {
        self.time_to_live
    }

    pub const fn len(&self) -> usize {
        std::mem::size_of::<Meta>()
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

    pub fn len(&self) -> usize {
        self.header.len() + self.value.len()
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

pub trait KVStoreReadOnlyView<'a> {
    fn len(&self) -> usize;
    fn is_empty(&self) -> bool;
    fn keys(&'a self) -> Box<dyn Iterator<Item = &'a KeyType> + 'a>;
}

pub type RemoveIfResult = Vec<Option<(Vec<u8>, Record)>>;
pub type Predicate = dyn FnMut(&KeyType, &Record) -> bool;
pub trait KVStore {
    fn get(&self, key: &KeyType) -> StorageResult<Record>;
    fn set(&self, key: KeyType, record: Record) -> StorageResult<SetStatus>;
    fn delete(&self, key: KeyType, header: Meta) -> StorageResult<Record>;
    fn flush(&self, header: Meta);
    fn len(&self) -> usize;
    fn into_read_only(&self) -> Box<dyn KVStoreReadOnlyView>;
    fn remove_if(
        &self,       
        f: &mut Predicate        
    ) -> RemoveIfResult;
}
        

type Storage = DashMap<KeyType, Record>;
pub struct KeyValueStore {
    memory: Storage,
    timer: Arc<dyn timer::Timer + Send + Sync>,
    cas_id: AtomicU64,
}

type StorageReadOnlyView = ReadOnlyView<KeyType, Record>;

impl<'a> KVStoreReadOnlyView<'a> for StorageReadOnlyView {
    fn len(&self) -> usize {
        StorageReadOnlyView::len(self)
    }

    fn is_empty(&self) -> bool {
        StorageReadOnlyView::is_empty(self)
    }

    fn keys(&'a self) -> Box<dyn Iterator<Item = &'a KeyType> + 'a> {
        let keys = self.keys();
        let result = Box::new(keys);
        result
    }
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

        if record.header.time_to_live == 0 {
            return false;
        }

        if record.header.timestamp + (record.header.time_to_live as u64) > current_time {
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

    fn delete(&self, key: KeyType, header: Meta) -> StorageResult<Record> {
        let mut cas_match: Option<bool> = None;
        match self.memory.remove_if(&key, |_key, record| -> bool {
            let result = header.cas == 0 || record.header.cas == header.cas;
            cas_match = Some(result);
            result
        }) {
            Some(key_value) => Ok(key_value.1),
            None => match cas_match {
                Some(_value) => Err(StorageError::KeyExists),
                None => Err(StorageError::NotFound),
            },
        }
    }

    fn flush(&self, header: Meta) {
        if header.time_to_live > 0 {
            self.memory.alter_all(|_key, mut value| {
                value.header.time_to_live = header.time_to_live;
                value
            });
        } else {
            self.memory.clear();
        }
    }

    fn into_read_only(&self) -> Box<dyn KVStoreReadOnlyView> {
        let storage_clone = self.memory.clone();
        Box::new(storage_clone.into_read_only())
    }

    fn remove_if(
        &self,    
        f: &mut Predicate        
    ) -> RemoveIfResult {
        let items: Vec<KeyType> =
            self.memory
                .iter()
                .filter(|record: &RefMulti<Vec<u8>, Record>| {
                        f(record.key(), record.value())  
                }).map(|record: RefMulti<Vec<u8>, Record>| {
                    record.key().clone()
                }).collect();

        let result: Vec<Option<(Vec<u8>, Record)>> = items
            .iter()
            .map(|key: &KeyType| {
                self.memory.remove(key)
            }).collect();
        result
    }

    fn len(&self) -> usize {
        self.memory.len()
    }
}
