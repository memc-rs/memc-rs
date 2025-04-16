use super::eviction_policy::EvictionPolicy;

/// Cache capabilities
pub trait CacheCapability {
    fn is_policy_supported(&self, policy: EvictionPolicy) -> bool;
}
