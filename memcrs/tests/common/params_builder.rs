use memcrs::{
    cache::eviction_policy::EvictionPolicy, memcache::cli::parser::RuntimeType,
    memory_store::StoreEngine,
};

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
