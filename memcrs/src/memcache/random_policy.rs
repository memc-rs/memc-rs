use std::sync::Arc;
use std::sync::atomic;
use crate::storage::store::{KVStore, KeyType, Record, Meta, SetStatus};
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
        return self.store.get(key);
    }

    fn set(&self, key: KeyType, record: Record) -> StorageResult<SetStatus> {
        return self.store.set(key, record);
    }

    fn delete(&self, key: KeyType, header: Meta) -> StorageResult<()> {
        return self.store.delete(key, header);
    }

    fn flush(&self, header: Meta) {
        return self.store.flush(header);
    }
}
