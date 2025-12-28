use chrono::prelude::*;
use jon_listen::writer::rotation_policy::{RotationByDay, RotationByDuration, RotationPolicy};
use std::time::Duration;

#[test]
fn test_rotation_by_duration_new() {
    let policy = RotationByDuration::new(Duration::from_secs(3600));
    // Verify policy calculates next rotation correctly
    let last = Local
        .with_ymd_and_hms(2024, 1, 1, 12, 0, 0)
        .single()
        .unwrap();
    let next = policy.next_rotation(last);
    let expected = Local
        .with_ymd_and_hms(2024, 1, 1, 13, 0, 0)
        .single()
        .unwrap();
    assert_eq!(
        next, expected,
        "Policy should calculate next rotation 1 hour later"
    );
}

#[test]
fn test_rotation_by_duration_next_rotation() {
    let policy = RotationByDuration::new(Duration::from_secs(3600)); // 1 hour
    let last_rotation = Local
        .with_ymd_and_hms(2024, 1, 1, 12, 0, 0)
        .single()
        .unwrap();

    let next = policy.next_rotation(last_rotation);
    let expected = Local
        .with_ymd_and_hms(2024, 1, 1, 13, 0, 0)
        .single()
        .unwrap();

    assert_eq!(next, expected);
}

#[test]
fn test_rotation_by_duration_various_durations() {
    // Test 30 minutes
    let policy_30min = RotationByDuration::new(Duration::from_secs(1800));
    let last = Local
        .with_ymd_and_hms(2024, 1, 1, 12, 0, 0)
        .single()
        .unwrap();
    let next = policy_30min.next_rotation(last);
    let expected = Local
        .with_ymd_and_hms(2024, 1, 1, 12, 30, 0)
        .single()
        .unwrap();
    assert_eq!(next, expected);

    // Test 24 hours
    let policy_24h = RotationByDuration::new(Duration::from_secs(86400));
    let next = policy_24h.next_rotation(last);
    let expected = Local
        .with_ymd_and_hms(2024, 1, 2, 12, 0, 0)
        .single()
        .unwrap();
    assert_eq!(next, expected);

    // Test 1 second
    let policy_1s = RotationByDuration::new(Duration::from_secs(1));
    let next = policy_1s.next_rotation(last);
    let expected = Local
        .with_ymd_and_hms(2024, 1, 1, 12, 0, 1)
        .single()
        .unwrap();
    assert_eq!(next, expected);
}

#[test]
fn test_rotation_by_duration_day_boundary() {
    let policy = RotationByDuration::new(Duration::from_secs(3600));
    let last = Local
        .with_ymd_and_hms(2024, 1, 1, 23, 30, 0)
        .single()
        .unwrap();

    let next = policy.next_rotation(last);
    let expected = Local
        .with_ymd_and_hms(2024, 1, 2, 0, 30, 0)
        .single()
        .unwrap();

    assert_eq!(next, expected);
}

#[test]
fn test_rotation_by_day_new() {
    let policy = RotationByDay::new();
    // Verify policy calculates next rotation at midnight
    let last = Local
        .with_ymd_and_hms(2024, 1, 1, 12, 0, 0)
        .single()
        .unwrap();
    let next = policy.next_rotation(last);
    let expected = Local
        .with_ymd_and_hms(2024, 1, 2, 0, 0, 0)
        .single()
        .unwrap();
    assert_eq!(
        next, expected,
        "Policy should calculate next rotation at midnight"
    );
}

#[test]
fn test_rotation_by_day_default() {
    let policy = RotationByDay::new();
    // Verify default() produces same behavior as new()
    let last = Local
        .with_ymd_and_hms(2024, 1, 1, 12, 0, 0)
        .single()
        .unwrap();
    let next = policy.next_rotation(last);
    let expected = Local
        .with_ymd_and_hms(2024, 1, 2, 0, 0, 0)
        .single()
        .unwrap();
    assert_eq!(
        next, expected,
        "Default() should produce same behavior as new()"
    );
}

#[test]
fn test_rotation_by_day_next_rotation_midday() {
    let policy = RotationByDay::new();
    // Test from midday - should rotate at next midnight
    let last = Local
        .with_ymd_and_hms(2024, 1, 1, 12, 30, 45)
        .single()
        .unwrap();

    let next = policy.next_rotation(last);
    let expected = Local
        .with_ymd_and_hms(2024, 1, 2, 0, 0, 0)
        .single()
        .unwrap();

    assert_eq!(next, expected);
}

