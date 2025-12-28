use metrics::{counter, histogram};
use std::time::Instant;

/// Track backpressure metrics
pub mod backpressure {
    use super::*;

    pub fn event() {
        counter!("backpressure_events_total", 1);
    }
}

/// Track file write metrics
pub mod file_write {
    use super::*;

    pub fn record_latency(duration: std::time::Duration) {
        histogram!("file_write_latency_seconds", duration.as_secs_f64());
    }

    /// Helper to measure and record write latency
    pub struct WriteTimer {
        start: Instant,
    }

    impl WriteTimer {
        pub fn start() -> Self {
            Self {
                start: Instant::now(),
            }
        }

        pub fn finish(self) {
            let duration = self.start.elapsed();
            record_latency(duration);
        }
    }
}

/// Track file rotation metrics
pub mod rotation {
    use super::*;

    pub fn event() {
        counter!("file_rotation_events_total", 1);
    }

    pub fn error() {
        counter!("file_rotation_errors_total", 1);
    }
}
