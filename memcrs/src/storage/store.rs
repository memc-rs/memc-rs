use super::error::{StorageError, StorageResult};
use super::timer;
use bytes::Bytes;
use dashmap::mapref::multiple::RefMulti;
use dashmap::{DashMap, ReadOnlyView};
use core::hash::{Hash};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;


#[derive(Debug)]
pub struct SetStatus {
    pub cas: u64,
}

// Read only view over a store
// pub trait KVStoreReadOnlyView<'a> {
//     fn len(&self) -> usize;
//     fn is_empty(&self) -> bool;
//     fn keys(&'a self) -> Box<dyn Iterator<Item = &'a KeyType> + 'a>;
// }

// Not a part of Store public API
pub mod impl_details {
    use super::*;
    
    pub trait StoreImplDetails<Key, Value> {
        //
        fn get_by_key(&self, key: &Key) -> StorageResult<Value>;

        //
        fn check_if_expired(&self, key: &Key, record: &Value) -> bool;
    
    }
}

//pub type RemoveIfResult = Vec<Option<(KeyType, Record)>>;
//pub type Predicate = dyn FnMut(&KeyType, &Record) -> bool;

// An abstraction over a generic store key <=> value store
pub trait KVStore<KeyType, Record>: impl_details::StoreImplDetails<KeyType, Record> {

    // Returns a value associated with a key
    fn get(&self, key: &KeyType) -> StorageResult<Record> {
        let result = self.get_by_key(key);
        match result {
            Ok(record) => {
                if self.check_if_expired(key, &record) {
                    return Err(StorageError::NotFound);
                }
                Ok(record)
            }
            Err(err) => Err(err),
        }
    }

    // Sets value that will be associated with a store.
    // If value already exists in a store CAS field is compared
    // and depending on CAS value comparison value is set or rejected.
    //
    // - if CAS is equal to 0 value is always set
    // - if CAS is not equal value is not set and there is an error
    //   returned with status KeyExists
    fn set(&self, key: KeyType, record: Record) -> StorageResult<SetStatus>;

    // Removes a value associated with a key a returns it to a caller if CAS
    // value comparison is successful or header.CAS is equal to 0:
    //
    // - if header.CAS != to stored record CAS KeyExists is returned
    // - if key is not found NotFound is returned
    fn delete(&self, key: KeyType, cas: u64) -> StorageResult<Record>;

    // Removes all values from a store
    //
    // - if header.ttl is set to 0 values are removed immediately,
    // - if header.ttl>0 values are removed from a store after
    //   ttl expiration
    fn flush(&self, ttl: u32);

    // Number of key value pairs stored in store
    fn len(&self) -> usize;

    fn is_empty(&self) -> bool;

    // Returns a read-only view over a stroe
    //fn as_read_only(&self) -> Box<dyn KVStoreReadOnlyView>;

    // Removes key-value pairs from a store for which
    // f predicate returns true
    fn remove_if(&self, f: &mut dyn FnMut(&KeyType, &Record) -> bool) -> Vec<Option<(KeyType, Record)>>;

    // Removes key value and returns as an option
    fn remove(&self, key: &KeyType) -> Option<(KeyType, Record)>;
}


pub struct KeyValueStore<Key , Value> {
    memory: DashMap<Key, Value>,
    timer: Arc<dyn timer::Timer + Send + Sync>,
    cas_id: AtomicU64,
}

// type StorageReadOnlyView = ReadOnlyView<KeyType, Record>;

// impl<'a> KVStoreReadOnlyView<'a> for StorageReadOnlyView {
//     fn len(&self) -> usize {
//         StorageReadOnlyView::len(self)
//     }

//     fn is_empty(&self) -> bool {
//         StorageReadOnlyView::is_empty(self)
//     }

//     fn keys(&'a self) -> Box<dyn Iterator<Item = &'a KeyType> + 'a> {
//         let keys = self.keys();
//         Box::new(keys)
//     }
// }

impl<Key: Eq + Hash + Clone, Value> KeyValueStore<Key, Value> {
    pub fn new(timer: Arc<dyn timer::Timer + Send + Sync>) -> KeyValueStore<Key, Value> {
        KeyValueStore {
            memory: DashMap::new(),
            timer,
            cas_id: AtomicU64::new(1),
        }
    }

    fn get_cas_id(&self) -> u64 {
        self.cas_id.fetch_add(1, Ordering::Release)
    }
}

impl<KeyType, Record> impl_details::StoreImplDetails<KeyType, Record> for KeyValueStore<KeyType, Record> {
    fn get_by_key(&self, key: &KeyType) -> StorageResult<Record> {
        match self.memory.get(key) {
            Some(record) => Ok(record.clone()),
            None => Err(StorageError::NotFound),
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

impl<KeyType, Record> KVStore<KeyType, Record> for KeyValueStore<KeyType, Record> {
    // Removes key value and returns as an option
    fn remove(&self, key: &KeyType) -> Option<(KeyType, Record)> {
        self.memory.remove(key)
    }

    fn set(&self, key: KeyType, mut record: Record) -> StorageResult<SetStatus> {
        //trace!("Set: {:?}", &record.header);
        if record.header.cas > 0 {
            match self.memory.get_mut(&key) {
                Some(mut key_value) => {
                    if key_value.header.cas != record.header.cas {
                        Err(StorageError::KeyExists)
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

    fn delete(&self, key: KeyType, cas: u64) -> StorageResult<Record> {
        let mut cas_match: Option<bool> = None;
        match self.memory.remove_if(&key, |_key, record| -> bool {
            let result = cas == 0 || record.header.cas == cas;
            cas_match = Some(result);
            result
        }) {
            Some(key_value) => Ok(key_value.1),
            None => match cas_match {
                Some(_value) => Err(StorageError::KeyExists),
                None => Err(StorageError::NotFound),
            },
        }
    }

    fn flush(&self, ttl: u32) {
        if ttl > 0 {
            self.memory.alter_all(|_key, mut value| {
                value.header.time_to_live = ttl;
                value
            });
        } else {
            self.memory.clear();
        }
    }

    // fn as_read_only(&self) -> Box<dyn KVStoreReadOnlyView> {
    //     let storage_clone = self.memory.clone();
    //     Box::new(storage_clone.into_read_only())
    // }

    fn remove_if(&self, f: &mut dyn FnMut(&KeyType, &Record) -> bool) -> Vec<Option<(KeyType, Record)>> {
        let items: Vec<KeyType> = self
            .memory
            .iter()
            .filter(|record: &RefMulti<KeyType, Record>| f(record.key(), record.value()))
            .map(|record: RefMulti<KeyType, Record>| record.key().clone())
            .collect();

        let result: Vec<Option<(KeyType, Record)>> =
            items.iter().map(|key: &KeyType| self.remove(key)).collect();
        result
    }

    fn len(&self) -> usize {
        self.memory.len()
    }

    fn is_empty(&self) -> bool {
        self.memory.is_empty()
    }
}
