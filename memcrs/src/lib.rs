#[macro_use]
extern crate log;

extern crate failure_derive;
extern crate num_derive;
pub mod protocol;
pub mod server;
pub mod storage;
pub mod version;

#[cfg(test)]
mod mock;
