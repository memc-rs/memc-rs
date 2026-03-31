use std::{env, fs::OpenOptions, io::Write};

use crate::memcache_server::listen_socket_config::ListenSocketConfig;

static MEMCACHED_FILE_ENV_VARIABLE: &str = "MEMCACHED_PORT_FILENAME";

pub struct PortFileWriter {}

impl PortFileWriter {
    pub fn new() -> Self {
        PortFileWriter {}
    }

    pub fn write_port_to_file(&self, config: ListenSocketConfig) -> std::io::Result<()> {
        self.write_port_to_file_with_env_var(config, MEMCACHED_FILE_ENV_VARIABLE)
    }

    fn write_port_to_file_with_env_var(
        &self,
        config: ListenSocketConfig,
        env_var: &str,
    ) -> std::io::Result<()> {
        match env::var(env_var) {
            Ok(file_name) => {
                let file_result = OpenOptions::new()
                    .write(true)
                    .create(true)
                    .open(file_name.clone());

                match file_result {
                    Ok(mut file) => {
                        let file_contents = format!("TCP INET: {}", config.port);
                        let write_result = file.write(file_contents.as_bytes());
                        match write_result {
                            Ok(_res) => {
                                log::info!("Information about port written to: {}", file_name);
                                return Ok(());
                            }
                            Err(err) => {
                                log::error!("Cannot write to file {}; error {}; information about port will not be saved, listen port: {}", file_name.clone(), err, config.port);
                                return Err(err);
                            }
                        }
                    }
                    Err(e) => {
                        log::error!("Cannot open file {}; error {}; information about port will not be saved, listen port: {}", file_name.clone(), e, config.port);
                        return Err(e);
                    }
                }
            }
            Err(err) => {
                log::info!(
                    "Environment variable \"{}\"not present, not writing info about port to a file: {}",
                    env_var, err
                );
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    use std::fs;
    use std::io::Read;

    fn create_test_config(port: i32) -> ListenSocketConfig {
        ListenSocketConfig {
            listen_backlog: 1024,
            listen_address: "127.0.0.1".parse().unwrap(),
            port,
        }
    }

    #[test]
    fn test_write_port_to_file_env_var_not_set() {
        let test_env_var = "TEST_MEMCACHED_PORT_FILENAME_NOT_SET";
        // Ensure the env var is not set
        env::remove_var(test_env_var);

        let writer = PortFileWriter::new();
        let config = create_test_config(11211);

        let result = writer.write_port_to_file_with_env_var(config, test_env_var);
        assert!(result.is_ok());
    }

    #[test]
    fn test_write_port_to_file_success() {
        let test_env_var = "TEST_MEMCACHED_PORT_FILENAME_SUCCESS";
        // Create a temp file path
        let mut temp_path = env::temp_dir();
        temp_path.push("test_port_file_success.txt");

        // Ensure file doesn't exist
        if temp_path.exists() {
            fs::remove_file(&temp_path).unwrap();
        }

        // Set the env var
        env::set_var(test_env_var, temp_path.to_str().unwrap());

        let writer = PortFileWriter::new();
        let config = create_test_config(8080);

        let result = writer.write_port_to_file_with_env_var(config, test_env_var);
        assert!(result.is_ok());

        // Check file contents
        let mut file = fs::File::open(&temp_path).unwrap();
        let mut contents = String::new();
        file.read_to_string(&mut contents).unwrap();
        assert_eq!(contents, "TCP INET: 8080");

        // Clean up
        fs::remove_file(&temp_path).unwrap();
        env::remove_var(test_env_var);
    }

    #[test]
    fn test_write_port_to_file_cannot_open_file() {
        let test_env_var = "TEST_MEMCACHED_PORT_FILENAME_CANNOT_OPEN";
        // Use a path that cannot be opened (e.g., trying to create a file inside a non-directory)
        let invalid_path = "/dev/null/file.txt";
        env::set_var(test_env_var, invalid_path);

        let writer = PortFileWriter::new();
        let config = create_test_config(11211);

        let result = writer.write_port_to_file_with_env_var(config, test_env_var);
        assert!(result.is_err());

        env::remove_var(test_env_var);
    }

    #[test]
    fn test_write_port_to_file_write_fails() {
        let test_env_var = "TEST_MEMCACHED_PORT_FILENAME_WRITE_FAILS";
        let mut temp_path = env::temp_dir();
        temp_path.push("test_dir_as_file_write_fails");

        // Create a directory with that name
        fs::create_dir(&temp_path).unwrap();

        env::set_var(test_env_var, temp_path.to_str().unwrap());

        let writer = PortFileWriter::new();
        let config = create_test_config(11211);

        let result = writer.write_port_to_file_with_env_var(config, test_env_var);
        // Opening a directory for writing should fail
        assert!(result.is_err());

        // Clean up
        fs::remove_dir(&temp_path).unwrap();
        env::remove_var(test_env_var);
    }
}
