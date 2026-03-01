use crate::cache::cache::{
    Cache, CacheMetaData, DeltaParam, DeltaResult, KeyType, Record, SetStatus,
};
use crate::cache::error::{CacheError, Result};
use crate::memcache::cli::parser::DashMapConfig;
use crate::memory_store::parallelism::get_number_of_shards;
use crate::memory_store::shared_store_state::SharedStoreState;
use crate::protocol::binary::network::DELTA_NO_INITIAL_VALUE;
use crate::server::timer;

use bytes::{Bytes, BytesMut};
use dashmap::DashMap;
use std::sync::Arc;

type Storage = DashMap<KeyType, Record>;
pub struct DashMapMemoryStore {
    memory: Storage,
    store_state: SharedStoreState,
}

impl DashMapMemoryStore {
    pub fn new(timer: Arc<dyn timer::Timer + Send + Sync>, _cfg: DashMapConfig) -> DashMapMemoryStore {
        let parallelism = std::thread::available_parallelism().map_or(1, usize::from);
        let shards = get_number_of_shards(parallelism);
        info!("Number of shards: {}", shards);
        let store_state = SharedStoreState::new(timer.clone());
        DashMapMemoryStore {
            memory: DashMap::with_shard_amount(shards),
            store_state,
        }
    }

    fn append_prepend_common(
        &self,
        key: KeyType,
        mut new_record: Record,
        is_append: bool,
    ) -> Result<SetStatus> {
        let cas = new_record.header.cas;
        let new_cas = self.store_state.set_cas_ttl(&mut new_record);
        match self.memory.entry(key) {
            dashmap::mapref::entry::Entry::Occupied(mut entry) => {
                let prev_record = entry.get();
                if cas != 0 && prev_record.header.cas != cas {
                    return Err(CacheError::KeyExists);
                }
                let mut new_value =
                    BytesMut::with_capacity(prev_record.value.len() + new_record.value.len());
                if is_append {
                    new_value.extend_from_slice(&prev_record.value);
                    new_value.extend_from_slice(&new_record.value);
                } else {
                    new_value.extend_from_slice(&new_record.value);
                    new_value.extend_from_slice(&prev_record.value);
                }
                new_record.value = new_value.freeze();
                entry.insert(new_record);
                Ok(SetStatus { cas: new_cas })
            }
            dashmap::mapref::entry::Entry::Vacant(_) => Err(CacheError::NotFound),
        }
    }
}

impl Cache for DashMapMemoryStore {
    /// Returns a value associated with a key
    fn get(&self, key: &KeyType) -> Result<Record> {
        match self.memory.entry(key.clone()) {
            dashmap::mapref::entry::Entry::Occupied(entry) => {
                let value = entry.get();
                if self.store_state.check_if_expired(key, value) {
                    entry.remove();
                    return Err(CacheError::NotFound);
                }
                Ok(value.clone())
            }
            dashmap::mapref::entry::Entry::Vacant(_) => Err(CacheError::NotFound),
        }
    }

    fn set(&self, key: KeyType, mut record: Record) -> Result<SetStatus> {
        match self.memory.entry(key) {
            dashmap::mapref::entry::Entry::Occupied(mut entry) => {
                let cas = entry.get().header.cas;
                if SharedStoreState::cas_mismatch(&record, cas) {
                    return Err(CacheError::KeyExists);
                }
                let cas = self.store_state.set_cas_ttl(&mut record);
                entry.insert(record);
                Ok(SetStatus { cas })
            }
            dashmap::mapref::entry::Entry::Vacant(entry) => {
                let cas = self.store_state.set_cas_ttl(&mut record);
                entry.insert(record);
                Ok(SetStatus { cas })
            }
        }
    }

    fn delete(&self, key: KeyType, header: CacheMetaData) -> Result<Record> {
        let mut cas_match: Option<bool> = None;
        match self.memory.remove_if(&key, |_key, record| -> bool {
            let result = header.cas == 0 || record.header.cas == header.cas;
            cas_match = Some(result);
            result
        }) {
            Some(key_value) => Ok(key_value.1),
            None => match cas_match {
                Some(_value) => Err(CacheError::KeyExists),
                None => Err(CacheError::NotFound),
            },
        }
    }

