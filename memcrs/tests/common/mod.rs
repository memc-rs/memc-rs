use std::{sync::Mutex, time::Duration};
use memcrs::{cache::eviction_policy::EvictionPolicy, memcache::cli::parser::RuntimeType, memory_store::StoreEngine, server};
use procspawn::SpawnError;
use lazy_static::lazy_static;

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

    fn join(&mut self) -> Result<(), SpawnError> {
        self.process_handle.join_timeout(Duration::from_secs(1))
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
        match self.join() {
            Ok(_) => {}
            Err(err) => {
                eprintln!("Problem when joining process: {err}");
            }
        }
    }
}

pub struct MemcrsdServerParamsBuilder {
    engine: StoreEngine,
    policy: EvictionPolicy,
    runtime: RuntimeType,
    memory_limit: u64,
    port: u16
}

impl MemcrsdServerParamsBuilder {
    pub fn new() -> MemcrsdServerParamsBuilder {
        MemcrsdServerParamsBuilder{
            engine: StoreEngine::DashMap,
            policy: EvictionPolicy::None,
            runtime: RuntimeType::CurrentThread,
            memory_limit: 1024*1024*64,
            port: 11211,
        }
    }

    pub fn with_engine(&mut self, engine: StoreEngine) -> &mut Self {
        self.engine = engine;
        self
    }

    pub fn with_policy(&mut self, policy: EvictionPolicy) -> &mut Self {
        self.policy = policy;
        self
    }

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
            },
            StoreEngine::Moka => {
                result.push(String::from("--store-engine"));
                result.push(String::from("moka"));
            }
        }

        match self.runtime {
            RuntimeType::CurrentThread => {
                result.push(String::from("--runtime-type"));
                result.push(String::from("current-thread"));
            },
            RuntimeType::MultiThread => {
                result.push(String::from("--runtime-type"));
                result.push(String::from("multi-thread"));
            }
        }

        result.push(String::from("--memory-limit"));
        result.push(self.memory_limit.to_string());

        result.push(String::from("--port"));
        result.push(self.port.to_string());

        result
    }
}

const STARTING_PORT: u16 = 11211;
struct PseudoRandomMemcrsdPort {
    port: u16,
}

impl PseudoRandomMemcrsdPort {
    fn new() -> PseudoRandomMemcrsdPort {
        PseudoRandomMemcrsdPort {
            port: STARTING_PORT,
        }
    }
    
    fn get_next_port(&mut self) {
        self.port += 10;
    }

    fn get(&mut self) -> u16 {
        self.port
    }
}

lazy_static! {
    static ref pseudoRanomPort: Mutex<PseudoRandomMemcrsdPort> = Mutex::new(PseudoRandomMemcrsdPort::new());
}

pub fn spawn_server(mut params: MemcrsdServerParamsBuilder) -> MemcrsdTestServer {
    pseudoRanomPort.lock().unwrap().get_next_port();
    params.with_port(pseudoRanomPort.lock().unwrap().get());
    let args = params.build();
    let handle = procspawn::spawn(args, |args| server::main::run(args));
    MemcrsdTestServer::new(handle)
}

pub fn get_connection_string() -> String {
    String::from(format!("memcache://127.0.0.1:{}?timeout=1&tcp_nodelay=true&protocol=binary", pseudoRanomPort.lock().unwrap().get()))
}