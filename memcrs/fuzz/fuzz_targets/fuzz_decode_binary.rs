#![no_main]
use libfuzzer_sys::fuzz_target;
extern crate memcrs;
use bytes::{BytesMut, BufMut};
use tokio_util::codec::{Decoder};

fuzz_target!(|data: &[u8]| {
    // fuzzed code goes here
    let mut codec = memcrs::protocol::binary_codec::MemcacheBinaryCodec::new(1024);
    let mut src = BytesMut::with_capacity(data.len());
    src.put(data);
    let _ = codec.decode(&mut src);
});
