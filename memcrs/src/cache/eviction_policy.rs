use clap::ValueEnum;

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum EvictionPolicy {
    None,
    TinyLeastFrequentlyUsed,
    LeastRecentylUsed
}

impl EvictionPolicy {
    pub fn as_str(&self) -> &'static str {
        match self {
            EvictionPolicy::None => "None",
            EvictionPolicy::TinyLeastFrequentlyUsed => "Tiny LFU",
            EvictionPolicy::LeastRecentylUsed => "LFU",
        }
    }
}