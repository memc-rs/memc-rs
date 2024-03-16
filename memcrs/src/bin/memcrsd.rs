use std::env;
extern crate memcrs;

fn main() {
    memcrs::server::main::run(env::args().collect());
}
