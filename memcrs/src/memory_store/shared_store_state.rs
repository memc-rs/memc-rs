use crate::cache::cache::{DeltaParam, KeyType, Record};
use crate::cache::error::{CacheError, Result};
use crate::server::timer::Timer;
use std::sync::atomic::Ordering;
use std::sync::{atomic::AtomicU64, Arc};

pub struct SharedStoreState {
    timer: Arc<dyn Timer + Send + Sync>,
    cas_id: AtomicU64,
}

impl SharedStoreState {
    pub fn new(timer: Arc<dyn Timer + Send + Sync>) -> SharedStoreState {
        SharedStoreState {
            timer,
            cas_id: AtomicU64::new(1),
        }
    }

    #[inline]
    pub fn cas_mismatch(record: &Record, cas: u64) -> bool {
        record.header.cas != 0 && cas != record.header.cas
    }

    pub fn set_cas_ttl(&self, record: &mut Record) -> u64 {
        record.header.cas = match record.header.cas {
            0 => self.get_cas_id(),
            _ => record.header.cas.wrapping_add(1),
        };
        let timestamp = self.timestamp();
        if record.header.time_to_live > 0 {
            record.header.time_to_live += timestamp;
        }
        record.header.cas
    }

    pub fn timestamp(&self) -> u32 {
        self.timer.timestamp()
    }

    pub fn get_cas_id(&self) -> u64 {
        self.cas_id.fetch_add(1, Ordering::Release)
    }

    /// Default implementation for performing arithmetic operations on a numeric value.
    /// Parses the record's value as a u64, adds or subtracts the delta based on `increment`,
    /// and returns the new value as Bytes. Fails if the value is not a valid u64.
    pub fn incr_decr_common(
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

    pub fn check_if_expired(&self, _key: &KeyType, record: &Record) -> bool {
        let current_time = self.timer.timestamp();

        if record.header.time_to_live == 0 {
            return false;
        }

        if record.header.time_to_live > current_time {
            return false;
        }
        true
    }
}
