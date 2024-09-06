use std::time::Duration;

use futures::task::Spawn;
use memcrs::{cache::eviction_policy::EvictionPolicy, memcache::cli::parser::RuntimeType, memory_store::StoreEngine, server};
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
}

impl MemcrsdServerParamsBuilder {
    pub fn new() -> MemcrsdServerParamsBuilder {
        MemcrsdServerParamsBuilder{
            engine: StoreEngine::DashMap,
            policy: EvictionPolicy::None,
            runtime: RuntimeType::CurrentThread,
            memory_limit: 1024*1024*64
        }
    }

    pub fn with_engine(mut self, engine: StoreEngine) -> MemcrsdServerParamsBuilder {
        self.engine = engine;
        self
    }

    pub fn with_policy(mut self, policy: EvictionPolicy) -> MemcrsdServerParamsBuilder {
        self.policy = policy;
        self
    }

    pub fn with_memory_limit(mut self, memory_limit: u64) -> MemcrsdServerParamsBuilder {
        self.memory_limit = memory_limit;
        self
    }

    pub fn build(self) -> Vec<String> {
        let mut result: Vec<String> = Vec::new();
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
        result
    }
}

pub fn spawn_server() -> MemcrsdTestServer {
    let args: Vec<String> = Vec::new();
    let handle = procspawn::spawn(args, |args| server::main::run(args));
    MemcrsdTestServer::new(handle)
}

pub fn get_connection_string() -> String {
    String::from("memcache://127.0.0.1:11211?timeout=10&tcp_nodelay=true&protocol=binary")
}