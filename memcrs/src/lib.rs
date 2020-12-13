#[macro_use]
extern crate log;
#[macro_use]
extern crate failure_derive;
extern crate num_derive;
pub mod server;
pub mod protocol;
pub mod storage;
pub mod version;

#[cfg(test)]
mod mock;