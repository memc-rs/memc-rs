use super::error::{StorageError, StorageResult};
use super::timer;
use dashmap::DashMap;
use std::str;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

#[derive(Clone, Debug)]
pub struct Meta {
    pub(self) timestamp: u64,
    pub(crate) cas: u64,
    pub(crate) flags: u32,
    expiration: u32,
}

impl Meta {
    pub fn new(cas: u64, flags: u32, expiration: u32) -> Meta {
        Meta {
            timestamp: 0,
            cas,
            flags,
            expiration,
        }
    }
}

#[derive(Clone, Debug)]
pub struct Record {
    pub(crate) header: Meta,
    pub(crate) value: Vec<u8>,
}

impl Record {
    pub fn new(value: Vec<u8>, cas: u64, flags: u32, expiration: u32) -> Record {
        let header = Meta::new(cas, flags, expiration);
        Record { header, value }
    }
}

impl PartialEq for Record {
    fn eq(&self, other: &Self) -> bool {
        self.value == other.value
    }
}

#[derive(Clone)]
pub struct DeltaParam {
    pub(crate) delta: u64,
    pub(crate) value: u64,
}
pub type IncrementParam = DeltaParam;
pub type DecrementParam = IncrementParam;

pub struct Storage {
    memory: DashMap<Vec<u8>, Record>,
    timer: Arc<dyn timer::Timer + Send + Sync>,
    cas_id: AtomicU64,
}
#[derive(Debug)]
pub struct DeltaResult {
    pub cas: u64,
    pub value: u64,
}

#[derive(Debug)]
pub struct SetStatus {
    pub cas: u64,
}

impl Storage {
    pub fn new(timer: Arc<dyn timer::Timer + Send + Sync>) -> Storage {
        Storage {
            memory: DashMap::new(),
            timer,
            cas_id: AtomicU64::new(1),
        }
    }

    pub fn get(&self, key: &[u8]) -> StorageResult<Record> {
        info!("Get: {:?}", str::from_utf8(key));
        self.get_by_key(key)
    }

