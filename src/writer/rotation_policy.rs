
use std::time::Duration;
use chrono::prelude::*;
use time;

pub trait RotationPolicy: Sync + Send {
    fn next_rotation(&self, last_rotation: DateTime<Local>) -> DateTime<Local>;
}

#[derive(Clone)]
pub struct RotationByDuration {
    duration: Duration
}

impl RotationByDuration {

    pub fn new(duration: Duration) -> Self {
        RotationByDuration { duration }
    }
}

impl RotationPolicy for RotationByDuration {

    fn next_rotation(&self, last_rotation: DateTime<Local>) -> DateTime<Local> {
        last_rotation.clone() + time::Duration::from_std(self.duration).unwrap()
    }
}

#[derive(Clone)]
pub struct RotationByDay;

impl RotationByDay {

    pub fn new() -> Self {
        RotationByDay { }
    }
}

impl RotationPolicy for RotationByDay {

    fn next_rotation(&self, last_rotation: DateTime<Local>) -> DateTime<Local> {
        (last_rotation + time::Duration::days(1)).date().and_hms(0, 0, 0)
    }
}
