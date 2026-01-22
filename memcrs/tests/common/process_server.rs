use crate::common::{random_port::pseudoRanomPort, MemcrsdServerParamsBuilder};
use memcrs::server;
use nix::{
    errno::Errno,
    sys::signal::{kill, SIGINT},
    unistd::Pid,
};

#[allow(dead_code)]
pub struct MemcrsdTestServer {
    process_handle: procspawn::JoinHandle<()>,
    port: u16,
}

#[allow(dead_code)]
impl MemcrsdTestServer {
    fn new(process_handle: procspawn::JoinHandle<()>, port: u16) -> MemcrsdTestServer {
        MemcrsdTestServer {
            process_handle,
            port,
        }
    }

    fn kill(&mut self) -> Result<(), Errno> {
        let pid = self.process_handle.pid();
        match pid {
            Some(raw_pid) => {
                let process_pid = Pid::from_raw(raw_pid as i32);
                kill(process_pid, SIGINT)
            }
            None => {
                let _ = self.process_handle.kill();
                Ok(())
            }
        }
    }

    pub fn get_connection_string(&self) -> String {
        String::from(format!(
            "memcache://127.0.0.1:{}?timeout=5&tcp_nodelay=true&protocol=binary",
            self.port
        ))
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

#[allow(dead_code)]
pub fn spawn_server(mut params: MemcrsdServerParamsBuilder) -> MemcrsdTestServer {
    let port = pseudoRanomPort.lock().unwrap().get_next_port();
    params.with_port(port);
    let args = params.build();
    let handle = procspawn::spawn(args, |args| server::main::run(args));
    MemcrsdTestServer::new(handle, port)
}
