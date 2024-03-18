use super::eviction_policy::EvictionPolicy;

/// Read only view over a store
pub trait CacheCapability {
  fn is_policy_supported(&self, policy: EvictionPolicy) -> bool;
}
