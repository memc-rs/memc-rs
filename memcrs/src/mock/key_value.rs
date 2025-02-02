use bytes::{Bytes, BytesMut, BufMut};
use rand::Rng;

pub struct KeyValue {
    pub key: Bytes,
    pub value: Bytes,
}

pub fn generate_random_with_max_size(capacity: usize, max_key_size: usize, max_value_size: usize) -> Vec<KeyValue> {
    let mut values: Vec<KeyValue> = Vec::with_capacity(capacity);
    let mut rng = rand::rng();
    for _idx in 0..capacity {
        let key_size = rng.random_range(5..max_key_size);
        let value_size = rng.random_range(5..max_value_size);
        let key = create_random_value(key_size);
        let value = create_random_value(value_size);
        values.push(KeyValue { key, value });
    }
    values
}

pub fn generate_random_with_size(capacity: usize, key_size: usize, value_size: usize) -> Vec<KeyValue> {
    let mut values: Vec<KeyValue> = Vec::with_capacity(capacity);
    for _idx in 0..capacity {
        let key = create_random_value(key_size);
        let value = create_random_value(value_size);
        values.push(KeyValue { key, value });
    }
    values
}

pub fn create_random_value(capacity: usize) -> Bytes {
    let mut rng = rand::rng();
    let mut value = BytesMut::with_capacity(capacity);
    for _ in 0..capacity {
        let random_char = rng.random_range(b'a'..=b'z') as u8;
        value.put_u8(random_char);
    }
    value.freeze()
}
