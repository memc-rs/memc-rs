use crate::cache::cache::ValueType;
use bytes::{BufMut, BytesMut};
use std::str;

pub fn from_string(val: &str) -> ValueType {
    let mut value = BytesMut::with_capacity(val.len());
    value.put_slice(val.as_bytes());
    value.freeze()
}

pub fn from_slice(val: &[u8]) -> ValueType {
    let mut value = BytesMut::with_capacity(val.len());
    value.put_slice(val);
    value.freeze()
}
