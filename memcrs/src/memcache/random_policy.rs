use crate::storage::error::StorageResult;
use crate::storage::store::{KVStore, KVStoreReadOnlyView, KeyType, Meta, Record, SetStatus, Predicate, RemoveIfResult};
use rand::{Rng, SeedableRng};
use rand::rngs::SmallRng;
use std::sync::atomic;
use std::sync::Arc;

pub struct RandomPolicy {
    store: Arc<dyn KVStore + Send + Sync>,
    memory_limit: u64,
    memory_usage: atomic::AtomicU64,
}

impl RandomPolicy {
    pub fn new(store: Arc<dyn KVStore + Send + Sync>, memory_limit: u64) -> RandomPolicy {
        RandomPolicy {
            store: store,
            memory_limit: memory_limit,
            memory_usage: atomic::AtomicU64::new(0),            
        }
    }

    fn incr_mem_usage(&self, value: u64) -> u64 {
        let mut usage = self.memory_usage.fetch_add(value, atomic::Ordering::SeqCst);
        while usage > self.memory_limit {
            let max = self.store.len();
            let mut small_rng = SmallRng::from_entropy();
            let item = small_rng.gen_range(0..max);
            let mut number_of_calls: usize = 0;
            let res = self.store.remove_if(&move |key: &KeyType, value: &Record| -> bool {                
                if number_of_calls < item || number_of_calls > item{                    
                    number_of_calls += 1;
                    return false;
                }
                number_of_calls += 1;
                true
            });

            match res {
                Some(val) => {
                    let len = val.1.len();
                    usage = self.decr_mem_usage(len as u64);
                },
                None => {
                    break;
                }
            }
        }
        usage
    }

    fn decr_mem_usage(&self, value: u64) -> u64 {
        self.memory_usage.fetch_sub(value, atomic::Ordering::SeqCst)
    }
}

impl KVStore for RandomPolicy {
    fn get(&self, key: &KeyType) -> StorageResult<Record> {
        self.store.get(key)
    }

    fn set(&self, key: KeyType, record: Record) -> StorageResult<SetStatus> {
        let len = record.len() as u64;
        self.incr_mem_usage(len);
        let result = self.store.set(key, record);        
        result
    }

    fn delete(&self, key: KeyType, header: Meta) -> StorageResult<Record> {
        let result = self.store.delete(key, header);
        if let Ok(record) = &result {
            self.decr_mem_usage(record.len() as u64);
        }
        result
    }

    fn flush(&self, header: Meta) {
        self.store.flush(header)
    }

    fn into_read_only(&self) -> Box<dyn KVStoreReadOnlyView> {
        self.store.into_read_only()
    }

    fn remove_if(
        &self,    
        f: &Predicate        
    ) -> RemoveIfResult {
        self.store.remove_if(f)
    }

    fn len(&self) -> usize {
        self.store.len()
    }
}
