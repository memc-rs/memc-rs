#[macro_use]
extern crate log;


extern crate num_derive;
pub mod memcache;
pub mod protocol;
pub mod server;
pub mod storage;
pub mod version;

#[cfg(test)]
mod mock;
