use log::debug;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;
use tokio::time::{interval_at, Instant};

pub trait Timer {
    fn timestamp(&self) -> u32;
}

pub trait SetableTimer {
    fn add_second(&self);
}

#[derive(Default)]
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

    pub async fn run(&self) {
        let start = Instant::now();
        let mut interval = interval_at(start, Duration::from_secs(1));
        loop {
            interval.tick().await;
            self.add_second();
            trace!("Server tick: {}", self.timestamp());
        }
    }
}

impl Timer for SystemTimer {
    fn timestamp(&self) -> u32 {
        self.seconds.load(Ordering::Acquire) as u32
    }
}

impl SetableTimer for SystemTimer {
    fn add_second(&self) {
        self.seconds.fetch_add(1, Ordering::Release);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::sync::Mutex;

    #[tokio::test]
    async fn test_initial_timestamp() {
        let timer = SystemTimer::new();
        assert_eq!(timer.timestamp(), 0);
    }

    #[tokio::test]
    async fn test_add_second() {
        let timer = SystemTimer::new();
        timer.add_second();
        assert_eq!(timer.timestamp(), 1);
        timer.add_second();
        assert_eq!(timer.timestamp(), 2);
    }

    #[tokio::test]
    async fn test_run_increments_time() {
        let timer = Arc::new(SystemTimer::new());
        let timer_clone = Arc::clone(&timer);

        let handle = tokio::spawn(async move {
            tokio::time::sleep(Duration::from_secs(2)).await; // Let it run for 2 ticks
        });

        tokio::spawn(async move {
            timer_clone.run().await;
        });

        handle.await.unwrap();
        assert!(timer.timestamp() >= 2);
    }

    #[derive(Default)]
    struct MockTimer {
        time: Mutex<u32>,
    }

    impl Timer for MockTimer {
        fn timestamp(&self) -> u32 {
            *self.time.blocking_lock()
        }
    }

    impl SetableTimer for MockTimer {
        fn add_second(&self) {
            let mut time = self.time.blocking_lock();
            *time += 1;
        }
    }

    #[test]
    fn test_mock_timer() {
        let mock_timer = MockTimer::default();
        assert_eq!(mock_timer.timestamp(), 0);
        mock_timer.add_second();
        assert_eq!(mock_timer.timestamp(), 1);
    }
}
