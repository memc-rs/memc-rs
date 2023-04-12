#[macro_use]
extern crate log;

extern crate num_derive;
pub mod memcache;
pub mod protocol;
pub mod memcache_server;
pub mod cache;
pub mod version;
pub mod memory_store;

#[cfg(test)]
mod mock;
