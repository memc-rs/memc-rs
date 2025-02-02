use crate::cache::eviction_policy::EvictionPolicy;
use crate::memory_store::StoreEngine;
use byte_unit::Byte;
use clap::{command, Parser, ValueEnum};
use std::{fmt::Debug, net::IpAddr, ops::RangeInclusive};

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum, Debug)]
pub enum RuntimeType {
    /// every thread will create its own runtime which will handle work without thread switching
    CurrentThread,
    /// work stealing threadpool runtime
    MultiThread,
}

impl RuntimeType {
    pub fn as_str(&self) -> &'static str {
        match self {
            RuntimeType::CurrentThread => "Work handled withing current thread runtime",
            RuntimeType::MultiThread => "Work stealing threadpool runtime",
        }
    }
}

const DEFAULT_PORT: u16 = 11211;
const DEFAULT_ADDRESS: &str = "127.0.0.1";
const CONNECTION_LIMIT: u32 = 1024;
const LISTEN_BACKLOG: u32 = 1024;
const MEMORY_LIMIT: &str = "64MiB";
const MAX_ITEM_SIZE: &str = "1MiB";

fn get_default_threads_number() -> usize {
    num_cpus::get_physical().to_string().parse().unwrap()
}

#[derive(Parser, Debug, Clone)]
#[command(author, version, about, long_about = None)]
/// memcached compatible server implementation in Rust
pub struct MemcrsdConfig {
    #[arg(short, long, value_name = "PORT", value_parser = port_in_range, default_value_t = DEFAULT_PORT)]
    /// TCP port to listen on
    pub port: u16,

    #[arg(short, long, value_name = "CONNECTION-LIMIT", default_value_t = CONNECTION_LIMIT)]
    /// max simultaneous connections
    pub connection_limit: u32,

    #[arg(short, long, value_name = "LISTEN-BACKLOG", default_value_t = LISTEN_BACKLOG)]
    /// set the backlog queue limit
    pub backlog_limit: u32,

    #[arg(short, long, value_name = "MEMORY-LIMIT", value_parser = parse_memory_mb, default_value = MEMORY_LIMIT)]
    /// memory limit in megabytes
    pub memory_limit: u64,

    #[arg(short, long, value_name = "MAX-ITEM-SIZE", value_parser = parse_memory_mb, default_value = MAX_ITEM_SIZE)]
    ///  adjusts max item size (min: 1k, max: 1024m)
    pub item_size_limit: u64,

    #[arg(short, long, value_name = "THREADS", default_value_t = get_default_threads_number())]
    /// number of threads to use (defualts to number of cores)
    pub threads: usize,

    #[arg(short, long, action = clap::ArgAction::Count, default_value_t = 1)]
    /// sets the level of verbosity
    pub verbose: u8,

    #[arg(short, long, value_name = "listen", default_value_t = String::from(DEFAULT_ADDRESS).parse::<IpAddr>().unwrap())]
    /// interface to listen on
    pub listen_address: IpAddr,

    #[arg(short, long, value_name = "RUNTIME-TYPE", default_value_t = RuntimeType::CurrentThread, value_enum)]
    ///  runtime type to use
    pub runtime_type: RuntimeType,

    #[arg(short, long, value_name = "EVICTION-POLICY", value_parser = parse_eviction_policy, default_value_t = EvictionPolicy::None, value_enum)]
    /// eviction policy to use
    pub eviction_policy: EvictionPolicy,

    #[arg(short, long, value_name = "STORE-ENGINE",  verbatim_doc_comment, value_parser = parse_store_engine, default_value_t = StoreEngine::DashMap, value_enum)]
    /// store engine to be used
    ///
    /// Possible values:
    /// - dash-map: store will use dash-map implementation
    /// - moka: store will use moka implementation 
    pub store_engine: StoreEngine,
}

const PORT_RANGE: RangeInclusive<usize> = 1..=65535;

fn port_in_range(s: &str) -> Result<u16, String> {
    let port: usize = s
        .parse()
        .map_err(|_| format!("`{s}` isn't a port number"))?;
    if PORT_RANGE.contains(&port) {
        Ok(port as u16)
    } else {
        Err(format!(
            "port not in range {}-{}",
            PORT_RANGE.start(),
            PORT_RANGE.end()
        ))
    }
}

fn parse_memory_mb(s: &str) -> Result<u64, String> {
    match Byte::parse_str(s, true) {
        Ok(bytes) => Ok(bytes.as_u64()),
        Err(byte_error) => Err(format!("{}", byte_error)),
    }
}

fn parse_eviction_policy(s: &str) -> Result<EvictionPolicy, String> {
    match s {
        "tiny_lfu" => Ok(EvictionPolicy::TinyLeastFrequentlyUsed),
        "lru" => Ok(EvictionPolicy::LeastRecentylUsed),
        "none" => Ok(EvictionPolicy::None),
        _ => Err(format!("Invalid eviction policy: {}", s)),
    }
}

fn parse_store_engine(s: &str) -> Result<StoreEngine, String> {
    match s {
        "moka" => Ok(StoreEngine::Moka),
        "dash-map" => Ok(StoreEngine::DashMap),
        _ => Err(format!("Invalid store engine selected: {}", s)),
    }
}

