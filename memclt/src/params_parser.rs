use byte_unit::Byte;
use clap::Parser;
use std::{fmt::Debug, net::IpAddr, ops::RangeInclusive};

const DEFAULT_PORT: u16 = 11211;
const DEFAULT_ADDRESS: &str = "127.0.0.1";
const MAX_ITEM_SIZE: &str = "1KiB";

#[derive(Parser, Debug, Clone)]
#[command(author, version, about, long_about = None)]
/// memcached compatible server implementation in Rust
pub struct MemcacheClientConfig {
    #[arg(short, long, value_name = "PORT", value_parser = port_in_range, default_value_t = DEFAULT_PORT)]
    /// TCP port to listen on
    pub port: u16,

    #[arg(short, long, value_name = "ITEM-SIZE", value_parser = parse_memory_mb, default_value = MAX_ITEM_SIZE)]
    ///  adjusts max item size (min: 1k, max: 1024m)
    pub item_size: u64,

    #[arg(short, long, action = clap::ArgAction::Count, default_value_t = 2)]
    /// sets the level of verbosity
    pub verbose: u8,

    #[arg(short, long, value_name = "address", default_value_t = String::from(DEFAULT_ADDRESS).parse::<IpAddr>().unwrap())]
    /// interface to listen on
    pub server_address: IpAddr,
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

impl MemcacheClientConfig {
    fn from_args(args: Vec<String>) -> Result<MemcacheClientConfig, String> {
        let memcrs_args = MemcacheClientConfig::parse_from(args.iter());
        Ok(memcrs_args)
    }
}

pub fn parse(args: Vec<String>) -> Result<MemcacheClientConfig, String> {
    MemcacheClientConfig::from_args(args)
}
