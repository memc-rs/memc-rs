use crate::cache::cache::{
    Cache, CacheMetaData as CacheMeta, DecrementParam, DeltaParam, DeltaResult, IncrementParam,
    KeyType as CacheKeyType, Record as CacheRecord, SetStatus as CacheSetStatus,
};
use crate::cache::error::Result;
use std::sync::Arc;

pub type Record = CacheRecord;
pub type Meta = CacheMeta;
pub type SetStatus = CacheSetStatus;
pub type KeyType = CacheKeyType;

/**
 * Implements Memcache commands based
 * on Key Value Store
 */
pub struct MemcStore {
    store: Arc<dyn Cache + Send + Sync>,
}

impl MemcStore {
    pub fn new(store: Arc<dyn Cache + Send + Sync>) -> MemcStore {
        MemcStore { store }
    }

    pub fn set(&self, key: KeyType, record: Record) -> Result<SetStatus> {
        self.store.set(key, record)
    }

    pub fn get(&self, key: &KeyType) -> Result<Record> {
        self.store.get(key)
    }

    // fn touch_record(&self, _record: &mut Record) {
    //     let _timer = self.timer.secs();
    // }

    pub fn add(&self, key: KeyType, record: Record) -> Result<SetStatus> {
        self.store.add(key, record)
    }

    pub fn replace(&self, key: KeyType, record: Record) -> Result<SetStatus> {
        self.store.replace(key, record)
    }

    pub fn append(&self, key: KeyType, new_record: Record) -> Result<SetStatus> {
        self.store.append(key, new_record)
    }

    pub fn prepend(&self, key: KeyType, new_record: Record) -> Result<SetStatus> {
        self.store.prepend(key, new_record)
    }

    pub fn increment(
        &self,
        header: Meta,
        key: KeyType,
        increment: IncrementParam,
    ) -> Result<DeltaResult> {
        self.add_delta(header, key, increment, true)
    }

    pub fn decrement(
        &self,
        header: Meta,
        key: KeyType,
        decrement: DecrementParam,
    ) -> Result<DeltaResult> {
        self.add_delta(header, key, decrement, false)
    }

    fn add_delta(
        &self,
        header: Meta,
        key: KeyType,
        delta: DeltaParam,
        increment: bool,
    ) -> Result<DeltaResult> {
        self.store.incr_decr(header, key, delta, increment)
    }

    pub fn delete(&self, key: KeyType, header: Meta) -> Result<Record> {
        self.store.delete(key, header)
    }

    pub fn flush(&self, header: Meta) {
        self.store.flush(header)
    }
}

#[cfg(test)]
mod add_tests;
#[cfg(test)]
mod append_prepend_tests;
#[cfg(test)]
mod delete_tests;
#[cfg(test)]
mod flush_tests;
#[cfg(test)]
mod increment_decrement_tests;
#[cfg(test)]
mod replace_tests;
#[cfg(test)]
mod set_tests;

#[cfg(test)]
mod test_utils {
    pub use super::*;
    pub use crate::cache::error::CacheError;
    pub use crate::mock::mock_server::{
        create_dash_map_server, create_moka_server, MockServer, SetableTimer,
    };
    pub use crate::mock::value::{from_slice, from_string};
    pub use bytes::{BufMut, Bytes, BytesMut};
}
