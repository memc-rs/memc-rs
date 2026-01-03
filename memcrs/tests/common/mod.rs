use lazy_static::lazy_static;
use memcrs::{
    cache::eviction_policy::EvictionPolicy, memcache::cli::parser::RuntimeType,
    memory_store::StoreEngine, server,
};
use nix::{
    errno::Errno,
    sys::signal::{kill, SIGINT},
    unistd::Pid,
};
use rand::Rng;
use std::sync::Mutex;

pub struct MemcrsdTestServer {
    process_handle: procspawn::JoinHandle<()>,
    port: u16,
}

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

        //
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

pub struct MemcrsdServerParamsBuilder {
    engine: StoreEngine,
    policy: EvictionPolicy,
    runtime: RuntimeType,
    memory_limit: u64,
    port: u16,
}

impl MemcrsdServerParamsBuilder {
    pub fn new() -> MemcrsdServerParamsBuilder {
        MemcrsdServerParamsBuilder {
            engine: StoreEngine::DashMap,
            policy: EvictionPolicy::None,
            runtime: RuntimeType::CurrentThread,
            memory_limit: 1024 * 1024 * 64,
            port: 11211,
        }
    }

    #[allow(dead_code)]
    pub fn with_engine(&mut self, engine: StoreEngine) -> &mut Self {
        self.engine = engine;
        self
    }

    #[allow(dead_code)]
    pub fn with_policy(&mut self, policy: EvictionPolicy) -> &mut Self {
        self.policy = policy;
        self
    }

    #[allow(dead_code)]
    pub fn with_memory_limit(&mut self, memory_limit: u64) -> &mut Self {
        self.memory_limit = memory_limit;
        self
    }

    pub fn with_port(&mut self, port: u16) -> &mut Self {
        self.port = port;
        self
    }

    pub fn build(&self) -> Vec<String> {
        let mut result: Vec<String> = Vec::new();
        result.push(String::from("./target/debug/memcrsd"));
        match self.engine {
            StoreEngine::DashMap => {
                result.push(String::from("--store-engine"));
                result.push(String::from("dash-map"));
            }
            StoreEngine::Moka => {
                result.push(String::from("--store-engine"));
                result.push(String::from("moka"));
            }
        }

        match self.runtime {
            RuntimeType::CurrentThread => {
                result.push(String::from("--runtime-type"));
                result.push(String::from("current-thread"));
            }
            RuntimeType::MultiThread => {
                result.push(String::from("--runtime-type"));
                result.push(String::from("multi-thread"));
            }
        }

        result.push(String::from("--memory-limit"));
        result.push(self.memory_limit.to_string());

        result.push(String::from("--port"));
        result.push(self.port.to_string());
        // result.push(String::from("-vvv"));
        result
    }
}

const STARTING_PORT: u16 = 10000;
struct PseudoRandomMemcrsdPort {
    port: u16,
}

impl PseudoRandomMemcrsdPort {
    fn new() -> PseudoRandomMemcrsdPort {
        PseudoRandomMemcrsdPort {
            port: STARTING_PORT,
        }
    }

    fn get_next_port(&mut self) -> u16 {
        self.port += 10;
        self.port
    }
}

lazy_static! {
    static ref pseudoRanomPort: Mutex<PseudoRandomMemcrsdPort> =
        Mutex::new(PseudoRandomMemcrsdPort::new());
}

pub fn spawn_server(mut params: MemcrsdServerParamsBuilder) -> MemcrsdTestServer {
    let port = pseudoRanomPort.lock().unwrap().get_next_port();
    params.with_port(port);
    let args = params.build();
    let handle = procspawn::spawn(args, |args| server::main::run(args));
    MemcrsdTestServer::new(handle, port)
}

#[allow(dead_code)]
pub fn create_value_with_size(size: usize) -> String {
    let mut rng = rand::rng();
    let mut value = String::with_capacity(size);
    for _ in 0..size {
        let random_char = rng.random_range(b'a'..=b'z') as char;
        value.push(random_char);
    }
    value
}