    fn flush(&self, header: CacheMetaData) {
        if header.time_to_live > 0 {
            self.memory.alter_all(|_key, mut value| {
                value.header.time_to_live = header.time_to_live;
                value
            });
        } else {
            self.memory.clear();
        }
    }

    fn run_pending_tasks(&self) {}

    /// Adds a new key-value pair to the cache, but only if the key does not already exist.
    /// If the key exists, the operation fails with KeyExists error.
    fn add(&self, key: KeyType, mut record: Record) -> Result<SetStatus> {
        let cas = self.store_state.set_cas_ttl(&mut record);
        match self.memory.entry(key) {
            dashmap::mapref::entry::Entry::Occupied(_) => Err(CacheError::KeyExists),
            dashmap::mapref::entry::Entry::Vacant(entry) => {
                entry.insert(record);
                Ok(SetStatus { cas })
            }
        }
    }

    /// Replaces the value of an existing key in the cache, but only if the key already exists.
    /// If the key does not exist, the operation fails with NotFound error.
    fn replace(&self, key: KeyType, mut record: Record) -> Result<SetStatus> {
        match self.memory.entry(key) {
            dashmap::mapref::entry::Entry::Occupied(mut entry) => {
                let cas = entry.get().header.cas;
                if SharedStoreState::cas_mismatch(&record, cas) {
                    return Err(CacheError::KeyExists);
                }
                let new_cas = self.store_state.set_cas_ttl(&mut record);
                entry.insert(record);
                Ok(SetStatus { cas: new_cas })
            }
            dashmap::mapref::entry::Entry::Vacant(_) => Err(CacheError::NotFound),
        }
    }

    /// Appends the new value to the existing value for the given key.
    /// The key must already exist in the cache, otherwise the operation fails with NotFound error.
    fn append(&self, key: KeyType, new_record: Record) -> Result<SetStatus> {
        self.append_prepend_common(key, new_record, true)
    }

    /// Prepends the new value to the existing value for the given key.
    /// The key must already exist in the cache, otherwise the operation fails with NotFound error.
    fn prepend(&self, key: KeyType, new_record: Record) -> Result<SetStatus> {
        self.append_prepend_common(key, new_record, false)
    }

    /// Performs an arithmetic operation (increment or decrement) on a numeric value stored in the cache.
    /// If `increment` is true, adds `delta` to the value; otherwise, subtracts `delta`.
    /// The value must be a valid unsigned 64-bit integer.
    /// Returns the new value after the operation.
    fn incr_decr(
        &self,
        header: CacheMetaData,
        key: KeyType,
        delta: DeltaParam,
        increment: bool,
    ) -> Result<DeltaResult> {
        let cas = header.cas;

        match self.memory.entry(key) {
            dashmap::mapref::entry::Entry::Occupied(mut entry) => {
                let record = entry.get_mut();
                match self.store_state.incr_decr_common(record, delta, increment) {
                    Ok(new_value) => {
                        let tmp_record = Record::new(Bytes::new(), cas, 0, 0);
                        if SharedStoreState::cas_mismatch(&tmp_record, record.header.cas) {
                            return Err(CacheError::KeyExists);
                        }
                        let new_cas = self.store_state.set_cas_ttl(record);
                        record.value = Bytes::from(new_value.to_string());
                        record.header.cas = new_cas;
                        Ok(DeltaResult {
                            value: new_value,
                            cas: new_cas,
                        })
                    }
                    Err(e) => Err(e),
                }
            }
            dashmap::mapref::entry::Entry::Vacant(entry) => {
                if header.get_expiration() != DELTA_NO_INITIAL_VALUE {
                    let cas = self.store_state.get_cas_id();
                    let record = Record::new(
                        Bytes::from(delta.value.to_string()),
                        cas,
                        0,
                        header.get_expiration(),
                    );
                    entry.insert(record);
                    return Ok(DeltaResult {
                        cas,
                        value: delta.value,
                    });
                }
                Err(CacheError::NotFound)
            }
        }
    }
}
