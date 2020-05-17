pub trait Timer {
    fn secs(&self) -> u64;
}

pub struct SystemTimer;

impl SystemTimer {
    pub fn new() -> Self {
        SystemTimer {}
    }
}

impl Timer for SystemTimer {
    fn secs(&self) -> u64 {
        0
    }
}
