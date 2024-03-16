use std::env;
extern crate memcrs;

fn main() {
    memcrs::server::main::server_main(env::args());
}
