use std::net::IpAddr;

use crate::version;
use byte_unit::{Byte, ByteUnit};
use clap::{command, Arg};

pub struct MemcrsArgs {
    pub port: u16,
    pub connection_limit: u32,
    pub backlog_limit: u32,
    pub memory_limit_mb: u64,
    pub item_size_limit: Byte,
    pub memory_limit: u64,
    pub runtimes: u32,
    pub log_level: tracing::Level,
    pub listen_address: IpAddr,
}

impl MemcrsArgs {
    fn from_args(runtimes: String, args: Vec<String>) -> Result<MemcrsArgs, String> {
        let matches = cli_args(&runtimes).get_matches_from(args);

        let port: u16 = match matches.get_one::<u16>("port") {
            Some(value) => *value,
            None => return Err("Port was not defined".to_string()),
        };

        let connection_limit: u32 = match matches.get_one::<u32>("conn-limit") {
            Some(value) => *value,
            None => return Err("Connection limit not defined".to_string()),
        };

        let backlog_limit: u32 = match matches.get_one::<u32>("listen-backlog") {
            Some(value) => *value,
            None => return Err("Listeng backlog not defined".to_string()),
        };

        let memory_limit_mb: u64 = match matches.get_one::<u64>("memory-limit") {
            Some(value) => *value,
            None => return Err("Memory limit in mega bytes not defined".to_string()),
        };

        let memory_limit_res = Byte::from_unit(memory_limit_mb as f64, ByteUnit::MiB).unwrap();
        let memory_limit: u64 = memory_limit_res.get_bytes() as u64;

        let item_size_limit_str: String = match matches.get_one::<String>("max-item-size") {
            Some(value) => value.clone(),
            None => return Err("Item size limit not defined".to_string()),
        };

        let item_size_limit_res = Byte::from_str(item_size_limit_str).unwrap();
        let item_size_limit_max = Byte::from_unit(1000f64, ByteUnit::MiB).unwrap();

        if item_size_limit_res.get_bytes() > item_size_limit_max.get_bytes() {
            return Err(format!(
                "Max item size cannot be greater than: {}",
                item_size_limit_max.get_appropriate_unit(false).to_string()
            ));
        }

        let runtimes: u32 = match matches.get_one::<u32>("runtimes") {
            Some(value) => *value,
            None => return Err("Invalid number of runtimes defined".to_string()),
        };

        let listen_address = match matches
            .get_one::<String>("listen")
            .unwrap()
            .parse::<IpAddr>()
        {
            Ok(ip_addr) => ip_addr,
            Err(err) => return Err(format!("Invalid ip address: {}", err.to_string())),
        };

        // Vary the output based on how many times the user used the "verbose" flag
        // (i.e. 'myprog -v -v -v' or 'myprog -vvv' vs 'myprog -v'
        let log_level = match matches.get_count("v") {
            0 => tracing::Level::ERROR,
            1 => tracing::Level::WARN,
            2 => tracing::Level::INFO,
            3 => tracing::Level::DEBUG,
            _ => tracing::Level::TRACE,
        };

        Ok(MemcrsArgs {
            port,
            backlog_limit,
            connection_limit,
            item_size_limit: item_size_limit_res,
            listen_address,
            log_level,
            memory_limit,
            memory_limit_mb,
            runtimes,
        })
    }
}

fn cli_args<'help>(runtimes: &'help String) -> clap::Command<'help> {
    command!()
        .version(version::MEMCRS_VERSION)
        .author("Dariusz Ostolski <memc-rs@memc.rs>")
        .about("memcrsd - memcached compatible server implementation in Rust")
        .arg(
            Arg::new("port")
                .short('p')
                .long("port")
                .default_value("11211")
                .help("TCP port to listen on")
                .takes_value(true),
        )
        .arg(
            Arg::new("listen")
                .short('l')
                .long("listen")
                .default_value("127.0.0.1")
                .help("interface to listen on")
                .takes_value(true),
        )
        .arg(
            Arg::new("conn-limit")
                .short('c')
                .long("conn-limit")
                .default_value("1024")
                .help("max simultaneous connections")
                .takes_value(true),
        )
        .arg(
            Arg::new("listen-backlog")
                .short('b')
                .long("listen-backlog")
                .default_value("1024")
                .help("set the backlog queue limit")
                .takes_value(true),
        )
        .arg(
            Arg::new("v")
                .short('v')
                .action(clap::ArgAction::Count)
                .help("Sets the level of verbosity"),
        )
        .arg(
            Arg::new("memory-limit")
                .short('m')
                .long("memory-limit")
                .default_value("64")
                .help("item memory in megabytes")
                .takes_value(true),
        )
        .arg(
            Arg::new("max-item-size")
                .short('I')
                .long("max-item-size")
                .default_value("1m")
                .help("adjusts max item size (min: 1k, max: 1024m)")
                .takes_value(true),
        )
        .arg(
            Arg::new("runtimes")
                .short('r')
                .long("runtimes")
                .default_value(&runtimes)
                .help("number of runtimes to use, each runtime will have n number of threads")
                .takes_value(true),
        )
}

pub fn parse(runtimes: String, args: Vec<String>) -> Result<MemcrsArgs, String> {
    MemcrsArgs::from_args(runtimes, args)
}

#[cfg(test)]
mod tests {
    use crate::memcache::cli::parser::cli_args;
    #[test]
    fn verify_cli() {
        cli_args(&"8".to_string()).debug_assert();
    }
}
