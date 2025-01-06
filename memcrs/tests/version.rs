procspawn::enable_test_support!();
mod common;

#[test]
fn version_check() {
    let params_builder: common::MemcrsdServerParamsBuilder = common::MemcrsdServerParamsBuilder::new();
    let _server_handle = common::spawn_server(params_builder);
    let client =
        memcache::connect(common::get_connection_string())
            .unwrap();
    // flush the database
    client.flush().unwrap();

    // get a server version
    let version = client.version();
    match version {
        Ok(val) => {
            let server_version = &val[0].1;
            assert_eq!(server_version, "0.0.1");
            println!("Server returned: {:?}", val[0].1)
        },
        Err(err) => {
            println!("Error returned: {:?}", err);
            unreachable!();
        }
    }

}