impl MemcrsdConfig {
    fn from_args(args: Vec<String>) -> Result<MemcrsdConfig, String> {
        let memcrs_args = MemcrsdConfig::parse_from(args.iter());
        Ok(memcrs_args)
    }
}

pub fn parse(args: Vec<String>) -> Result<MemcrsdConfig, String> {
    MemcrsdConfig::from_args(args)
}

#[cfg(test)]
mod tests {
    use std::error::Error;

    use super::*;
    use clap::CommandFactory;
    #[test]
    fn verify_cli() {
        MemcrsdConfig::command().debug_assert()
    }

    #[test]
    fn test_default_config() {
        // Test if the default values are parsed correctly
        let args: Vec<String> = vec![];
        let config = parse(args).unwrap();

        // Assert default values
        assert_eq!(config.port, DEFAULT_PORT);
        assert_eq!(config.connection_limit, CONNECTION_LIMIT);
        assert_eq!(config.backlog_limit, LISTEN_BACKLOG);
        assert_eq!(config.memory_limit, parse_memory_mb(MEMORY_LIMIT).unwrap());
        assert_eq!(config.item_size_limit, parse_memory_mb(MAX_ITEM_SIZE).unwrap());
        assert_eq!(config.threads, get_default_threads_number());
        assert_eq!(config.verbose, 1);
        assert_eq!(config.listen_address, DEFAULT_ADDRESS.parse::<IpAddr>().unwrap());
        assert_eq!(config.runtime_type, RuntimeType::CurrentThread);
        assert_eq!(config.eviction_policy, EvictionPolicy::None);
        assert_eq!(config.store_engine, StoreEngine::DashMap);
    }

    #[test]
    fn test_custom_port() {
        // Test if a custom port value is parsed correctly
        let args = vec!["".to_string(), "--port".to_string(), "8080".to_string()];
        let config = parse(args).unwrap();

        assert_eq!(config.port, 8080);
    }

    #[test]
    fn test_invalid_port() {
        // Test if an invalid port gives an error
        let args = vec!["".to_string(), "--port".to_string(), "70000".to_string()];
        let result = MemcrsdConfig::try_parse_from(args);
        assert!(result.is_err());

        let error = result.unwrap_err();
        let source = error.source().unwrap();
        assert_eq!(source.to_string(), "port not in range 1-65535");
    }

    #[test]
    fn test_memory_limit_parsing() {
        // Test if the memory limit is parsed correctly
        let args = vec!["".to_string(), "--memory-limit".to_string(), "128MiB".to_string()];
        let config = parse(args).unwrap();

        assert_eq!(config.memory_limit, parse_memory_mb("128MiB").unwrap());
    }

    #[test]
    fn test_invalid_memory_limit() {
        // Test if an invalid memory limit results in an error
        let args = vec!["".to_string(), "--memory-limit".to_string(), "invalid".to_string()];
        let result = MemcrsdConfig::try_parse_from(args);

        assert!(result.is_err());
    }

    #[test]
    fn test_runtime_type() {
        // Test if the runtime type is parsed correctly
        let args = vec!["".to_string(), "--runtime-type".to_string(), "multi-thread".to_string()];
        let config = MemcrsdConfig::try_parse_from(args).unwrap();

        assert_eq!(config.runtime_type, RuntimeType::MultiThread);
    }

    #[test]
    fn test_eviction_policy() {
        // Test if the eviction policy is parsed correctly
        let args = vec!["".to_string(), "--eviction-policy".to_string(), "lru".to_string()];
        let config = MemcrsdConfig::try_parse_from(args).unwrap();

        assert_eq!(config.eviction_policy, EvictionPolicy::LeastRecentylUsed);
    }

    #[test]
    fn test_invalid_eviction_policy() {
        // Test if an invalid eviction policy results in an error
        let args = vec!["".to_string(), "--eviction-policy".to_string(), "invalid-policy".to_string()];
        let result = MemcrsdConfig::try_parse_from(args);

        assert!(result.is_err());
        let error = result.unwrap_err();
        let source = error.source().unwrap();
        assert_eq!(source.to_string(), "Invalid eviction policy: invalid-policy");
    }

    #[test]
    fn test_store_engine() {
        // Test if the store engine is parsed correctly
        let args = vec!["".to_string(), "--store-engine".to_string(), "moka".to_string()];
        let config = parse(args).unwrap();

        assert_eq!(config.store_engine, StoreEngine::Moka);
    }

    #[test]
    fn test_invalid_store_engine() {
        // Test if an invalid store engine results in an error
        let args = vec!["".to_string(), "--store-engine".to_string(), "invalid-store".to_string()];
        let result = MemcrsdConfig::try_parse_from(args);

        assert!(result.is_err());
        let error = result.unwrap_err();
        let source = error.source().unwrap();
        assert_eq!(source.to_string(), "Invalid store engine selected: invalid-store");
    }

    #[test]
    fn test_verbose_flag() {
        // Test if the verbose flag is parsed correctly
        let args = vec!["".to_string(), "--verbose".to_string(), "--verbose".to_string()];
        let config = parse(args).unwrap();

        assert_eq!(config.verbose, 2);
    }

}
