use super::error::{CacheError, Result};
use bytes::Bytes;

/// Cache key type
pub type KeyType = Bytes;

/// Cache value associated with a key
pub type ValueType = Bytes;

#[derive(Clone)]
pub struct DeltaParam {
    pub(crate) delta: u64,
    pub(crate) value: u64,
}

pub type IncrementParam = DeltaParam;
pub type DecrementParam = IncrementParam;

pub type DeltaResultValueType = u64;
#[derive(Debug)]
pub struct DeltaResult {
    pub cas: u64,
    pub value: DeltaResultValueType,
}

/// Meta data stored with cache value
#[derive(Clone, Debug)]
pub struct CacheMetaData {
    pub(crate) cas: u64,
    pub(crate) flags: u32,
    pub(crate) time_to_live: u32,
}

impl CacheMetaData {
    pub fn new(cas: u64, flags: u32, time_to_live: u32) -> CacheMetaData {
        CacheMetaData {
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

// Not a part of Store public API
pub mod impl_details {

    use super::*;
    pub trait CacheImplDetails {
        /// Default implementation for performing arithmetic operations on a numeric value.
        /// Parses the record's value as a u64, adds or subtracts the delta based on `increment`,
        /// and returns the new value as Bytes. Fails if the value is not a valid u64.
        fn incr_decr_common(
            &self,
            record: &Record,
            delta: DeltaParam,
            increment: bool,
        ) -> Result<u64> {
            str::from_utf8(&record.value)
                .map(|value: &str| {
                    value
                        .parse::<u64>()
                        .map_err(|_err| CacheError::ArithOnNonNumeric)
                })
                .map_err(|_err| CacheError::ArithOnNonNumeric)
                .and_then(|value: std::result::Result<u64, CacheError>| {
                    //flatten result
                    value
                })
                .map(|mut value: u64| {
                    if increment {
                        value += delta.delta;
                    } else if delta.delta > value {
                        value = 0;
                    } else {
                        value -= delta.delta;
                    }
                    value
                })
        }
    }
}

// An abstraction over a generic store key <=> value store
pub trait Cache: impl_details::CacheImplDetails {
    /// Returns a value associated with a key
    fn get(&self, key: &KeyType) -> Result<Record>;
    // let result = self.get_by_key(key);
    // match result {
    //     Ok(record) => {
    //         if self.check_if_expired(key, &record) {
    //             return Err(CacheError::NotFound);
    //         }
    //         Ok(record)
    //     }
    //     Err(err) => Err(err),
    // }

    /// Sets value that will be associated with a store.
    /// If value already exists in a store CAS field is compared
    /// and depending on CAS value comparison value is set or rejected.
    ///
    /// - if CAS is equal to 0 value is always set
    /// - if CAS is not equal value is not set and there is an error
    ///   returned with status KeyExists
    fn set(&self, key: KeyType, record: Record) -> Result<SetStatus>;

    /// Removes a value associated with a key a returns it to a caller if CAS
    /// value comparison is successful or header.CAS is equal to 0:
    ///
    /// - if header.CAS != to stored record CAS KeyExists is returned
    /// - if key is not found NotFound is returned
    fn delete(&self, key: KeyType, header: CacheMetaData) -> Result<Record>;

    /// Removes all values from a store
    ///
    /// - if header.ttl is set to 0 values are removed immediately,
    /// - if header.ttl>0 values are removed from a store after
    ///   ttl expiration
    fn flush(&self, header: CacheMetaData);

    /// runs pending tasks (if any)
    /// will be scheudled periodicall
    fn run_pending_tasks(&self);

    /// Adds a new key-value pair to the cache, but only if the key does not already exist.
    /// If the key exists, the operation fails with KeyExists error.
    fn add(&self, key: KeyType, record: Record) -> Result<SetStatus>;

    /// Replaces the value of an existing key in the cache, but only if the key already exists.
    /// If the key does not exist, the operation fails with NotFound error.
    fn replace(&self, key: KeyType, record: Record) -> Result<SetStatus>;

    /// Appends the new value to the existing value for the given key.
    /// The key must already exist in the cache, otherwise the operation fails with NotFound error.
    fn append(&self, key: KeyType, new_record: Record) -> Result<SetStatus>;

    /// Prepends the new value to the existing value for the given key.
    /// The key must already exist in the cache, otherwise the operation fails with NotFound error.
    fn prepend(&self, key: KeyType, new_record: Record) -> Result<SetStatus>;

    /// Performs an arithmetic operation (increment or decrement) on a numeric value stored in the cache.
    /// If `increment` is true, adds `delta` to the value; otherwise, subtracts `delta`.
    /// The value must be a valid unsigned 64-bit integer.
    /// Returns the new value after the operation.
    fn incr_decr(
        &self,
        header: CacheMetaData,
        key: KeyType,
        delta: DeltaParam,
        increment: bool,
    ) -> Result<DeltaResult>;
}

#[cfg(test)]
mod tests {

    use super::*;
    use bytes::Bytes;

    #[test]
    fn test_cache_metadata_new() {
        let meta = CacheMetaData::new(42, 1, 3600);
        assert_eq!(meta.cas, 42);
        assert_eq!(meta.flags, 1);
        assert_eq!(meta.time_to_live, 3600);
    }

    #[test]
    fn test_cache_metadata_get_expiration() {
        let meta = CacheMetaData::new(100, 2, 7200);
        assert_eq!(meta.get_expiration(), 7200);
    }

    #[test]
    fn test_cache_metadata_len() {
        let meta = CacheMetaData::new(0, 0, 0);
        assert_eq!(meta.len(), std::mem::size_of::<CacheMetaData>());
    }

    #[test]
    fn test_cache_metadata_is_empty() {
        let meta = CacheMetaData::new(0, 0, 0);
        assert!(!meta.is_empty());
    }

    #[test]
    fn test_record_new() {
        let value = Bytes::from("test_value");
        let record = Record::new(value.clone(), 10, 3, 600);
        assert_eq!(record.value, value);
        assert_eq!(record.header.cas, 10);
        assert_eq!(record.header.flags, 3);
        assert_eq!(record.header.time_to_live, 600);
    }

    #[test]
    fn test_record_len() {
        let value = Bytes::from("1234");
        let record = Record::new(value.clone(), 1, 0, 300);
        assert_eq!(
            record.len(),
            std::mem::size_of::<CacheMetaData>() + value.len()
        );
    }

    #[test]
    fn test_record_is_empty() {
        let value = Bytes::from("test");
        let record = Record::new(value, 1, 0, 300);
        assert!(!record.is_empty());
    }

    #[test]
    fn test_record_equality() {
        let value1 = Bytes::from("value");
        let value2 = Bytes::from("value");
        let record1 = Record::new(value1.clone(), 1, 0, 300);
        let record2 = Record::new(value2.clone(), 2, 1, 600);
        assert_eq!(record1, record2);
    }
}
