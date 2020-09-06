use super::*;
use std::sync::atomic::{AtomicUsize, Ordering};

struct MockSystemTimer {
    current_time: AtomicUsize,
}

trait SetableTimer: timer::Timer {
    fn set(&self, time: u64);
}

impl MockSystemTimer {
    pub fn new() -> Self {
        MockSystemTimer {
            current_time: AtomicUsize::new(0),
        }
    }
}

impl timer::Timer for MockSystemTimer {
    fn secs(&self) -> u64 {
        self.current_time.load(Ordering::Relaxed) as u64
    }
}

impl SetableTimer for MockSystemTimer {
    fn set(&self, time: u64) {
        self.current_time.store(time as usize, Ordering::Relaxed)
    }
}

struct MockServer {
    pub timer: Arc<MockSystemTimer>,
    pub storage: Storage,
}

impl MockServer {
    pub fn new() -> Self {
        let timer = Arc::new(MockSystemTimer::new());
        MockServer {
            timer: timer.clone(),
            storage: Storage::new(timer),
        }
    }
}

fn create_server() -> MockServer {
    MockServer::new()
}

#[test]
fn if_not_defined_cas_should_be_1() {
    let server = create_server();
    let key = String::from("key").into_bytes();
    let record = Record::new(String::from("Test data").into_bytes(), 0, 0, 0);
    let result = server.storage.set(key.clone(), record.clone());
    assert!(result.is_ok());
    let found = server.storage.get(&key);
    assert!(found.is_ok());
    match found {
        Ok(r) => {
            assert_eq!(r, record);
            assert_eq!(r.header.cas, 1)
        }
        Err(_er) => unreachable!(),
    }
}

#[test]
fn if_cas_defined_it_should_be_returned() {
    let storage = create_server().storage;
    let cas: u64 = 0xDEAD_BEEF;
    let key = String::from("key").into_bytes();
    let record = Record::new(String::from("Test data").into_bytes(), cas, 0, 0);
    info!("Record {:?}", &record.header);
    let result = storage.set(key.clone(), record.clone());
    assert!(result.is_ok());
    let found = storage.get(&key);
    assert!(found.is_ok());
    match found {
        Ok(r) => {
            assert_eq!(r, record);
            assert_eq!(r.header.cas, cas)
        }
        Err(_er) => unreachable!(),
    }
}

#[test]
fn insert_should_fail_on_cas_mismatch() {
    let storage = create_server().storage;
    let cas: u64 = 0xDEAD_BEEF;
    let key = String::from("key").into_bytes();
    let mut record = Record::new(String::from("Test data").into_bytes(), cas, 0, 0);
    let result = storage.set(key.clone(), record.clone());
    assert!(result.is_ok());
    record.header.cas = 1;
    let result = storage.set(key, record);
    match result {
        Ok(_) => unreachable!(),
        Err(err) => assert_eq!(err, StorageError::KeyExists),
    }
}

#[test]
fn record_should_expire_in_given_time() {
    let server = create_server();
    let cas: u64 = 0xDEAD_BEEF;
    let key = String::from("key").into_bytes();
    let record = Record::new(String::from("Test data").into_bytes(), cas, 0, 123);
    let result = server.storage.set(key.clone(), record.clone());
    assert!(result.is_ok());
    println!("{:?}", result);
    let found = server.storage.get(&key);
    assert!(found.is_ok());

    server.timer.set(128);
    let found = server.storage.get(&key);
    assert!(found.is_err());
    match found {
        Ok(_r) => unreachable!(),
        Err(err) => assert_eq!(err, StorageError::NotFound),
    }
}

#[test]
fn delete_record() {
    let server = create_server();
    let key = String::from("key").into_bytes();
    let record = Record::new(String::from("Test data").into_bytes(), 0, 0, 0);
    let result = server.storage.set(key.clone(), record.clone());
    assert!(result.is_ok());
    let found = server.storage.get(&key);
    assert!(found.is_ok());
    let header = Header::new(0, 0, 0);
    let deleted = server.storage.delete(key.clone(), header);
    match deleted {
        Ok(_) => match server.storage.get(&key) {
            Ok(_) => unreachable!(),
            Err(err) => assert_eq!(err, StorageError::NotFound),
        },
        Err(_err) => unreachable!(),
    }
}

#[test]
fn delete_should_return_not_exists() {
    let server = create_server();
    let key = String::from("key").into_bytes();
    let record = Record::new(String::from("Test data").into_bytes(), 0, 0, 0);
    let result = server.storage.set(key.clone(), record.clone());
    assert!(result.is_ok());
    let found = server.storage.get(&key);
    assert!(found.is_ok());
    let header = Header::new(0, 0, 0);
    let deleted = server
        .storage
        .delete(String::from("bad key").into_bytes(), header);
    match deleted {
        Ok(_) => unreachable!(),
        Err(err) => assert_eq!(err, StorageError::NotFound),
    }
}

#[test]
fn delete_if_cas_doesnt_match_should_not_delete() {
    let server = create_server();
    let key = String::from("key").into_bytes();
    let record = Record::new(String::from("Test data").into_bytes(), 1, 0, 0);
    let result = server.storage.set(key.clone(), record);
    assert!(result.is_ok());
    let found = server.storage.get(&key);
    assert!(found.is_ok());
    let header = Header::new(2, 0, 0);
    let deleted = server
        .storage
        .delete(String::from("key").into_bytes(), header);
    match deleted {
        Ok(_) => unreachable!(),
        Err(err) => assert_eq!(err, StorageError::KeyExists),
    }
}

