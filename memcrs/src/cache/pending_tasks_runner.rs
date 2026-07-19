use crate::cache::cache::Cache;
use log::debug;
use std::sync::Arc;
use std::time::{Duration, Instant as StdInstant};
use tokio::time::{interval_at, Instant};
use tokio_util::sync::CancellationToken;
#[cfg(feature = "tikv-alloc")]
use tikv_jemalloc_ctl::{stats, epoch};

pub struct PendingTasksRunner {
    store: Arc<dyn Cache + Send + Sync>,
    cancellation_token: CancellationToken,
}

fn log_memory_usage() {
    cfg_if::cfg_if! {
        if #[cfg(feature = "tikv-alloc")] {
         epoch::advance().unwrap();

            let allocated = stats::allocated::read().unwrap();
            let resident = stats::resident::read().unwrap();
            let allocated_as_bytes = byte_unit::Byte::from_u64(allocated as u64)
            .get_appropriate_unit(byte_unit::UnitType::Decimal);
            let resident_as_bytes = byte_unit::Byte::from_u64(resident as u64)
            .get_appropriate_unit(byte_unit::UnitType::Decimal);
            log::info!("{} bytes allocated/{} bytes resident", allocated_as_bytes, resident_as_bytes);
        }
    }
}

impl PendingTasksRunner {
    const INTERVAL_IN_MILIS: u64 = 100;
    pub fn new(store: Arc<dyn Cache + Send + Sync>, cancellation_token: CancellationToken) -> Self {
        debug!("Creating pending tasks runner");
        PendingTasksRunner {
            store,
            cancellation_token,
        }
    }

    pub async fn run(&self, thread_handles: std::vec::Vec<std::thread::JoinHandle<()>>) {
        let start = Instant::now();
        let mut interval = interval_at(
            start,
            Duration::from_millis(PendingTasksRunner::INTERVAL_IN_MILIS),
        );
        let mut log_interval = interval_at(
            start,
            Duration::from_secs(1),
        );
        loop {
            tokio::select! {
                _ = self.cancellation_token.cancelled() => {
                    info!("Pending tasks runner received cancellation signal, stopping...");
                    break;
                },
                _ = interval.tick() => {
                    let start = StdInstant::now();
                    self.store.run_pending_tasks();
                    let duration = start.elapsed();
                    if duration.as_millis() > (PendingTasksRunner::INTERVAL_IN_MILIS * 2) as u128 {
                        warn!("Server pending tasts finished in: {:?}", duration);
                    } else {
                        trace!("Server pending tasts finished in: {:?}", duration);
                    }
                },
                _ = log_interval.tick() => {
                    log_memory_usage();
                },
            }
        }
        log::info!("Waiting for worker threads to finish");
        for handle in thread_handles {
            if handle.is_finished() {
                continue;
            }
            if let Err(e) = handle.join() {
                log::info!("Thread panicked: {:?}", e);
            }
        }
        log::info!("Worker threads finished");
    }
}
