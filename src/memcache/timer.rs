use std::sync::atomic::{AtomicU64, Ordering};

pub trait Timer {
    fn secs(&self) -> u64;
}

pub trait SetableTimer {
    fn add_second(&self);
}

pub struct SystemTimer {
    seconds: AtomicU64,
}

impl SystemTimer {
    pub fn new() -> Self {
        SystemTimer {
            seconds: AtomicU64::new(0),
        }
    }
}

impl Timer for SystemTimer {
    fn secs(&self) -> u64 {
        self.seconds.load(Ordering::SeqCst) 
    }
}

impl SetableTimer for SystemTimer {
    fn add_second(&self) {
        self.seconds.fetch_add(1, Ordering::SeqCst);
    }
}
