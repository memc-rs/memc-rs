use std::sync::Arc;
use tokio_util::sync::CancellationToken;

use crate::{
    cache::{cache::Cache, pending_tasks_runner},
    memcache,
    server::timer,
};

pub struct ServerContext {
    cancellation_token: CancellationToken,
    system_timer: Arc<timer::SystemTimer>,
    store: Arc<dyn Cache + Send + Sync>,
    pending_tasks_runner: Arc<pending_tasks_runner::PendingTasksRunner>,
}

impl ServerContext {
    pub fn get_default_server_context(
        store_config: memcache::builder::MemcacheStoreConfig,
    ) -> Self {
        let cancellation_token = CancellationToken::new();
        let system_timer = Arc::new(timer::SystemTimer::new(cancellation_token.clone()));
        let store = memcache::builder::MemcacheStoreBuilder::from_config(
            store_config,
            system_timer.clone(),
        );
        let pending_tasks_runner = Arc::new(pending_tasks_runner::PendingTasksRunner::new(
            store.clone(),
            cancellation_token.clone(),
        ));
        Self {
            cancellation_token,
            system_timer,
            store,
            pending_tasks_runner,
        }
    }

    pub fn cancellation_token(&self) -> CancellationToken {
        self.cancellation_token.clone()
    }

    pub fn system_timer(&self) -> Arc<timer::SystemTimer> {
        self.system_timer.clone()
    }

    pub fn store(&self) -> Arc<dyn Cache + Send + Sync> {
        self.store.clone()
    }

    pub fn pending_tasks_runner(&self) -> Arc<pending_tasks_runner::PendingTasksRunner> {
        self.pending_tasks_runner.clone()
    }
}
