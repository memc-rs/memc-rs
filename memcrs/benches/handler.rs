use std::hint::black_box;
use criterion::{criterion_group, criterion_main, Criterion};
use bytes::Bytes;
use memcrs::memcache_server::handler::mock::create_moka_handler;
use memcrs::protocol::binary::binary;
use memcrs::protocol::binary::encoder;
use memcrs::memcache_server::handler::BinaryHandler;
use memcrs::memcache_server::handler::mock::{create_header, insert_value, create_dash_map_handler, create_get_request};
use memcrs::mock::value::from_string;


fn test_get(handler: &BinaryHandler, key: &Bytes, header: binary::RequestHeader) {

    let request = create_get_request(header, key.clone());
    let result = handler.handle_request(request);

    match result {
        Some(resp) => {
            if let encoder::BinaryResponse::Get(_response) = resp {
                assert_eq!(0, 0);
            } else {
                unreachable!();
            }
        }
        None => unreachable!(),
    }
}

fn criterion_benchmark(c: &mut Criterion) {
    let key = Bytes::from("test_key");
    let value = from_string("test value");
    let header = create_header(binary::Command::GetKey, &key);

    
    let dash_map_handler: BinaryHandler = create_dash_map_handler();
    let moka_handler: BinaryHandler = create_moka_handler();
    insert_value(&dash_map_handler, key.clone(), value.clone());
    insert_value(&moka_handler, key.clone(), value.clone());
    c.bench_function("test_get test_key dash map", |b| b.iter(|| test_get(black_box(&dash_map_handler, ), black_box(&key), black_box(header) )));
    c.bench_function("test_get test_key moka", |b| b.iter(|| test_get(black_box(&moka_handler, ), black_box(&key), black_box(header) )));
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);