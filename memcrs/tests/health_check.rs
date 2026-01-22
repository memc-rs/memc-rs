//procspawn::enable_test_support!();
mod common;

#[test]
fn health_check_works() {
    let params_builder: common::MemcrsdServerParamsBuilder =
        common::MemcrsdServerParamsBuilder::new();
    let server_handle = common::spawn_server(params_builder);
    let client = memcache::connect(server_handle.get_connection_string()).unwrap();
    // flush the database
    client.flush().unwrap();

    // retrieve version from memcached server
    let version: Result<Vec<(String, String)>, memcache::MemcacheError> = client.version();

    match version {
        Ok(versions) => {
            assert_eq!(versions.len(), 1);
            assert_eq!(versions[0].1, memcrs::version::MEMCRS_VERSION);
            println!("Memcrs version: {}", versions[0].1);
        }
        Err(_) => {
            unreachable!();
        }
    }
}
