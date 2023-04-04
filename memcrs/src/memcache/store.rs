use bytes::{Bytes, BytesMut};

use crate::storage::error::{StorageError, StorageResult};
use crate::storage::store::{
    KVStore, SetStatus as KVSetStatus,
};

use std::str;
use std::sync::Arc;

pub type SetStatus = KVSetStatus;
pub type ValueType = Bytes;
pub type KeyType = Bytes;

#[derive(Clone)]
pub struct DeltaParam {
    pub(crate) delta: u64,
    pub(crate) value: u64,
}
pub type IncrementParam = DeltaParam;
pub type DecrementParam = IncrementParam;
pub type DeltaResultValueType = u64;
#[derive(Debug)]
pub struct DeltaResult {
    pub cas: u64,
    pub value: DeltaResultValueType,
}

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

    pub const fn is_empty(&self) -> bool {
        self.len() == 0
    }
}


#[derive(Clone, Debug)]
pub struct Record {
    pub(crate) header: Meta,
    pub(crate) value: ValueType,
}

impl Record {
    pub fn new(value: ValueType, cas: u64, flags: u32, expiration: u32) -> Record {
        let header = Meta::new(cas, flags, expiration);
        Record { header, value }
    }

    pub fn len(&self) -> usize {
        self.header.len() + self.value.len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

impl PartialEq for Record {
    fn eq(&self, other: &Self) -> bool {
        self.value == other.value
    }
}

pub trait MemCacheStoreType : KVStore<KeyType, Record> {
}

impl MemCacheStoreType {
    
}
/**
 * Implements Memcache commands based
 * on Key Value Store
 */
pub struct MemcStore {
    store: Arc<dyn MemCacheStoreType + Send + Sync>,
}

impl MemcStore {
    pub fn new(store: Arc<dyn MemCacheStoreType + Send + Sync>) -> MemcStore {
        MemcStore { store }
    }

    pub fn set(&self, key: KeyType, record: Record) -> StorageResult<SetStatus> {
        self.store.set(key, record)
    }

    pub fn get(&self, key: &KeyType) -> StorageResult<Record> {
        self.store.get(key)
    }

    // fn touch_record(&self, _record: &mut Record) {
    //     let _timer = self.timer.secs();
    // }

    pub fn add(&self, key: KeyType, record: Record) -> StorageResult<SetStatus> {
        match self.get(&key) {
            Ok(_record) => Err(StorageError::KeyExists),
            Err(_err) => self.set(key, record),
        }
    }

    pub fn replace(&self, key: KeyType, record: Record) -> StorageResult<SetStatus> {
        match self.get(&key) {
            Ok(_record) => self.set(key, record),
            Err(_err) => Err(StorageError::NotFound),
        }
    }

    pub fn append(&self, key: KeyType, new_record: Record) -> StorageResult<SetStatus> {
        match self.get(&key) {
            Ok(mut record) => {
                record.header.cas = new_record.header.cas;
                let mut value =
                    BytesMut::with_capacity(record.value.len() + new_record.value.len());
                value.extend_from_slice(&record.value);
                value.extend_from_slice(&new_record.value);
                record.value = value.freeze();
                self.set(key, record)
            }
            Err(_err) => Err(StorageError::NotFound),
        }
    }

    pub fn prepend(&self, key: KeyType, new_record: Record) -> StorageResult<SetStatus> {
        match self.get(&key) {
            Ok(mut record) => {
                let mut value =
                    BytesMut::with_capacity(record.value.len() + new_record.value.len());
                value.extend_from_slice(&new_record.value);
                value.extend_from_slice(&record.value);
                record.value = value.freeze();
                record.header.cas = new_record.header.cas;
                self.set(key, record)
            }
            Err(_err) => Err(StorageError::NotFound),
        }
    }

    pub fn increment(
        &self,
        header: Meta,
        key: KeyType,
        increment: IncrementParam,
    ) -> StorageResult<DeltaResult> {
        self.add_delta(header, key, increment, true)
    }

    pub fn decrement(
        &self,
        header: Meta,
        key: KeyType,
        decrement: DecrementParam,
    ) -> StorageResult<DeltaResult> {
        self.add_delta(header, key, decrement, false)
    }

    fn add_delta(
        &self,
        header: Meta,
        key: KeyType,
        delta: DeltaParam,
        increment: bool,
    ) -> StorageResult<DeltaResult> {
        match self.get(&key) {
            Ok(mut record) => {
                str::from_utf8(&record.value)
                    .map(|value: &str| {
                        value
                            .parse::<u64>()
                            .map_err(|_err| StorageError::ArithOnNonNumeric)
                    })
                    .map_err(|_err| StorageError::ArithOnNonNumeric)
                    .and_then(|value: std::result::Result<u64, StorageError>| {
                        //flatten result
                        value
                    })
                    .map(|mut value: u64| {
                        if increment {
                            value += delta.delta;
                        } else if delta.delta > value {
                            value = 0;
                        } else {
                            value -= delta.delta;
                        }
                        record.value = Bytes::from(value.to_string());
                        record.header = header;
                        self.set(key, record).map(|result| DeltaResult {
                            cas: result.cas,
                            value,
                        })
                    })
                    .and_then(|result: std::result::Result<DeltaResult, StorageError>| {
                        //flatten result
                        result
                    })
            }
            Err(_err) => {
                if header.get_expiration() != 0xffffffff {
                    let record = Record::new(
                        Bytes::from(delta.value.to_string()),
                        0,
                        0,
                        header.get_expiration(),
                    );
                    return self.set(key, record).map(|result| DeltaResult {
                        cas: result.cas,
                        value: delta.value,
                    });
                }
                Err(StorageError::NotFound)
            }
        }
    }

    pub fn delete(&self, key: KeyType, header: Meta) -> StorageResult<Record> {
        self.store.delete(key, header.cas)
    }

    pub fn flush(&self, header: Meta) {
        self.store.flush(header.time_to_live)
    }
}

#[cfg(test)]
mod storage_tests;
