use memcrs::{memcache::cli::parser::RuntimeType, memory_store::StoreEngine};

pub struct MemcrsdServerParamsBuilder {
    engine: StoreEngine,
    runtime: RuntimeType,
    port: u16,
}

impl MemcrsdServerParamsBuilder {
    pub fn new() -> MemcrsdServerParamsBuilder {
        MemcrsdServerParamsBuilder {
            engine: StoreEngine::DashMap,
            runtime: RuntimeType::CurrentThread,
            port: 11211,
        }
    }

    #[allow(dead_code)]
    pub fn with_engine(&mut self, engine: StoreEngine) -> &mut Self {
        self.engine = engine;
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

        result.push(String::from("--port"));
        result.push(self.port.to_string());
        // result.push(String::from("-vvv"));
        result
    }
}
