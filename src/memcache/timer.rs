use std::sync::atomic::{AtomicUsize, Ordering};

pub trait Timer {
    fn secs(&self) -> u64;
}

pub trait SetableTimer {
    fn add_second(&self);
}

pub struct SystemTimer {
    seconds: AtomicUsize,
}

impl SystemTimer {
    pub fn new() -> Self {
        SystemTimer {
            seconds: AtomicUsize::new(0),
        }
    }
}

impl Timer for SystemTimer {
    fn secs(&self) -> u64 {
        self.seconds.load(Ordering::Relaxed) as u64
    }
}

impl SetableTimer for SystemTimer {
    fn add_second(&self) {
        self.seconds.fetch_add(1, Ordering::Relaxed);
    }
}
