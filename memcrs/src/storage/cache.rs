use super::error::{StorageError, StorageResult};
use bytes::Bytes;

/// Cache key type
pub type KeyType = Bytes;

/// Cache value associated with a key
pub type ValueType = Bytes;

/// Meta data stored with cache value
#[derive(Clone, Debug)]
pub struct CacheMetaData {
    pub(crate) timestamp: u64,
    pub(crate) cas: u64,
    pub(crate) flags: u32,
    pub(crate) time_to_live: u32,
}

impl CacheMetaData {
    pub fn new(cas: u64, flags: u32, time_to_live: u32) -> CacheMetaData {
        CacheMetaData {
            timestamp: 0,
            cas,
            flags,
            time_to_live,
        }
    }

    pub fn get_expiration(&self) -> u32 {
        self.time_to_live
    }

    pub const fn len(&self) -> usize {
        std::mem::size_of::<CacheMetaData>()
    }

    pub const fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

/// Value and meta data stored in cache
#[derive(Clone, Debug)]
pub struct Record {
    pub(crate) header: CacheMetaData,
    pub(crate) value: ValueType,
}

impl Record {
    pub fn new(value: ValueType, cas: u64, flags: u32, expiration: u32) -> Record {
        let header = CacheMetaData::new(cas, flags, expiration);
        Record { header, value }
    }

    pub fn len(&self) -> usize {
        self.header.len() + self.value.len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

impl PartialEq for Record {
    fn eq(&self, other: &Self) -> bool {
        self.value == other.value
    }
}

/// Result of set operation on cache
/// cas indicates version stored in cache
#[derive(Debug)]
pub struct SetStatus {
    pub cas: u64,
}

/// Read only view over a store
pub trait KVStoreReadOnlyView<'a> {
  fn len(&self) -> usize;
  fn is_empty(&self) -> bool;
  fn keys(&'a self) -> Box<dyn Iterator<Item = &'a KeyType> + 'a>;
}

// Not a part of Store public API
pub mod impl_details {
  use super::*;
  pub trait StoreImplDetails {
      //
      fn get_by_key(&self, key: &KeyType) -> StorageResult<Record>;

      //
      fn check_if_expired(&self, key: &KeyType, record: &Record) -> bool;
  }
}

pub type RemoveIfResult = Vec<Option<(KeyType, Record)>>;
pub type Predicate = dyn FnMut(&KeyType, &Record) -> bool;


// An abstraction over a generic store key <=> value store
pub trait KVStore: impl_details::StoreImplDetails {
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
  fn delete(&self, key: KeyType, header: CacheMetaData) -> StorageResult<Record>;

  // Removes all values from a store
  //
  // - if header.ttl is set to 0 values are removed immediately,
  // - if header.ttl>0 values are removed from a store after
  //   ttl expiration
  fn flush(&self, header: CacheMetaData);

  // Number of key value pairs stored in store
  fn len(&self) -> usize;

  fn is_empty(&self) -> bool;

  // Returns a read-only view over a stroe
  fn as_read_only(&self) -> Box<dyn KVStoreReadOnlyView>;

  // Removes key-value pairs from a store for which
  // f predicate returns true
  fn remove_if(&self, f: &mut Predicate) -> RemoveIfResult;

  // Removes key value and returns as an option
  fn remove(&self, key: &KeyType) -> Option<(KeyType, Record)>;
}
