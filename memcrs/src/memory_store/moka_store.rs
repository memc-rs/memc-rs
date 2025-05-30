use crate::cache::cache::{
    impl_details, Cache, CacheMetaData, KeyType, Record, SetStatus,
};
use crate::cache::error::{CacheError, Result};
use crate::server::timer;
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
            if record.header.cas == 0 {
                let cas = self.get_cas_id();
                record.header.cas = cas;
            } else {
                record.header.cas += 1;
            }
            let timestamp = self.timer.timestamp();
            if record.header.time_to_live > 0 {
                record.header.time_to_live += timestamp;
            }
            let cas = record.header.cas;
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
}
