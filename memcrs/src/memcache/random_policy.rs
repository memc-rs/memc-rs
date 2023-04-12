use crate::cache::error::Result;
use crate::cache::cache::{
    impl_details::CacheImplDetails, Cache, CacheReadOnlyView, KeyType, CacheMetaData, CachePredicate, Record,
    RemoveIfResult, SetStatus
};
use rand::rngs::SmallRng;
use rand::{Rng, SeedableRng};
use std::sync::atomic;
use std::sync::Arc;

pub struct RandomPolicy {
    store: Arc<dyn Cache + Send + Sync>,
    memory_limit: u64,
    memory_usage: atomic::AtomicU64,
}

impl RandomPolicy {
    pub fn new(store: Arc<dyn Cache + Send + Sync>, memory_limit: u64) -> RandomPolicy {
        RandomPolicy {
            store,
            memory_limit,
            memory_usage: atomic::AtomicU64::new(0),
        }
    }

    fn incr_mem_usage(&self, value: u64) -> u64 {
        let mut usage = self.memory_usage.fetch_add(value, atomic::Ordering::SeqCst);

        let mut small_rng = SmallRng::from_entropy();
        while usage > self.memory_limit {
            debug!("Current memory usage: {}", usage);
            debug!("Memory limit: {}", self.memory_limit);

            let max = self.store.len();
            if max == 0 {
                self.decr_mem_usage(usage);
                break;
            }
            let item = small_rng.gen_range(0..max);
            let mut number_of_calls: usize = 0;
            let res = self
                .store
                .remove_if(&mut move |_key: &KeyType, _value: &Record| -> bool {
                    if number_of_calls != item {
                        number_of_calls += 1;
                        return false;
                    }
                    number_of_calls += 1;
                    true
                });

            res.iter().for_each(|record| match record {
                Some(val) => {
                    let len = val.1.len();
                    debug!("Evicted: {} bytes from storage", len);
                    usage = self.decr_mem_usage(len as u64);
                }
                None => {}
            });
        }
        usage
    }

    fn decr_mem_usage(&self, value: u64) -> u64 {
        self.memory_usage.fetch_sub(value, atomic::Ordering::SeqCst)
    }
}

impl CacheImplDetails for RandomPolicy {
    //
    fn get_by_key(&self, key: &KeyType) -> Result<Record> {
        self.store.get_by_key(key)
    }

    //
    fn check_if_expired(&self, key: &KeyType, record: &Record) -> bool {
        self.store.check_if_expired(key, record)
    }
}

impl Cache for RandomPolicy {
    fn get(&self, key: &KeyType) -> Result<Record> {
        self.store.get(key)
    }

    fn set(&self, key: KeyType, record: Record) -> Result<SetStatus> {
        let len = record.len() as u64;
        self.incr_mem_usage(len);
        self.store.set(key, record)
    }

    fn delete(&self, key: KeyType, header: CacheMetaData) -> Result<Record> {
        let result = self.store.delete(key, header);
        if let Ok(record) = &result {
            self.decr_mem_usage(record.len() as u64);
        }
        result
    }

    // Removes key value and returns as an option
    fn remove(&self, key: &KeyType) -> Option<(KeyType, Record)> {
        let result = self.store.remove(key);
        if let Some(key_value) = &result {
            self.decr_mem_usage(key_value.1.len() as u64);
        }
        result
    }

    fn flush(&self, header: CacheMetaData) {
        self.store.flush(header)
    }

    fn as_read_only(&self) -> Box<dyn CacheReadOnlyView> {
        self.store.as_read_only()
    }

    fn remove_if(&self, f: &mut CachePredicate) -> RemoveIfResult {
        self.store.remove_if(f)
    }

    fn len(&self) -> usize {
        self.store.len()
    }

    fn is_empty(&self) -> bool {
        self.store.is_empty()
    }
}

mod tests {}
