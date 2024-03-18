use clap::ValueEnum;

pub mod moka_store;
pub mod store;

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum, Debug)]
pub enum StoreEngine {
    /// store based on dashmap library
    DashMap,
    /// store based on moka library
    Moka,
}