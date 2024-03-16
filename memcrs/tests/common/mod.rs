use memcrs::server;
use procspawn::SpawnError;

pub struct MemcrsdTestServer {
    process_handle: procspawn::JoinHandle<()>,
}

impl MemcrsdTestServer {
    fn new(process_handle: procspawn::JoinHandle<()>) -> MemcrsdTestServer {
        MemcrsdTestServer { process_handle }
    }
    fn kill(&mut self) -> Result<(), SpawnError> {
        self.process_handle.kill()
    }
}

impl Drop for MemcrsdTestServer {
    fn drop(&mut self) {
        match self.kill() {
            Ok(_) => {}
            Err(err) => {
                eprintln!("Problem when killing process: {err}");
            }
        }
    }
}

pub fn spawn_server() -> MemcrsdTestServer {
    let args: Vec<String> = Vec::new();
    let handle = procspawn::spawn(args, |args| server::main::run(args));
    MemcrsdTestServer::new(handle)
}
