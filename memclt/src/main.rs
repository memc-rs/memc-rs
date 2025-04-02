
use std::env;
extern crate clap;
mod params_parser;

fn main() {
    memcapability::run(env::args().collect())
}


mod memcapability {
    use log::info;
    use std::process;
    use env_logger::Builder;
    use super::params_parser;

    fn get_log_level(verbose: u8) -> log::LevelFilter {
        // Vary the output based on how many times the user used the "verbose" flag
        // // (i.e. 'myprog -v -v -v' or 'myprog -vvv' vs 'myprog -v'
        match verbose {
            0 => log::LevelFilter::Error,
            1 => log::LevelFilter::Warn,
            2 => log::LevelFilter::Info,
            3 => log::LevelFilter::Debug,
            _ => log::LevelFilter::Trace,
        }
    }

    pub fn run(args: Vec<String>) {

        let cli_config = match params_parser::parse(args) {
            Ok(config) => config,
            Err(err) => {
                eprint!("{}", err);
                process::exit(1);
            }
        };
        let mut builder = Builder::new();
        builder.filter_level(get_log_level(cli_config.verbose));
        builder.init();

        info!("Server address: {}", cli_config.server_address.to_string());
        info!("Server port: {}", cli_config.port);
        info!(
            "Max item size: {}",
            byte_unit::Byte::from_u64(cli_config.item_size)
                .get_appropriate_unit(byte_unit::UnitType::Decimal)
                .to_string()
        );
    }
}
