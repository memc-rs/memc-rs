use clap::ValueEnum;

pub mod dash_map_store;
pub mod moka_store;
mod parallelism;
pub mod shared_store_state;

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum, Debug)]
pub enum StoreEngine {
    /// store based on dashmap library
    DashMap,
    /// store based on moka library
    Moka,
}

impl StoreEngine {
    pub fn as_str(&self) -> &'static str {
        match self {
            StoreEngine::DashMap => "DashMap backend",
            StoreEngine::Moka => "Moka backend",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::StoreEngine;

    #[test]
    fn test_as_str() {
        assert_eq!(StoreEngine::DashMap.as_str(), "DashMap backend");
        assert_eq!(StoreEngine::Moka.as_str(), "Moka backend");
    }

    #[test]
    fn test_enum_ordering() {
        assert!(StoreEngine::DashMap < StoreEngine::Moka);
    }

    #[test]
    fn test_enum_equality() {
        assert_eq!(StoreEngine::DashMap, StoreEngine::DashMap);
        assert_eq!(StoreEngine::Moka, StoreEngine::Moka);
        assert_ne!(StoreEngine::DashMap, StoreEngine::Moka);
    }
}
