use std::sync::Arc;
use std::sync::atomic;
use crate::storage::store::{KVStore, KeyType, Record, Meta, SetStatus, KVStoreReadOnlyView};
use crate::storage::error::StorageResult;


pub struct RandomPolicy {
    store:  Arc<dyn KVStore+ Send + Sync>,
    memory_limit: u64,
    memory_usage: atomic::AtomicU64
}

impl RandomPolicy {
    pub fn new(store: Arc<dyn KVStore+ Send + Sync>, memory_limit: u64) -> RandomPolicy {
        RandomPolicy {
            store: store,
            memory_limit: memory_limit,
            memory_usage: atomic::AtomicU64::new(0)
        }
    }
}

impl KVStore for RandomPolicy {
    fn get(&self, key: &KeyType) -> StorageResult<Record> {
        self.store.get(key)
    }

    fn set(&self, key: KeyType, record: Record) -> StorageResult<SetStatus> {
        let len = record.len();
        let result = self.store.set(key, record);
        if let Ok(_status) = &result  {
            self.memory_usage.fetch_add(len as u64, atomic::Ordering::SeqCst);    
        }
        
        result
    }

    fn delete(&self, key: KeyType, header: Meta) -> StorageResult<Record> {
        let result = self.store.delete(key, header);
        if let Ok(record) =  &result {
            self.memory_usage.fetch_sub(record.len() as u64, atomic::Ordering::SeqCst);          
        }
        result
    }

    fn flush(&self, header: Meta) {
        self.store.flush(header)
    }

    fn into_read_only(&self) -> Box<dyn KVStoreReadOnlyView> {
        self.store.into_read_only()
    }
}
