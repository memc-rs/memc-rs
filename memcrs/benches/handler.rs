use criterion::{BenchmarkId, Throughput};
use criterion::{criterion_group, criterion_main, Criterion};
use bytes::{BufMut, Bytes, BytesMut};
use rand::Rng;
use memcrs::memcache_server::handler::mock::{create_get_request_by_key, create_moka_handler};
use memcrs::protocol::binary::encoder;
use memcrs::memcache_server::handler::BinaryHandler;
use memcrs::memcache_server::handler::mock::{create_dash_map_handler, create_set_request};

struct KeyValue {
    pub key: Bytes,
    pub value: Bytes
}

fn generate_random_key_values(capacity: usize) -> Vec<KeyValue> {
    let mut values: Vec<KeyValue> = Vec::with_capacity(capacity);
    for _idx in 0..capacity {
        let key = create_random_value(200);
        let value = create_random_value(1024);
        values.push(KeyValue {key, value});
    }
    values
}

pub fn create_random_value(capacity: usize) -> Bytes {
    let mut rng = rand::thread_rng();
    let mut value =  BytesMut::with_capacity(capacity);
    for _ in 0..capacity {
        let random_char = rng.gen_range(b'a'..=b'z') as u8;
        value.put_u8(random_char);
    }
    value.freeze()
}

fn test_get(handler: &BinaryHandler, key: &Bytes) {
    let request = create_get_request_by_key(key);
    let result = handler.handle_request(request);
    match result {
        Some(resp) => {
            if let encoder::BinaryResponse::Get(_response) = resp {
                assert_eq!(0, 0);
            } else if let  encoder::BinaryResponse::Error(_error) = resp {
                assert_eq!(0, 0);
            } else {
                unreachable!();
            }
        }
        None => unreachable!(),
    }
}

fn test_set(handler: &BinaryHandler, key: Bytes, value: Bytes) {
    let request = create_set_request(key, value);
    let result = handler.handle_request(request);
    match result {
        Some(resp) => {
            if let encoder::BinaryResponse::Set(_response) = resp {
                assert_eq!(0, 0);
            } else if let  encoder::BinaryResponse::Error(_error) = resp {
                assert_eq!(0, 0);
            } else {
                unreachable!();
            }
        }
        None => unreachable!(),
    }
}

fn criterion_simple_random_get(c: &mut Criterion) {
    static KB: usize = 1024;
    let dash_map_handler: BinaryHandler = create_dash_map_handler();
    let moka_handler: BinaryHandler = create_moka_handler();
    
    let mut group = c.benchmark_group("criterion_simple_random_get");
    for size in [KB, 2*KB, 4*KB].iter() {
        let values = generate_random_key_values(*size);
        let not_existing_values =  generate_random_key_values(*size);
        values.iter().for_each(|key_value| {
            test_set(&dash_map_handler, key_value.key.clone(), key_value.value.clone());
            test_set(&moka_handler, key_value.key.clone(), key_value.value.clone());
        });

        group.throughput(Throughput::Elements((*size*2) as u64));
        group.bench_with_input(
                BenchmarkId::new("dash_map", (2*size).to_string()),
                &values, |b, values| {
                b.iter(|| -> () {
                    not_existing_values.iter().for_each(|key_value| {
                        test_get(&dash_map_handler, &key_value.key);
                    });
                    values.iter().for_each(|key_value | -> () {
                        test_get(&dash_map_handler, &key_value.key);
                    });
                } 
            );
        });

        group.bench_with_input(
            BenchmarkId::new("moka", (2*size).to_string()),
            &values, |b, values| {
            b.iter(|| -> () {
                not_existing_values.iter().for_each(|key_value| {
                    test_get(&dash_map_handler, &key_value.key);
                });
                values.iter().for_each(|key_value | -> () {
                    test_get(&moka_handler, &key_value.key);
                });
            } 
            );
        });
    }
    group.finish();
}

fn criterion_simple_random_set(c: &mut Criterion) {
    static KB: usize = 1024;
    let dash_map_handler: BinaryHandler = create_dash_map_handler();
    let moka_handler: BinaryHandler = create_moka_handler();
    
    let mut group = c.benchmark_group("criterion_simple_random_set");
    for size in [KB, 2*KB, 4*KB].iter() {
        group.throughput(Throughput::Elements(*size as u64));
        let values = generate_random_key_values(*size);
        group.bench_with_input(
                BenchmarkId::new("dash_map", size.to_string()),
                &values, |b, values| {
                b.iter(|| -> () {
                    values.iter().for_each(|key_value | -> () {
                        test_set(&dash_map_handler, key_value.key.clone(), key_value.value.clone())
                    });
                } 
            );
        });

        group.bench_with_input(
            BenchmarkId::new("moka", size.to_string()),
            &values, |b, values| {
            b.iter(|| -> () {
                values.iter().for_each(|key_value | -> () {
                    test_set(&moka_handler, key_value.key.clone(), key_value.value.clone())
                });
            } 
            );
        });
    }
    group.finish();
}


criterion_group!(benches, criterion_simple_random_get, criterion_simple_random_set);
criterion_main!(benches);
