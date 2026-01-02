use crate::cache::cache::{impl_details, Cache, CacheMetaData, KeyType, Record, SetStatus};
use crate::cache::error::{CacheError, Result};
use crate::server::timer;

use bytes::BytesMut;
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
        let parallelism = std::thread::available_parallelism().map_or(1, usize::from);
        let shards = Self::get_number_of_shards(parallelism);
        info!("Number of shards: {}", shards);
        DashMapMemoryStore {
            memory: DashMap::with_shard_amount(shards),
            timer,
            cas_id: AtomicU64::new(1),
        }
    }

    // This function is used to get the number of shards based on the available parallelism.
    // It calculates the optimal number of shards based on the square of the parallelism divided by 4.
    // It then finds the closest power of 2 to that number and returns it.
    fn get_number_of_shards(parallelism: usize) -> usize {
        let parallelism = parallelism.max(2);
        let parallelism = parallelism.min(192);

        let optimal_number_shards = parallelism.pow(2) / 4;
        if optimal_number_shards < 2 {
            return 2;
        }

        let closest_power_of_2 = optimal_number_shards.ilog2();
        let shards_power_of_2 = 2usize.pow(closest_power_of_2);
        info!("Avialable parallelism: {}", parallelism);
        info!("Optimal number of shards: {}", optimal_number_shards);
        info!("Closest power of 2: {}", closest_power_of_2);

        if shards_power_of_2 > 1 {
            shards_power_of_2
        } else {
            2
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

    fn add(&self, key: KeyType, mut record: Record) -> Result<SetStatus> {
        use dashmap::mapref::entry::Entry;
        match self.memory.entry(key) {
            Entry::Occupied(_) => Err(CacheError::KeyExists),
            Entry::Vacant(entry) => {
                let cas = self.get_cas_id();
                record = self.set_cas_ttl(record, cas);
                entry.insert(record);
                Ok(SetStatus { cas })
            }
        }
    }

    fn replace(&self, key: KeyType, mut record: Record) -> Result<SetStatus> {
        use dashmap::mapref::entry::Entry;
        match self.memory.entry(key) {
            Entry::Occupied(mut entry) => {
                let cas = self.get_cas_id();
                record = self.set_cas_ttl(record, cas);
                entry.insert(record);
                Ok(SetStatus { cas })
            }
            Entry::Vacant(_) => Err(CacheError::NotFound),
        }
    }

    fn append(&self, key: KeyType, new_record: Record) -> Result<SetStatus> {
        use dashmap::mapref::entry::Entry;
        match self.memory.entry(key) {
            Entry::Occupied(mut entry) => {
                let mut record = entry.get().clone();
                let mut value = BytesMut::with_capacity(record.value.len() + new_record.value.len());
                value.extend_from_slice(&record.value);
                value.extend_from_slice(&new_record.value);
                record.value = value.freeze();
                
                let cas = self.get_cas_id();
                record = self.set_cas_ttl(record, cas);
                entry.insert(record);
                Ok(SetStatus { cas })
            }
            Entry::Vacant(_) => Err(CacheError::NotFound),
        }
    }

    fn prepend(&self, key: KeyType, new_record: Record) -> Result<SetStatus> {
        use dashmap::mapref::entry::Entry;
        match self.memory.entry(key) {
            Entry::Occupied(mut entry) => {
                let mut record = entry.get().clone();
                let mut value = BytesMut::with_capacity(record.value.len() + new_record.value.len());
                value.extend_from_slice(&new_record.value);
                value.extend_from_slice(&record.value);
                record.value = value.freeze();
                
                let cas = self.get_cas_id();
                record = self.set_cas_ttl(record, cas);
                entry.insert(record);
                Ok(SetStatus { cas })
            }
            Entry::Vacant(_) => Err(CacheError::NotFound),
        }
    }

    fn run_pending_tasks(&self) {}
}

#[cfg(test)]
mod tests {
    use super::DashMapMemoryStore;

    fn is_power_of_two(x: usize) -> bool {
        x != 0 && (x & (x - 1)) == 0
    }

    #[test]
    fn test_get_parallelism_returns_power_of_two() {
        for parallelism in vec![
            3,
            7,
            11,
            15,
            21,
            31,
            63,
            127,
            4096,
            8192,
            9_223_372_036_854_775_783,
            usize::MAX / 2,
            usize::MAX,
        ] {
            let shards = DashMapMemoryStore::get_number_of_shards(parallelism);
            assert!(
                is_power_of_two(shards),
                "Returned value {} is not a power of 2 for parallelism {}",
                shards,
                parallelism
            );
        }
    }

    #[test]
    fn test_get_parallelism_minimum_value() {
        // Should never return less than 2
        assert_eq!(DashMapMemoryStore::get_number_of_shards(0), 2);
        assert_eq!(DashMapMemoryStore::get_number_of_shards(1), 2);
        assert_eq!(DashMapMemoryStore::get_number_of_shards(2), 2);
    }
}
