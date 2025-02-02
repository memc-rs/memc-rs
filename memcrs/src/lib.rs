#[macro_use]
extern crate log;

extern crate num_derive;
pub mod cache;
pub mod memcache;
pub mod memcache_server;
pub mod memory_store;
pub mod protocol;
pub mod server;
pub mod version;

#[cfg(any(test, feature = "criterion"))]
pub mod mock;
