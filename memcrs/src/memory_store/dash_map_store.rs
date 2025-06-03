use crate::cache::cache::{impl_details, Cache, CacheMetaData, KeyType, Record, SetStatus};
use crate::cache::error::{CacheError, Result};
use crate::server::timer;

use dashmap::DashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

type Storage = DashMap<KeyType, Record>;
pub struct DashMapMemoryStore {
    memory: Storage,
    timer: Arc<dyn timer::Timer + Send + Sync>,
    cas_id: AtomicU64,
}

impl DashMapMemoryStore {
    pub fn new(timer: Arc<dyn timer::Timer + Send + Sync>) -> DashMapMemoryStore {
        // cores²/2
        let parallelism = std::thread::available_parallelism().map_or(1, usize::from);
        let shards = parallelism.pow(2) / 4;

        info!("Avialable parallelism: {}", parallelism);
        info!("Number of shards: {}", shards);

        DashMapMemoryStore {
            memory: DashMap::with_shard_amount(shards),
            timer,
            cas_id: AtomicU64::new(1),
        }
    }

    fn get_cas_id(&self) -> u64 {
        self.cas_id.fetch_add(1, Ordering::Release)
    }

    fn set_cas_ttl(&self, mut record: Record, cas: u64) -> Record {
        record.header.cas = cas;
        let timestamp = self.timer.timestamp();
        if record.header.time_to_live != 0 {
            record.header.time_to_live += timestamp;
        }
        record
    }
}

impl impl_details::CacheImplDetails for DashMapMemoryStore {
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

impl Cache for DashMapMemoryStore {
    // Removes key value and returns as an option
    fn remove(&self, key: &KeyType) -> Option<(KeyType, Record)> {
        self.memory.remove(key)
    }

    fn set(&self, key: KeyType, mut record: Record) -> Result<SetStatus> {
        //trace!("Set: {:?}", &record.header);
        if record.header.cas > 0 {
            match self.memory.get_mut(&key) {
                Some(mut key_value) => {
                    if key_value.header.cas != record.header.cas {
                        Err(CacheError::KeyExists)
                    } else {
                        let cas = record.header.cas + 1;
                        record = self.set_cas_ttl(record, cas);
                        *key_value = record;
                        Ok(SetStatus { cas })
                    }
                }
                None => {
                    let cas = record.header.cas + 1;
                    record = self.set_cas_ttl(record, cas);
                    self.memory.insert(key, record);
                    Ok(SetStatus { cas })
                }
            }
        } else {
            let cas = self.get_cas_id();
            record = self.set_cas_ttl(record, cas);
            self.memory.insert(key, record);
            Ok(SetStatus { cas })
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

    fn len(&self) -> usize {
        self.memory.len()
    }

    fn run_pending_tasks(&self) {}
}
