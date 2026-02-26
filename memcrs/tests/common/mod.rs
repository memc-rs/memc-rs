use rand::RngExt;

mod multi_thread_server;
mod params_builder;
mod process_server;
mod random_port;

pub use multi_thread_server::spawn_server;
pub use params_builder::MemcrsdServerParamsBuilder;

#[allow(dead_code)]
pub fn create_value_with_size(size: usize) -> String {
    let mut rng = rand::rng();
    let mut value = String::with_capacity(size);
    for _ in 0..size {
        let random_char = rng.random_range(b'a'..=b'z') as char;
        value.push(random_char);
    }
    value
}
