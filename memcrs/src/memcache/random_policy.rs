use crate::storage::error::StorageResult;
use crate::storage::store::{KVStore, KVStoreReadOnlyView, KeyType, Meta, Record, SetStatus};
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
        self.memory_usage.fetch_add(value, atomic::Ordering::SeqCst)
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
        let result = self.store.set(key, record);
        if let Ok(_status) = &result {
            self.incr_mem_usage(len);
        }
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
}
