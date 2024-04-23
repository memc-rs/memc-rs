use clap::ValueEnum;

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum EvictionPolicy {
    None,
    TinyLeastFrequentlyUsed,
    LeastRecentylUsed
}
