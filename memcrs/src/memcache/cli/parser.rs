use crate::version;
use byte_unit::{Byte, ByteUnit};
use clap::{command, crate_authors, value_parser, Arg};
use std::net::IpAddr;
use tracing;

pub enum RuntimeType {
    CurrentThread,
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

pub struct MemcrsArgs {
    pub port: u16,
    pub connection_limit: u32,
    pub backlog_limit: u32,
    pub memory_limit_mb: u64,
    pub item_size_limit: Byte,
    pub memory_limit: u64,
    pub threads: usize,
    pub log_level: tracing::Level,
    pub listen_address: IpAddr,
    pub runtime_type: RuntimeType,
}

const DEFAULT_PORT: u16 = 11211;
const DEFAULT_ADDRESS: &str = "127.0.0.1";
const CONNECTION_LIMIT: u32 = 1024;
const LISTEN_BACKLOG: u32 = 1024;
const MEMORY_LIMIT: u64 = 64;
const MAX_ITEM_SIZE: &str = "1m";
const NUMBER_OF_THREADS: usize = 4;
const RUNTIME_TYPE: &str = "current";

impl MemcrsArgs {
    fn from_args(threads: String, args: Vec<String>) -> Result<MemcrsArgs, String> {
        let number_of_threads: usize = threads.parse().unwrap_or(NUMBER_OF_THREADS);
        let matches = cli_args(&threads).get_matches_from(args);

        let port: u16 = *matches.get_one::<u16>("port").unwrap_or(&DEFAULT_PORT);

        let connection_limit: u32 = *matches
            .get_one::<u32>("connection-limit")
            .unwrap_or(&CONNECTION_LIMIT);

        let backlog_limit: u32 = *matches
            .get_one::<u32>("listen-backlog")
            .unwrap_or(&LISTEN_BACKLOG);

        let memory_limit_mb: u64 = *matches
            .get_one::<u64>("memory-limit")
            .unwrap_or(&MEMORY_LIMIT);

        let memory_limit_res = Byte::from_unit(memory_limit_mb as f64, ByteUnit::MiB).unwrap();
        let memory_limit: u64 = memory_limit_res.get_bytes() as u64;

        let item_size_limit_str: String = matches
            .get_one::<String>("max-item-size")
            .unwrap_or(&String::from(MAX_ITEM_SIZE))
            .clone();

        let item_size_limit_res = Byte::from_str(item_size_limit_str).unwrap();
        let item_size_limit_max = Byte::from_unit(1000f64, ByteUnit::MiB).unwrap();

        if item_size_limit_res.get_bytes() > item_size_limit_max.get_bytes() {
            return Err(format!(
                "Max item size cannot be greater than: {}",
                item_size_limit_max.get_appropriate_unit(false)
            ));
        }

        let threads: usize = *matches
            .get_one::<usize>("threads")
            .unwrap_or(&number_of_threads);

        let listen_address = match matches
            .get_one::<String>("listen")
            .unwrap_or(&String::from(DEFAULT_ADDRESS))
            .parse::<IpAddr>()
        {
            Ok(ip_addr) => ip_addr,
            Err(err) => return Err(format!("Invalid ip address: {}", err)),
        };

        let runtime_type = match matches
            .get_one::<String>("runtime-type")
            .unwrap_or(&String::from(RUNTIME_TYPE))
            .as_str()
        {
            "current" => RuntimeType::CurrentThread,
            "multi" => RuntimeType::MultiThread,
            _ => unreachable!(),
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
            connection_limit,
            backlog_limit,
            memory_limit_mb,
            item_size_limit: item_size_limit_res,
            memory_limit,
            threads,
            log_level,
            listen_address,
            runtime_type,
        })
    }
}

fn cli_args<'help>(threads: &'help str) -> clap::Command<'help> {
    command!()
        .version(version::MEMCRS_VERSION)
        .author(crate_authors!("\n"))
        .about("memcrsd - memcached compatible server implementation in Rust")
        .arg(
            Arg::new("port")
                .short('p')
                .long("port")
                .default_value("11211")
                .value_parser(value_parser!(u16))
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
            Arg::new("connection-limit")
                .short('c')
                .long("connection-limit")
                .value_parser(value_parser!(u32))
                .default_value("1024")
                .help("max simultaneous connections")
                .takes_value(true),
        )
        .arg(
            Arg::new("listen-backlog")
                .short('b')
                .long("listen-backlog")
                .value_parser(value_parser!(u32))
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
                .value_parser(value_parser!(u64))
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
            Arg::new("threads")
                .short('t')
                .long("threads")
                .value_parser(value_parser!(usize))
                .default_value(threads)
                .help("number of threads to use")
                .takes_value(true),
        )
        .arg(
            Arg::new("runtime-type")
                .short('r')
                .long("runtime-type")
                .default_value("current")
                .value_parser(["current", "multi"])
                .help("runtime type to use")
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
