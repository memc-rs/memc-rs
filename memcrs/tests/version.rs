procspawn::enable_test_support!();
mod common;

#[test]
fn version_check() {
    let params_builder: common::MemcrsdServerParamsBuilder = common::MemcrsdServerParamsBuilder::new();
    let server_handle = common::spawn_server(params_builder);
    let client =
        memcache::connect(server_handle.get_connection_string())
            .unwrap();
    // flush the database
    client.flush().unwrap();

    // get a server version
    let version = client.version();
    match version {
        Ok(val) => {
            let server_version = &val[0].1;
            assert_eq!(server_version, memcrs::version::MEMCRS_VERSION);
            println!("Server returned: {:?}", server_version);
        },
        Err(err) => {
            println!("Error returned: {:?}", err);
            unreachable!();
        }
    }

}