    fn get_by_key(&self, key: &[u8]) -> StorageResult<Record> {
        let result = match self.memory.get(key) {
            Some(record) => Ok(record.clone()),
            None => Err(StorageError::NotFound),
        };

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

    fn check_if_expired(&self, key: &[u8], record: &Record) -> bool {
        let current_time = self.timer.secs();

        if record.header.expiration == 0 {
            return false;
        }

        if record.header.timestamp + (record.header.expiration as u64) > current_time {
            return false;
        }
        match self.memory.remove(key) {
            Some(_) => true,
            None => true,
        }
    }

    fn touch_record(&self, _record: &mut Record) {
        let _timer = self.timer.secs();
    }
    /**
     * FIXME: Make it atomic operation based on CAS, now there is a race between
     * check_cas and insert
     */
    pub fn set(&self, key: Vec<u8>, mut record: Record) -> StorageResult<SetStatus> {
        info!("Set: {:?}", &record.header);

        if record.header.cas > 0 {
            match self.memory.get_mut(&key) {
                Some(mut key_value) => {
                    if key_value.header.cas != record.header.cas {
                        Err(StorageError::KeyExists)
                    } else {
                        record.header.cas += 1;
                        let cas = record.header.cas;
                        *key_value = record;
                        Ok(SetStatus { cas })
                    }
                }
                None => {
                    record.header.cas += 1;
                    let cas = record.header.cas;
                    self.memory.insert(key, record);
                    Ok(SetStatus { cas })
                }
            }
        } else {
            let cas = self.get_cas_id();
            record.header.cas = cas;
            self.memory.insert(key, record);
            Ok(SetStatus { cas })
        }
    }

    fn get_cas_id(&self) -> u64 {
        self.cas_id.fetch_add(1, Ordering::SeqCst) as u64
    }

    pub fn add(&self, key: Vec<u8>, record: Record) -> StorageResult<SetStatus> {
        match self.get_by_key(&key) {
            Ok(_record) => Err(StorageError::KeyExists),
            Err(_err) => self.set(key, record),
        }
    }

    pub fn replace(&self, key: Vec<u8>, record: Record) -> StorageResult<SetStatus> {
        match self.get_by_key(&key) {
            Ok(_record) => self.set(key, record),
            Err(_err) => Err(StorageError::NotFound),
        }
    }

    pub fn append(&self, key: Vec<u8>, mut new_record: Record) -> StorageResult<SetStatus> {
        match self.get_by_key(&key) {
            Ok(mut record) => {
                record.header.cas = new_record.header.cas;
                record.value.reserve(new_record.value.len());
                record.value.append(&mut new_record.value);
                self.set(key, record)
            }
            Err(_err) => Err(StorageError::NotFound),
        }
    }

    pub fn prepend(&self, key: Vec<u8>, mut new_record: Record) -> StorageResult<SetStatus> {
        match self.get_by_key(&key) {
            Ok(mut record) => {
                let cas = new_record.header.cas;
                new_record.value.reserve(record.value.len());
                new_record.value.append(&mut record.value);
                new_record.header = record.header;
                new_record.header.cas = cas;
                self.set(key, new_record)
            }
            Err(_err) => Err(StorageError::NotFound),
        }
    }

    pub fn increment(
        &self,
        header: Meta,
        key: Vec<u8>,
        increment: IncrementParam,
    ) -> StorageResult<DeltaResult> {
        self.add_delta(header, key, increment, true)
    }

    pub fn decrement(
        &self,
        header: Meta,
        key: Vec<u8>,
        decrement: DecrementParam,
    ) -> StorageResult<DeltaResult> {
        self.add_delta(header, key, decrement, false)
    }

    fn add_delta(
        &self,
        header: Meta,
        key: Vec<u8>,
        delta: DeltaParam,
        increment: bool,
    ) -> StorageResult<DeltaResult> {
        match self.get_by_key(&key) {
            Ok(mut record) => {
                str::from_utf8(&record.value)
                    .map(|value: &str| {
                        value
                            .parse::<u64>()
                            .map_err(|_err| StorageError::ArithOnNonNumeric)
                    })
                    .map_err(|_err| StorageError::ArithOnNonNumeric)
                    .and_then(|value: std::result::Result<u64, StorageError>| {
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
                        record.value = value.to_string().as_bytes().to_vec();
                        record.header = header;
                        self.set(key, record).map(|result| DeltaResult {
                            cas: result.cas,
                            value,
                        })
                    })
                    .and_then(|result: std::result::Result<DeltaResult, StorageError>| {
                        //flatten result
                        result
                    })
            }
            Err(_err) => {
                if header.expiration != 0xffffffff {
                    let record = Record::new(
                        delta.value.to_string().as_bytes().to_vec(),
                        0,
                        0,
                        header.expiration,
                    );
                    return self.set(key, record).map(|result| DeltaResult {
                        cas: result.cas,
                        value: delta.value,
                    });
                }
                Err(StorageError::NotFound)
            }
        }
    }

    pub fn delete(&self, key: Vec<u8>, header: Meta) -> StorageResult<()> {
        let mut cas_match: Option<bool> = None;
        match self.memory.remove_if(&key, |_key, record| -> bool {
            let result = header.cas == 0 || record.header.cas == header.cas;
            cas_match = Some(result);
            result
        }) {
            Some(_key_value) => Ok(()),
            None => match cas_match {
                Some(_value) => Err(StorageError::KeyExists),
                None => Err(StorageError::NotFound),
            },
        }
    }

    pub fn flush(&self, header: Meta) {
        if header.expiration == 0 {
            self.memory.alter_all(|_key, mut value| {
                value.header.expiration = header.expiration;
                value
            });
        } else {
            self.memory.clear();
        }
        
    }
}

#[cfg(test)]
mod storage_tests;