#[test]
fn delete_if_cas_match_should_succeed() {
    let server = create_server();
    let key = String::from("key").into_bytes();
    let record = Record::new(String::from("Test data").into_bytes(), 5, 0, 0);
    let result = server.storage.set(key.clone(), record);
    assert!(result.is_ok());
    let found = server.storage.get(&key);
    assert!(found.is_ok());
    let header = Header::new(5, 0, 0);
    let deleted = server
        .storage
        .delete(String::from("key").into_bytes(), header);
    assert!(deleted.is_ok());
}

#[test]
fn flush_should_remove_all_elements_in_cache() {
    let server = create_server();
    for key_suffix in 1..10 {
        let mut key_str = String::from("key");
        key_str.push_str(&key_suffix.to_string());
        let key = key_str.into_bytes();
        let record = Record::new(String::from("Test data").into_bytes(), 0, 0, 5);
        let result = server.storage.set(key.clone(), record);
        assert!(result.is_ok());
    }

    server.storage.flush(Header::new(0, 0, 3));
    server.timer.set(10);

    for key_suffix in 1..10 {
        let mut key_str = String::from("key");
        key_str.push_str(&key_suffix.to_string());
        let key = key_str.into_bytes();
        let result = server.storage.get(&key);
        match result {
            Ok(_) => unreachable!(),
            Err(err) => assert_eq!(err, StorageError::NotFound),
        }
    }
}

#[test]
fn add_should_succeed_if_not_already_stored() {
    let server = create_server();
    let key = String::from("key").into_bytes();
    let record = Record::new(String::from("Test data").into_bytes(), 5, 0, 0);
    let result = server.storage.add(key.clone(), record);
    assert!(result.is_ok());    
}

#[test]
fn add_should_fail_if_already_stored() {
    let server = create_server();
    let key = String::from("key").into_bytes();
    let record = Record::new(String::from("Test data").into_bytes(), 5, 0, 0);
    let result = server.storage.set(key.clone(), record.clone());
    assert!(result.is_ok());    
    let add_result = server.storage.add(key.clone(), record);
    match add_result {
        Ok(_) => unreachable!(),
        Err(err) => assert_eq!(err, StorageError::KeyExists)
    }
}

#[test]
fn replace_should_fail_if_not_stored() {
    let server = create_server();
    let key = String::from("key").into_bytes();
    let record = Record::new(String::from("Test data").into_bytes(), 5, 0, 0);
    let result = server.storage.replace(key.clone(), record);
    match result {
        Ok(_) => unreachable!(),
        Err(err) => assert_eq!(err, StorageError::NotFound)
    }    
}

#[test]
fn replace_should_succeed_if_stored() {
    let server = create_server();
    let key = String::from("key").into_bytes();
    let record = Record::new(String::from("Test data").into_bytes(), 0, 0, 0);
    let result = server.storage.set(key.clone(), record);
    assert!(result.is_ok());
    match result {
        Ok(status) => {
            let new_record = Record::new(String::from("New record").into_bytes(), status.cas, 0, 0);
            let replace_result = server.storage.replace(key.clone(), new_record);
            assert!(replace_result.is_ok());
        },
        Err(_) => unreachable!()
    }    
}

#[test]
fn append_should_fail_if_not_exist() {
    let server = create_server();
    let key = String::from("key").into_bytes();
    let record = Record::new(String::from("Test data").into_bytes(), 0, 0, 0);
    let result = server.storage.append(key.clone(), record);
    
    match result {
        Ok(status) => unreachable!(),
        Err(err) => assert_eq!(err, StorageError::NotFound)
    }    
}

#[test]
fn prepend_should_fail_if_not_exist() {
    let server = create_server();
    let key = String::from("key").into_bytes();
    let record = Record::new(String::from("Test data").into_bytes(), 0, 0, 0);
    let result = server.storage.prepend(key.clone(), record);
    
    match result {
        Ok(_) => unreachable!(),
        Err(err) => assert_eq!(err, StorageError::NotFound)
    }    
}

#[test]
fn append_should_add_at_the_end() {
    let server = create_server();
    let key = String::from("key").into_bytes();
    let record = Record::new(String::from("Foo").into_bytes(), 0, 0, 0);
    let result = server.storage.set(key.clone(), record);
    
    match result {
        Ok(status) => {
            let append_data = Record::new(String::from("bar").into_bytes(), status.cas, 0, 0);
            let append_result = server.storage.append(key.clone(), append_data);
            assert!(append_result.is_ok());
            let get_result = server.storage.get(&key);
            match get_result {
                Ok(record) => {
                    let value = String::from("Foobar").into_bytes();                    
                    assert_eq!(record.value, value);
                },
                Err(_) => unreachable!()

            }
        },
        Err(_) => unreachable!()
    }    
}

#[test]
fn prepend_should_add_at_the_begining() {
    let server = create_server();
    let key = String::from("key").into_bytes();
    let record = Record::new(String::from("Foo").into_bytes(), 0, 0, 0);
    let result = server.storage.set(key.clone(), record);
    
    match result {
        Ok(status) => {
            let append_data = Record::new(String::from("bar").into_bytes(), status.cas, 0, 0);
            let append_result = server.storage.prepend(key.clone(), append_data);
            assert!(append_result.is_ok());
            let get_result = server.storage.get(&key);
            match get_result {
                Ok(record) => {
                    let value = String::from("barFoo").into_bytes();                    
                    assert_eq!(record.value, value);
                },
                Err(_) => unreachable!()

            }
        },
        Err(_) => unreachable!()
    }    
}
