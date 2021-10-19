use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;
use tokio::time::{interval_at, Instant};
pub trait Timer {
    fn timestamp(&self) -> u64;
}

pub trait SetableTimer {
    fn add_second(&self);
}

pub struct SystemTimer {
    seconds: AtomicU64,
}

impl SystemTimer {
    pub fn new() -> Self {
        debug!("Creating system timer");
        SystemTimer {
            seconds: AtomicU64::new(0),
        }
    }

    pub async fn run(&self) -> () {
        let start = Instant::now();
        let mut interval = interval_at(start, Duration::from_secs(1));
        loop {
            interval.tick().await;
            self.add_second();
            debug!("Server tick: {}", self.timestamp());
        }
    }
}

impl Timer for SystemTimer {
    fn timestamp(&self) -> u64 {
        self.seconds.load(Ordering::SeqCst)
    }
}

impl SetableTimer for SystemTimer {
    fn add_second(&self) {
        self.seconds.fetch_add(1, Ordering::SeqCst);
    }
}
