use bytes::{BufMut, Bytes, BytesMut};
use rand::RngExt;

pub struct KeyValue {
    pub key: Bytes,
    pub value: Bytes,
}

pub fn generate_random_with_max_size(
    capacity: usize,
    max_key_size: usize,
    max_value_size: usize,
) -> Vec<KeyValue> {
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

pub fn generate_random_with_size(
    capacity: usize,
    key_size: usize,
    value_size: usize,
) -> Vec<KeyValue> {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_random_value() {
        let size = 10;
        let value = create_random_value(size);
        assert_eq!(value.len(), size);
        for &byte in value.iter() {
            assert!(byte >= b'a' && byte <= b'z', "Byte out of range: {}", byte);
        }
    }

    #[test]
    fn test_generate_random_with_size() {
        let capacity = 5;
        let key_size = 8;
        let value_size = 12;
        let result = generate_random_with_size(capacity, key_size, value_size);

        assert_eq!(result.len(), capacity);
        for kv in result {
            assert_eq!(kv.key.len(), key_size);
            assert_eq!(kv.value.len(), value_size);
        }
    }

    #[test]
    fn test_generate_random_with_max_size() {
        let capacity = 5;
        let max_key_size = 15;
        let max_value_size = 20;
        let result = generate_random_with_max_size(capacity, max_key_size, max_value_size);

        assert_eq!(result.len(), capacity);
        for kv in result {
            assert!(kv.key.len() >= 5 && kv.key.len() <= max_key_size);
            assert!(kv.value.len() >= 5 && kv.value.len() <= max_value_size);
        }
    }
}