#[test]
fn test_rotation_by_day_next_rotation_just_before_midnight() {
    let policy = RotationByDay::new();
    // Test just before midnight
    let last = Local
        .with_ymd_and_hms(2024, 1, 1, 23, 59, 59)
        .single()
        .unwrap();

    let next = policy.next_rotation(last);
    let expected = Local
        .with_ymd_and_hms(2024, 1, 2, 0, 0, 0)
        .single()
        .unwrap();

    assert_eq!(next, expected);
}

#[test]
fn test_rotation_by_day_next_rotation_at_midnight() {
    let policy = RotationByDay::new();
    // Test at midnight - should rotate at next midnight
    let last = Local
        .with_ymd_and_hms(2024, 1, 1, 0, 0, 0)
        .single()
        .unwrap();

    let next = policy.next_rotation(last);
    let expected = Local
        .with_ymd_and_hms(2024, 1, 2, 0, 0, 0)
        .single()
        .unwrap();

    assert_eq!(next, expected);
}

#[test]
fn test_rotation_by_day_next_rotation_just_after_midnight() {
    let policy = RotationByDay::new();
    // Test just after midnight
    let last = Local
        .with_ymd_and_hms(2024, 1, 2, 0, 0, 1)
        .single()
        .unwrap();

    let next = policy.next_rotation(last);
    let expected = Local
        .with_ymd_and_hms(2024, 1, 3, 0, 0, 0)
        .single()
        .unwrap();

    assert_eq!(next, expected);
}

#[test]
fn test_rotation_by_day_month_boundary() {
    let policy = RotationByDay::new();
    // Test at end of month
    let last = Local
        .with_ymd_and_hms(2024, 1, 31, 12, 0, 0)
        .single()
        .unwrap();

    let next = policy.next_rotation(last);
    let expected = Local
        .with_ymd_and_hms(2024, 2, 1, 0, 0, 0)
        .single()
        .unwrap();

    assert_eq!(next, expected);
}

#[test]
fn test_rotation_by_day_year_boundary() {
    let policy = RotationByDay::new();
    // Test at end of year
    let last = Local
        .with_ymd_and_hms(2023, 12, 31, 12, 0, 0)
        .single()
        .unwrap();

    let next = policy.next_rotation(last);
    let expected = Local
        .with_ymd_and_hms(2024, 1, 1, 0, 0, 0)
        .single()
        .unwrap();

    assert_eq!(next, expected);
}

#[test]
fn test_rotation_by_day_leap_year() {
    let policy = RotationByDay::new();
    // Test leap year February 28 -> March 1
    let last = Local
        .with_ymd_and_hms(2024, 2, 28, 12, 0, 0)
        .single()
        .unwrap();

    let next = policy.next_rotation(last);
    let expected = Local
        .with_ymd_and_hms(2024, 2, 29, 0, 0, 0)
        .single()
        .unwrap();

    assert_eq!(next, expected);

    // Test leap year February 29 -> March 1
    let last = Local
        .with_ymd_and_hms(2024, 2, 29, 12, 0, 0)
        .single()
        .unwrap();
    let next = policy.next_rotation(last);
    let expected = Local
        .with_ymd_and_hms(2024, 3, 1, 0, 0, 0)
        .single()
        .unwrap();
    assert_eq!(next, expected);
}

#[test]
fn test_rotation_by_day_non_leap_year() {
    let policy = RotationByDay::new();
    // Test non-leap year February 28 -> March 1 (skips Feb 29)
    let last = Local
        .with_ymd_and_hms(2023, 2, 28, 12, 0, 0)
        .single()
        .unwrap();

    let next = policy.next_rotation(last);
    let expected = Local
        .with_ymd_and_hms(2023, 3, 1, 0, 0, 0)
        .single()
        .unwrap();

    assert_eq!(next, expected);
}

#[test]
fn test_rotation_policy_trait_rotation_by_duration() {
    let policy: Box<dyn RotationPolicy> =
        Box::new(RotationByDuration::new(Duration::from_secs(3600)));
    let last = Local
        .with_ymd_and_hms(2024, 1, 1, 12, 0, 0)
        .single()
        .unwrap();
    let next = policy.next_rotation(last);
    let expected = Local
        .with_ymd_and_hms(2024, 1, 1, 13, 0, 0)
        .single()
        .unwrap();
    assert_eq!(next, expected);
}

#[test]
fn test_rotation_policy_trait_rotation_by_day() {
    let policy: Box<dyn RotationPolicy> = Box::new(RotationByDay::new());
    let last = Local
        .with_ymd_and_hms(2024, 1, 1, 12, 0, 0)
        .single()
        .unwrap();
    let next = policy.next_rotation(last);
    let expected = Local
        .with_ymd_and_hms(2024, 1, 2, 0, 0, 0)
        .single()
        .unwrap();
    assert_eq!(next, expected);
}
