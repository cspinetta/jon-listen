//! Test helpers and utilities for jon-listen tests
//!
//! This module provides shared utilities to reduce duplication across test files
//! and improve test maintainability.

use jon_listen::settings::{
    BackpressurePolicy, FileWriterConfig, FormattingConfig, ProtocolType, RotationPolicyConfig,
    RotationPolicyType, ServerConfig, Settings,
};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tempfile::TempDir;
use tokio::time::sleep;

/// Create a temporary directory for testing
pub fn create_temp_dir() -> TempDir {
    TempDir::new().expect("Failed to create temporary directory")
}

/// Create test settings with a specific protocol
///
/// Returns both the Settings and the TempDir to keep it alive during tests
pub fn create_test_settings(protocol: ProtocolType) -> (Settings, TempDir) {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("Time went backwards");
    let filename = format!("test_{}.log", now.subsec_nanos());
    let temp_dir = create_temp_dir();

    let server = ServerConfig {
        protocol,
        host: "0.0.0.0".to_string(),
        port: 0, // Use port 0 to get any available port
        max_connections: 1000,
    };
    let rotation_policy_config = RotationPolicyConfig {
        count: 10,
        policy: RotationPolicyType::ByDuration,
        duration: Some(3600),
    };
    let formatting_config = FormattingConfig {
        startingmsg: false,
        endingmsg: false,
    };
    let file_config = FileWriterConfig {
        filedir: temp_dir.path().to_path_buf(),
        filename,
        rotation: rotation_policy_config,
        formatting: formatting_config,
        backpressure_policy: BackpressurePolicy::Block,
    };
    let settings = Settings {
        debug: false,
        threads: 1,
        buffer_bound: 20,
        server,
        filewriter: file_config,
        metrics_port: 9090,
    };
    (settings, temp_dir)
}

/// Create test settings wrapped in Arc
pub fn create_test_settings_arc(protocol: ProtocolType) -> (Arc<Settings>, TempDir) {
    let (settings, temp_dir) = create_test_settings(protocol);
    (Arc::new(settings), temp_dir)
}

/// Create a test file with content
pub async fn create_test_file(temp_dir: &TempDir, filename: &str, content: &[u8]) -> PathBuf {
    use tokio::fs;
    let file_path = temp_dir.path().join(filename);
    fs::write(&file_path, content)
        .await
        .expect("Failed to create test file");
    file_path
}

/// Wait for a condition to become true with timeout
///
/// # Example
/// ```no_run
/// use tests::helpers::wait_for_condition;
/// use std::sync::atomic::{AtomicBool, Ordering};
///
/// let flag = Arc::new(AtomicBool::new(false));
/// let flag_clone = flag.clone();
/// tokio::spawn(async move {
///     tokio::time::sleep(Duration::from_millis(100)).await;
///     flag_clone.store(true, Ordering::Relaxed);
/// });
///
/// wait_for_condition(
///     || flag.load(Ordering::Relaxed),
///     Duration::from_secs(1),
/// )
/// .await
/// .expect("Condition should become true");
/// ```
pub async fn wait_for_condition<F>(condition: F, max_wait: Duration) -> Result<(), String>
where
    F: Fn() -> bool,
{
    let start = tokio::time::Instant::now();
    let check_interval = Duration::from_millis(10);

    while start.elapsed() < max_wait {
        if condition() {
            return Ok(());
        }
        sleep(check_interval).await;
    }

    Err(format!(
        "Condition did not become true within {:?}",
        max_wait
    ))
}

/// Assert that a file exists and optionally check its content
pub async fn assert_file_exists(file_path: &std::path::Path) {
    assert!(
        file_path.exists(),
        "File should exist: {}",
        file_path.display()
    );
}

/// Assert that a file exists and contains expected content
pub async fn assert_file_contains(file_path: &std::path::Path, expected_content: &str) {
    use tokio::fs;
    assert_file_exists(file_path).await;
    let content = fs::read_to_string(file_path)
        .await
        .expect("Failed to read file");
    assert!(
        content.contains(expected_content),
        "File should contain '{}', but got: {}",
        expected_content,
        content
    );
}

/// Create a unique test filename based on current time
pub fn unique_test_filename(prefix: &str) -> String {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("Time went backwards");
    format!("{}_{}.log", prefix, now.subsec_nanos())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::Arc;

    #[tokio::test]
    async fn test_wait_for_condition_success() {
        let flag = Arc::new(AtomicBool::new(false));
        let flag_clone = flag.clone();
        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(50)).await;
            flag_clone.store(true, Ordering::Relaxed);
        });

        let result =
            wait_for_condition(|| flag.load(Ordering::Relaxed), Duration::from_secs(1)).await;

        assert!(result.is_ok(), "Condition should become true");
    }

    #[tokio::test]
    async fn test_wait_for_condition_timeout() {
        let flag = Arc::new(AtomicBool::new(false));

        let result =
            wait_for_condition(|| flag.load(Ordering::Relaxed), Duration::from_millis(100)).await;

        assert!(result.is_err(), "Condition should timeout");
        assert!(result.unwrap_err().contains("did not become true"));
    }

    #[tokio::test]
    async fn test_create_test_file() {
        let temp_dir = create_temp_dir();
        let file_path = create_test_file(&temp_dir, "test.txt", b"test content").await;

        assert_file_exists(&file_path).await;
        assert_file_contains(&file_path, "test content").await;
    }

    #[tokio::test]
    async fn test_unique_test_filename() {
        let name1 = unique_test_filename("test");
        let name2 = unique_test_filename("test");

        // Names should be different (unless generated in the same nanosecond)
        assert!(name1.starts_with("test_"));
        assert!(name2.starts_with("test_"));
    }
}
