use chrono::prelude::*;
use std::time::Duration;

pub trait RotationPolicy: Sync + Send {
    fn next_rotation(&self, last_rotation: DateTime<Local>) -> DateTime<Local>;
}

#[derive(Clone)]
pub struct RotationByDuration {
    duration: Duration,
}

impl RotationByDuration {
    pub fn new(duration: Duration) -> Self {
        RotationByDuration { duration }
    }
}

impl RotationPolicy for RotationByDuration {
    fn next_rotation(&self, last_rotation: DateTime<Local>) -> DateTime<Local> {
        last_rotation + chrono::Duration::from_std(self.duration).unwrap()
    }
}

#[derive(Clone)]
pub struct RotationByDay;

impl RotationByDay {
    pub fn new() -> Self {
        RotationByDay {}
    }
}

impl Default for RotationByDay {
    fn default() -> Self {
        Self::new()
    }
}

impl RotationPolicy for RotationByDay {
    fn next_rotation(&self, last_rotation: DateTime<Local>) -> DateTime<Local> {
        let next_day = (last_rotation + chrono::Duration::days(1)).date_naive();
        let midnight = next_day.and_hms_opt(0, 0, 0).unwrap();
        Local.from_local_datetime(&midnight).single().unwrap()
    }
}
