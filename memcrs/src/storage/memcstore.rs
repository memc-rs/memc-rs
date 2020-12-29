use bytes::{BytesMut, Bytes};

use super::error::{StorageError, StorageResult};
use super::store::{KVStore, Meta as KVMeta, Record as KVRecord, SetStatus as KVSetStatus};
use super::timer;
use std::str;
use std::sync::Arc;

pub type Record = KVRecord;
pub type Meta = KVMeta;
pub type SetStatus = KVSetStatus;

#[derive(Clone)]
pub struct DeltaParam {
    pub(crate) delta: u64,
    pub(crate) value: u64,
}
pub type IncrementParam = DeltaParam;
pub type DecrementParam = IncrementParam;

#[derive(Debug)]
pub struct DeltaResult {
    pub cas: u64,
    pub value: u64,
}
/**
 * Implements Memcache commands based
 * on Key Value Store
 */
pub struct MemcStore {
    store: KVStore,
}

impl MemcStore {
    pub fn new(timer: Arc<dyn timer::Timer + Send + Sync>) -> MemcStore {
        MemcStore {
            store: KVStore::new(timer),
        }
    }

    pub fn set(&self, key: Vec<u8>, mut record: Record) -> StorageResult<SetStatus> {
        self.store.set(key, record)
    }

    pub fn get(&self, key: &[u8]) -> StorageResult<Record> {
        self.store.get(key)
    }

    fn touch_record(&self, _record: &mut Record) {
        //let _timer = self.timer.secs();
    }

    pub fn add(&self, key: Vec<u8>, record: Record) -> StorageResult<SetStatus> {
        match self.get(&key) {
            Ok(_record) => Err(StorageError::KeyExists),
            Err(_err) => self.store.set(key, record),
        }
    }

    pub fn replace(&self, key: Vec<u8>, record: Record) -> StorageResult<SetStatus> {
        match self.get(&key) {
            Ok(_record) => self.store.set(key, record),
            Err(_err) => Err(StorageError::NotFound),
        }
    }

    pub fn append(&self, key: Vec<u8>, mut new_record: Record) -> StorageResult<SetStatus> {
        match self.get(&key) {
            Ok(mut record) => {
                record.header.cas = new_record.header.cas;
                let mut value = BytesMut::with_capacity(record.value.len()+new_record.value.len());            
                value.extend_from_slice(&record.value);
                value.extend_from_slice(&new_record.value);
                record.value = value.freeze();
                self.store.set(key, record)
            }
            Err(_err) => Err(StorageError::NotFound),
        }
    }

    pub fn prepend(&self, key: Vec<u8>, mut new_record: Record) -> StorageResult<SetStatus> {
        match self.get(&key) {
            Ok(mut record) => {                
                let mut value = BytesMut::with_capacity(record.value.len()+new_record.value.len());
                value.extend_from_slice(&new_record.value);
                value.extend_from_slice(&record.value);                
                record.value = value.freeze();
                record.header.cas = new_record.header.cas;                
                self.store.set(key, record)
            }
            Err(_err) => Err(StorageError::NotFound),
        }
    }

    pub fn increment(
        &self,
        header: Meta,
        key: Vec<u8>,
        increment: IncrementParam,
    ) -> StorageResult<DeltaResult> {
        self.add_delta(header, key, increment, true)
    }

    pub fn decrement(
        &self,
        header: Meta,
        key: Vec<u8>,
        decrement: DecrementParam,
    ) -> StorageResult<DeltaResult> {
        self.add_delta(header, key, decrement, false)
    }

    fn add_delta(
        &self,
        header: Meta,
        key: Vec<u8>,
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
                        self.store.set(key, record).map(|result| DeltaResult {
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
                    return self.store.set(key, record).map(|result| DeltaResult {
                        cas: result.cas,
                        value: delta.value,
                    });
                }
                Err(StorageError::NotFound)
            }
        }
    }

    pub fn delete(&self, key: Vec<u8>, header: Meta) -> StorageResult<()> {
        self.store.delete(key, header)
    }

    pub fn flush(&self, header: Meta) {
        self.store.flush(header)
    }
}

#[cfg(test)]
mod storage_tests;
