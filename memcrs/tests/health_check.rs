use memcache::MemcacheError;


#[tokio::test]
async fn health_check_works() {
  spawn_app().await.expect("Failed to start app");
  let client = memcache::connect("memcache://127.0.0.1:11211?timeout=10&tcp_nodelay=true&protocol=binary").unwrap();
  // flush the database
  client.flush().unwrap();

  // retrieve version from memcached server
  let version: Result<Vec<(String, String)>, memcache::MemcacheError> = client.version();

  match version {
      Ok(versions) => {
        assert_eq!(versions.len(), 1);
      },
      Err(_) => {
        unreachable!();
      }
  }

}

async fn spawn_app() -> std::io::Result<()> {
  todo!()
}