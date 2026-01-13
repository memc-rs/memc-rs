use crate::cache::cache::impl_details::CacheImplDetails;
use crate::cache::cache::{
    impl_details, Cache, CacheMetaData, DeltaParam, DeltaResult, KeyType, Record, SetStatus,
};
use crate::cache::error::{CacheError, Result};
use crate::server::timer;
use bytes::{Bytes, BytesMut};
use moka::ops::compute::Op;
use moka::sync::Cache as MokaCache;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

type MokaStorage = MokaCache<KeyType, Record>;

pub struct MokaMemoryStore {
    memory: MokaStorage,
    timer: Arc<dyn timer::Timer + Send + Sync>,
    cas_id: AtomicU64,
}

impl MokaMemoryStore {
    pub fn new(timer: Arc<dyn timer::Timer + Send + Sync>, max_capacity: u64) -> MokaMemoryStore {
        MokaMemoryStore {
            memory: MokaCache::new(max_capacity),
            timer,
            cas_id: AtomicU64::new(1),
        }
    }

    fn get_cas_id(&self) -> u64 {
        self.cas_id.fetch_add(1, Ordering::Release)
    }

    fn set_cas_ttl(&self, record: &mut Record) -> u64 {
        record.header.cas = match record.header.cas {
            0 => self.get_cas_id(),
            _ => record.header.cas.wrapping_add(1),
        };
        let timestamp = self.timer.timestamp();
        if record.header.time_to_live > 0 {
            record.header.time_to_live += timestamp;
        }
        record.header.cas
    }

    fn append_prepend_common(
        &self,
        key: KeyType,
        mut new_record: Record,
        is_append: bool,
    ) -> Result<SetStatus> {
        let cas = new_record.header.cas;
        self.set_cas_ttl(&mut new_record);
        let mut result: Result<SetStatus> = Err(CacheError::NotFound);
        let _entry = self
            .memory
            .entry(key)
            .and_compute_with(|maybe_entry| match maybe_entry {
                Some(entry) => {
                    let prev_record = entry.into_value();
                    if cas != 0 && prev_record.header.cas != cas {
                        result = Err(CacheError::KeyExists);
                        Op::Nop
                    } else {
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

impl impl_details::CacheImplDetails for MokaMemoryStore {
    fn get_by_key(&self, key: &KeyType) -> Result<Record> {
        match self.memory.get(key) {
            Some(record) => Ok(record.clone()),
            None => Err(CacheError::NotFound),
        }
    }

    fn check_if_expired(&self, key: &KeyType, record: &Record) -> bool {
        let current_time = self.timer.timestamp();

        if record.header.time_to_live == 0 {
            return false;
        }

        if record.header.time_to_live > current_time {
            return false;
        }
        match self.remove(key) {
            Some(_) => true,
            None => true,
        }
    }
}

impl Cache for MokaMemoryStore {
    // Removes key value and returns as an option
    fn remove(&self, key: &KeyType) -> Option<(KeyType, Record)> {
        self.memory.remove(key).map(|record| (key.clone(), record))
    }

    fn set(&self, key: KeyType, mut record: Record) -> Result<SetStatus> {
        //trace!("Set: {:?}", &record.header);

        let mut result: Result<SetStatus> = Err(CacheError::KeyExists);
        let _entry = self.memory.entry(key).and_compute_with(|maybe_entry| {
            if let Some(entry) = maybe_entry {
                let key_value = entry.into_value();
                if record.header.cas > 0 && key_value.header.cas != record.header.cas {
                    return Op::Nop;
                }
            }
            let cas = self.set_cas_ttl(&mut record);
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

    fn len(&self) -> usize {
        self.memory.entry_count() as usize
    }

    fn run_pending_tasks(&self) {
        self.memory.run_pending_tasks()
    }

    /// Adds a new key-value pair to the cache, but only if the key does not already exist.
    /// If the key exists, the operation fails with KeyExists error.
    fn add(&self, key: KeyType, mut record: Record) -> Result<SetStatus> {
        let cas = self.set_cas_ttl(&mut record);
        let entry = self.memory.entry(key).or_insert(record);
        match entry.is_fresh() {
            true => Ok(SetStatus { cas }),
            false => Err(CacheError::KeyExists),
        }
    }

    /// Replaces the value of an existing key in the cache, but only if the key already exists.
    /// If the key does not exist, the operation fails with NotFound error.
    fn replace(&self, key: KeyType, mut record: Record) -> Result<SetStatus> {
        let cas = record.header.cas;
        self.set_cas_ttl(&mut record);

        let mut result: Result<SetStatus> = Err(CacheError::NotFound);
        let _entry = self
            .memory
            .entry(key)
            .and_compute_with(|maybe_entry| match maybe_entry {
                Some(entry) => {
                    let prev_record = entry.into_value();
                    if cas != 0 && prev_record.header.cas != cas {
                        result = Err(CacheError::KeyExists);
                        Op::Nop
                    } else {
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
                    if cas != 0 && record.header.cas != cas {
                        result = Err(CacheError::KeyExists);
                        Op::Nop
                    } else {
                        match self.incr_decr_common(&record, delta, increment) {
                            Ok(new_value) => {
                                if cas != 0 && record.header.cas != cas {
                                    result = Err(CacheError::KeyExists);
                                    return Op::Nop;
                                }
                                let new_cas = self.get_cas_id();
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
                    if header.get_expiration() != 0xffffffff {
                        let cas = self.get_cas_id();
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
