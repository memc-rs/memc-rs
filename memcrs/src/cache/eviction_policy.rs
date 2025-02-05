use clap::ValueEnum;

#[derive(Debug, Clone, Copy, ValueEnum, PartialEq)]
pub enum EvictionPolicy {
    None,
    TinyLeastFrequentlyUsed,
    LeastRecentylUsed,
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

#[cfg(test)]
mod tests {
    use super::*;
    use clap::ValueEnum;

    #[test]
    fn test_eviction_policy_as_str() {
        assert_eq!(EvictionPolicy::None.as_str(), "None");
        assert_eq!(EvictionPolicy::TinyLeastFrequentlyUsed.as_str(), "Tiny LFU");
        assert_eq!(EvictionPolicy::LeastRecentylUsed.as_str(), "LFU");
    }

    #[test]
    fn test_eviction_policy_value_enum() {
        let none = EvictionPolicy::from_str("none", true).unwrap();
        let tiny_lfu = EvictionPolicy::from_str("tiny-least-frequently-used", true).unwrap();
        let lru = EvictionPolicy::from_str("least-recentyl-used", true).unwrap();
        
        assert_eq!(none, EvictionPolicy::None);
        assert_eq!(tiny_lfu, EvictionPolicy::TinyLeastFrequentlyUsed);
        assert_eq!(lru, EvictionPolicy::LeastRecentylUsed);
    }
}
