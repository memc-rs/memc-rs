use crate::cache::cache::Cache;
use log::debug;
use std::sync::Arc;
use std::time::{Duration, Instant as StdInstant};
use tokio::time::{interval_at, Instant};

pub struct PendingTasksRunner {
    store: Arc<dyn Cache + Send + Sync>,
}

impl PendingTasksRunner {
    const INTERVAL_IN_MILIS: u64 = 100;
    pub fn new(store: Arc<dyn Cache + Send + Sync>) -> Self {
        debug!("Creating pending tasks runner");
        PendingTasksRunner { store }
    }

    pub async fn run(&self) {
        let start = Instant::now();
        let mut interval = interval_at(
            start,
            Duration::from_millis(PendingTasksRunner::INTERVAL_IN_MILIS),
        );
        loop {
            interval.tick().await;
            let start = StdInstant::now();
            self.store.run_pending_tasks();
            let duration = start.elapsed();
            if duration.as_millis() > (PendingTasksRunner::INTERVAL_IN_MILIS * 2) as u128 {
                warn!("Server pending tasts finished in: {:?}", duration);
            } else {
                debug!("Server pending tasts finished in: {:?}", duration);
            }
        }
    }
}
