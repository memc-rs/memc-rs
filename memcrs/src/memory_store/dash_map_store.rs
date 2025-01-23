use crate::cache::cache::{
    impl_details, Cache, CacheMetaData,  CacheReadOnlyView, KeyType, Record,
   SetStatus,
};
use crate::cache::error::{CacheError, Result};
use crate::server::timer;

use dashmap::{DashMap, ReadOnlyView};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

type Storage = DashMap<KeyType, Record>;
pub struct DashMapMemoryStore {
    memory: Storage,
    timer: Arc<dyn timer::Timer + Send + Sync>,
    cas_id: AtomicU64,
}

type StorageReadOnlyView = ReadOnlyView<KeyType, Record>;

impl<'a> CacheReadOnlyView<'a> for StorageReadOnlyView {
    fn len(&self) -> usize {
        StorageReadOnlyView::len(self)
    }

    fn is_empty(&self) -> bool {
        StorageReadOnlyView::is_empty(self)
    }

    fn keys(&'a self) -> Box<dyn Iterator<Item = &'a KeyType> + 'a> {
        let keys = self.keys();
        Box::new(keys)
    }
}

impl DashMapMemoryStore {
    pub fn new(timer: Arc<dyn timer::Timer + Send + Sync>) -> DashMapMemoryStore {
        DashMapMemoryStore {
            memory: DashMap::new(),
            timer,
            cas_id: AtomicU64::new(1),
        }
    }

    fn get_cas_id(&self) -> u64 {
        self.cas_id.fetch_add(1, Ordering::Release)
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

        if record.header.timestamp + (record.header.time_to_live as u64) > current_time {
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
                        record.header.cas += 1;
                        record.header.timestamp = self.timer.timestamp();
                        let cas = record.header.cas;
                        *key_value = record;
                        Ok(SetStatus { cas })
                    }
                }
                None => {
                    record.header.cas += 1;
                    record.header.timestamp = self.timer.timestamp();
                    let cas = record.header.cas;
                    self.memory.insert(key, record);
                    Ok(SetStatus { cas })
                }
            }
        } else {
            let cas = self.get_cas_id();
            record.header.cas = cas;
            record.header.timestamp = self.timer.timestamp();
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

    fn as_read_only(&self) -> Box<dyn CacheReadOnlyView> {
        let storage_clone = self.memory.clone();
        Box::new(storage_clone.into_read_only())
    }

    fn len(&self) -> usize {
        self.memory.len()
    }

    fn is_empty(&self) -> bool {
        self.memory.is_empty()
    }

    fn run_pending_tasks(&self) {}
}
