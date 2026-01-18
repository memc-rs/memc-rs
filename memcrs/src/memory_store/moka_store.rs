use crate::cache::cache::{
    Cache, CacheMetaData, DeltaParam, DeltaResult, KeyType, Record, SetStatus,
};
use crate::cache::error::{CacheError, Result};
use crate::memory_store::shared_store_state::SharedStoreState;
use crate::protocol::binary::network::DELTA_NO_INITIAL_VALUE;
use crate::server::timer;
use bytes::{Bytes, BytesMut};
use moka::ops::compute::Op;
use moka::sync::Cache as MokaCache;
use std::sync::Arc;

type MokaStorage = MokaCache<KeyType, Record>;

pub struct MokaMemoryStore {
    memory: MokaStorage,
    store_state: SharedStoreState,
}

impl MokaMemoryStore {
    pub fn new(timer: Arc<dyn timer::Timer + Send + Sync>, max_capacity: u64) -> MokaMemoryStore {
        let store_state = SharedStoreState::new(timer.clone());
        MokaMemoryStore {
            memory: MokaCache::new(max_capacity),
            store_state,
        }
    }

    fn append_prepend_common(
        &self,
        key: KeyType,
        mut new_record: Record,
        is_append: bool,
    ) -> Result<SetStatus> {
        let mut result: Result<SetStatus> = Err(CacheError::NotFound);
        let _entry = self
            .memory
            .entry(key)
            .and_compute_with(|maybe_entry| match maybe_entry {
                Some(entry) => {
                    let prev_record = entry.into_value();
                    if SharedStoreState::cas_mismatch(&new_record, prev_record.header.cas) {
                        result = Err(CacheError::KeyExists);
                        Op::Nop
                    } else {
                        self.store_state.set_cas_ttl(&mut new_record);
                        let mut new_value = BytesMut::with_capacity(
                            prev_record.value.len() + new_record.value.len(),
                        );
                        if is_append {
                            new_value.extend_from_slice(&prev_record.value);
                            new_value.extend_from_slice(&new_record.value);
                        } else {
                            new_value.extend_from_slice(&new_record.value);
                            new_value.extend_from_slice(&prev_record.value);
                        }
                        new_record.value = new_value.freeze();
                        result = Ok(SetStatus {
                            cas: new_record.header.cas,
                        });
                        Op::Put(new_record)
                    }
                }
                None => Op::Nop,
            });
        result
    }
}

impl Cache for MokaMemoryStore {
    /// Returns a value associated with a key
    fn get(&self, key: &KeyType) -> Result<Record> {
        let mut result = Err(CacheError::NotFound);

        let _entry =
            self.memory
                .entry(key.clone())
                .and_compute_with(|maybe_entry| match maybe_entry {
                    Some(record) => {
                        if self.store_state.check_if_expired(key, record.value()) {
                            result = Err(CacheError::NotFound);
                            return Op::Remove;
                        }
                        result = Ok(record.value().clone());
                        Op::Nop
                    }
                    None => {
                        result = Err(CacheError::NotFound);
                        Op::Nop
                    }
                });
        result
    }

    fn set(&self, key: KeyType, mut record: Record) -> Result<SetStatus> {
        //trace!("Set: {:?}", &record.header);
        let mut result: Result<SetStatus> = Err(CacheError::KeyExists);
        let _entry = self.memory.entry(key).and_compute_with(|maybe_entry| {
            if let Some(entry) = maybe_entry {
                let key_value = entry.into_value();
                if SharedStoreState::cas_mismatch(&record, key_value.header.cas) {
                    return Op::Nop;
                }
            }
            let cas = self.store_state.set_cas_ttl(&mut record);
            result = Ok(SetStatus { cas });
            Op::Put(record)
        });
        result
    }

    fn delete(&self, key: KeyType, header: CacheMetaData) -> Result<Record> {
        let mut result: Result<Record> = Err(CacheError::NotFound);
        let _entry = self.memory.entry(key).and_compute_with(|maybe_entry| {
            if let Some(entry) = maybe_entry {
                let record = entry.into_value();
                let should_remove = header.cas == 0 || record.header.cas == header.cas;
                if should_remove {
                    result = Ok(record);
                    return Op::Remove;
                }
                result = Err(CacheError::KeyExists);
                return Op::Nop;
            }
            Op::Nop
        });
        result
    }

    fn flush(&self, header: CacheMetaData) {
        if header.time_to_live > 0 {
            // FIXME!!!
            // self.memory.iter().for_each(|re| {

            // });
            // self.memory.alter_all(|_key, mut value| {
            //     value.header.time_to_live = header.time_to_live;
            //     value
            // });
        } else {
            self.memory.invalidate_all();
        }
    }

    fn run_pending_tasks(&self) {
        self.memory.run_pending_tasks()
    }

    /// Adds a new key-value pair to the cache, but only if the key does not already exist.
    /// If the key exists, the operation fails with KeyExists error.
    fn add(&self, key: KeyType, mut record: Record) -> Result<SetStatus> {
        let cas = self.store_state.set_cas_ttl(&mut record);
        let entry = self.memory.entry(key).or_insert(record);
        match entry.is_fresh() {
            true => Ok(SetStatus { cas }),
            false => Err(CacheError::KeyExists),
        }
    }

    /// Replaces the value of an existing key in the cache, but only if the key already exists.
    /// If the key does not exist, the operation fails with NotFound error.
    fn replace(&self, key: KeyType, mut record: Record) -> Result<SetStatus> {
        let mut result: Result<SetStatus> = Err(CacheError::NotFound);
        let _entry = self
            .memory
            .entry(key)
            .and_compute_with(|maybe_entry| match maybe_entry {
                Some(entry) => {
                    let cas = entry.into_value().header.cas;
                    if SharedStoreState::cas_mismatch(&record, cas) {
                        result = Err(CacheError::KeyExists);
                        Op::Nop
                    } else {
                        self.store_state.set_cas_ttl(&mut record);
                        result = Ok(SetStatus {
                            cas: record.header.cas,
                        });
                        Op::Put(record)
                    }
                }
                None => Op::Nop,
            });
        result
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
        let mut result: Result<DeltaResult> = Err(CacheError::NotFound);
        let _entry = self
            .memory
            .entry(key)
            .and_compute_with(|maybe_entry| match maybe_entry {
                Some(entry) => {
                    let mut record = entry.into_value();
                    let entry_cas = record.header.cas;
                    let tmp_record = Record::new(Bytes::new(), cas, 0, 0);
                    if SharedStoreState::cas_mismatch(&tmp_record, entry_cas) {
                        result = Err(CacheError::KeyExists);
                        Op::Nop
                    } else {
                        match self.store_state.incr_decr_common(&record, delta, increment) {
                            Ok(new_value) => {
                                let new_cas = self.store_state.get_cas_id();
                                record.value = Bytes::from(new_value.to_string());
                                record.header.cas = new_cas;
                                result = Ok(DeltaResult {
                                    value: new_value,
                                    cas: new_cas,
                                });
                                Op::Put(record)
                            }
                            Err(e) => {
                                result = Err(e);
                                Op::Nop
                            }
                        }
                    }
                }
                None => {
                    if header.get_expiration() != DELTA_NO_INITIAL_VALUE {
                        let cas = self.store_state.get_cas_id();
                        let record = Record::new(
                            Bytes::from(delta.value.to_string()),
                            cas,
                            0,
                            header.get_expiration(),
                        );

                        result = Ok(DeltaResult {
                            cas,
                            value: delta.value,
                        });
                        return Op::Put(record);
                    }
                    Op::Nop
                }
            });
        result
    }
}
