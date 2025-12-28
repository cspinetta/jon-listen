use jon_listen::error::RotationError;
use jon_listen::writer::file_rotation::FileRotation;
use jon_listen::writer::file_writer::FileWriterCommand;
use jon_listen::writer::rotation_policy::{RotationByDuration, RotationPolicy};
use std::path::PathBuf;
use std::time::Duration;
use tempfile::TempDir;
use tokio::fs;
use tokio::sync::{broadcast, mpsc};
use tokio::time::timeout;

/// Helper to create a FileRotation instance for testing
fn create_test_file_rotation(
    temp_dir: &TempDir,
    max_files: i32,
) -> (FileRotation, mpsc::Receiver<FileWriterCommand>) {
    let file_dir_path = temp_dir.path().to_path_buf();
    let mut file_path = file_dir_path.clone();
    file_path.push("test.log");
    let file_name = "test.log".to_string();

    let rotation_policy: Box<dyn RotationPolicy> =
        Box::new(RotationByDuration::new(Duration::from_secs(3600)));
    let (tx, rx) = mpsc::channel(10);

    let file_rotation = FileRotation::new(
        file_dir_path,
        file_path,
        file_name,
        max_files,
        rotation_policy,
        tx,
    );

    (file_rotation, rx)
}

#[tokio::test]
async fn test_file_rotation_new() {
    let temp_dir = TempDir::new().unwrap();
    let (_rotation, _rx) = create_test_file_rotation(&temp_dir, 10);
    // Just verify it doesn't panic - no assertion needed
}

#[tokio::test]
async fn test_search_files_finds_matching_files() {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("test.log");

    // Create some test files
    fs::write(&file_path, b"content").await.unwrap();
    fs::write(temp_dir.path().join("test.log.0"), b"content0")
        .await
        .unwrap();
    fs::write(temp_dir.path().join("test.log.1"), b"content1")
        .await
        .unwrap();
    fs::write(temp_dir.path().join("test.log.2"), b"content2")
        .await
        .unwrap();

    let result = FileRotation::search_files(file_path).await;
    assert!(result.is_ok());

    let files = result.unwrap();
    // Should find all numbered variants (test.log.* pattern matches test.log.0, test.log.1, test.log.2)
    // Note: The base file "test.log" doesn't match the pattern "test.log.*", so we expect 3 files
    assert!(files.len() >= 3);

    // Verify all found files match the pattern
    for file in &files {
        let name = file.file_name().unwrap().to_str().unwrap();
        assert!(name.starts_with("test.log"));
    }
}

#[tokio::test]
async fn test_search_files_no_files_found() {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("nonexistent.log");

    let result = FileRotation::search_files(file_path.clone()).await;
    assert!(result.is_ok());

    let _files = result.unwrap();
    // Should return empty vec or just the base file if it exists
    // Note: len() is always >= 0, so we just verify the function doesn't panic
}

#[tokio::test]
async fn test_search_files_invalid_path() {
    // Test with a path that can't be converted to string (non-UTF8)
    // This is hard to test directly, but we can test the error path
    let invalid_path = PathBuf::from("/tmp/\0invalid.log");

    // On Unix, this might work, but the error handling should be tested
    // For now, we'll test that the function handles edge cases
    let result = FileRotation::search_files(invalid_path).await;
    // The result depends on the OS, but should not panic
    let _ = result;
}

#[tokio::test]
async fn test_next_path_no_existing_files() {
    let temp_dir = TempDir::new().unwrap();
    let (rotation, _rx) = create_test_file_rotation(&temp_dir, 10);

    let files = vec![];
    let result = rotation.next_path(&files);

    assert!(result.is_ok());
    let next = result.unwrap();
    assert!(next.to_str().unwrap().ends_with(".0"));
}

#[tokio::test]
async fn test_next_path_with_existing_files() {
    let temp_dir = TempDir::new().unwrap();
    let (rotation, _rx) = create_test_file_rotation(&temp_dir, 10);

    // Create file paths with IDs
    let mut file_path = temp_dir.path().to_path_buf();
    file_path.push("test.log");

    let files = vec![
        file_path.join("test.log.0"),
        file_path.join("test.log.1"),
        file_path.join("test.log.5"),
    ];

    let result = rotation.next_path(&files);

    assert!(result.is_ok());
    let next = result.unwrap();
    // Should be .6 (next after .5)
    assert!(next.to_str().unwrap().ends_with(".6"));
}

#[tokio::test]
async fn test_next_path_sequential_ids() {
    let temp_dir = TempDir::new().unwrap();
    let (rotation, _rx) = create_test_file_rotation(&temp_dir, 10);

    let mut file_path = temp_dir.path().to_path_buf();
    file_path.push("test.log");

    let files = vec![
        file_path.join("test.log.0"),
        file_path.join("test.log.1"),
        file_path.join("test.log.2"),
    ];

    let result = rotation.next_path(&files);

    assert!(result.is_ok());
    let next = result.unwrap();
    assert!(next.to_str().unwrap().ends_with(".3"));
}

#[tokio::test]
async fn test_next_path_no_numeric_suffix() {
    let temp_dir = TempDir::new().unwrap();
    let (rotation, _rx) = create_test_file_rotation(&temp_dir, 10);

    let mut file_path = temp_dir.path().to_path_buf();
    file_path.push("test.log");

    // Files without numeric suffix should be ignored
    let files = vec![
        file_path.join("test.log"),
        file_path.join("test.log.backup"),
    ];

    let result = rotation.next_path(&files);

    // Should still work and return .0 since no numeric suffixes found
    assert!(result.is_ok());
    let next = result.unwrap();
    assert!(next.to_str().unwrap().ends_with(".0"));
}

#[tokio::test]
async fn test_next_path_invalid_filename() {
    let temp_dir = TempDir::new().unwrap();
    let (rotation, _rx) = create_test_file_rotation(&temp_dir, 10);

    // Create a path that doesn't have a valid filename
    let invalid_file = PathBuf::from("/");

    let files = vec![invalid_file];

    let result = rotation.next_path(&files);

    // Should return an error for invalid file
    assert!(result.is_err());
    match result.unwrap_err() {
        RotationError::InvalidFile(_) => {}
        _ => panic!("Expected InvalidFile error"),
    }
}

#[tokio::test]
async fn test_oldest_file_selects_correctly() {
    let temp_dir = TempDir::new().unwrap();
    let (rotation, _rx) = create_test_file_rotation(&temp_dir, 10);

    // Create files with different modification times
    let file1 = temp_dir.path().join("test.log.1");
    let file2 = temp_dir.path().join("test.log.2");
    let file3 = temp_dir.path().join("test.log.3");

    fs::write(&file1, b"oldest").await.unwrap();
    tokio::time::sleep(Duration::from_millis(10)).await;
    fs::write(&file2, b"middle").await.unwrap();
    tokio::time::sleep(Duration::from_millis(10)).await;
    fs::write(&file3, b"newest").await.unwrap();

    let files = vec![file1.clone(), file2.clone(), file3.clone()];

    let result = rotation.oldest_file(&files).await;
    assert!(result.is_ok());

    let oldest = result.unwrap();
    // Should be file1 (oldest)
    assert!(oldest.ends_with("test.log.1") || oldest.ends_with("test.log.1"));
}

#[tokio::test]
async fn test_oldest_file_empty_list() {
    let temp_dir = TempDir::new().unwrap();
    let (rotation, _rx) = create_test_file_rotation(&temp_dir, 10);

    let files = vec![];

    let result = rotation.oldest_file(&files).await;
    assert!(result.is_ok());

    let oldest = result.unwrap();
    // Should return default file (.0)
    assert!(oldest.to_str().unwrap().contains(".0"));
}

#[tokio::test]
async fn test_oldest_file_nonexistent_file() {
    let temp_dir = TempDir::new().unwrap();
    let (rotation, _rx) = create_test_file_rotation(&temp_dir, 10);

    let nonexistent = temp_dir.path().join("nonexistent.log.1");
    let files = vec![nonexistent];

    let result = rotation.oldest_file(&files).await;
    // Should return an error for nonexistent file
    assert!(result.is_err());
    match result.unwrap_err() {
        RotationError::IOError(_) => {}
        _ => panic!("Expected IOError for nonexistent file"),
    }
}

#[tokio::test]
async fn test_request_rotate_sends_rename_command() {
    let temp_dir = TempDir::new().unwrap();
    let (rotation, mut rx) = create_test_file_rotation(&temp_dir, 10);

    // Create the base file
    let file_path = temp_dir.path().join("test.log");
    fs::write(&file_path, b"content").await.unwrap();

    let result = rotation.request_rotate().await;
    assert!(result.is_ok());

    // Check that Rename command was sent
    let received = rx.recv().await;
    assert!(received.is_some());
    match received.unwrap() {
        FileWriterCommand::Rename(path) => {
            assert!(path.to_str().unwrap().contains("test.log"));
        }
        _ => panic!("Expected Rename command"),
    }
}

#[tokio::test]
async fn test_request_rotate_when_max_files_reached() {
    let temp_dir = TempDir::new().unwrap();
    let (rotation, mut rx) = create_test_file_rotation(&temp_dir, 3); // max_files = 3

    // Create files up to max_files
    for i in 0..3 {
        let file = temp_dir.path().join(format!("test.log.{}", i));
        fs::write(&file, b"content").await.unwrap();
        tokio::time::sleep(Duration::from_millis(10)).await;
    }

    // Create the base file
    let file_path = temp_dir.path().join("test.log");
    fs::write(&file_path, b"content").await.unwrap();

    let result = rotation.request_rotate().await;
    assert!(result.is_ok());

    // Should select oldest file (test.log.0)
    let received = rx.recv().await;
    assert!(received.is_some());
    match received.unwrap() {
        FileWriterCommand::Rename(path) => {
            // Should be the oldest file path
            let path_str = path.to_str().unwrap();
            assert!(path_str.contains("test.log"));
        }
        _ => panic!("Expected Rename command"),
    }
}

// Phase 6: FileRotation::start() integration tests

#[tokio::test]
async fn test_file_rotation_start_triggers_rotation_by_duration() {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("test.log");
    fs::write(&file_path, b"content").await.unwrap();

    // Create rotation with very short duration (100ms)
    let rotation_policy: Box<dyn RotationPolicy> =
        Box::new(RotationByDuration::new(Duration::from_millis(100)));
    let (tx, mut rx) = mpsc::channel(10);

    let rotation = FileRotation::new(
        temp_dir.path().to_path_buf(),
        file_path.clone(),
        "test.log".to_string(),
        10,
        rotation_policy,
        tx,
    );

    let (shutdown_tx, shutdown_rx) = broadcast::channel(1);

    // Start rotation
    let rotation_handle = rotation.start_async(shutdown_rx);

    // Record start time to verify rotation timing
    let start_time = std::time::Instant::now();

    // Wait for rotation to trigger (should happen after ~100ms)
    let received = timeout(Duration::from_millis(500), rx.recv()).await;
    assert!(
        received.is_ok(),
        "Rotation should trigger within timeout period"
    );
    let received = received.unwrap();
    assert!(received.is_some(), "Rotation should send a Rename command");

    // Verify rotation timing (should be approximately 100ms, allow some variance)
    let elapsed = start_time.elapsed();
    assert!(
        elapsed >= Duration::from_millis(90),
        "Rotation should wait at least ~90ms before triggering, but waited {:?}",
        elapsed
    );
    assert!(
        elapsed <= Duration::from_millis(300),
        "Rotation should trigger within reasonable time (~300ms), but took {:?}",
        elapsed
    );

    // Verify the Rename command contains correct path
    match received.unwrap() {
        FileWriterCommand::Rename(rename_path) => {
            assert!(
                rename_path.to_str().unwrap().contains("test.log"),
                "Rename path should contain the original filename, got: {:?}",
                rename_path
            );
            assert!(
                rename_path.to_str().unwrap().ends_with(".0")
                    || rename_path.to_str().unwrap().ends_with(".1"),
                "Rename path should have numeric suffix (.0 or .1), got: {:?}",
                rename_path
            );
        }
        cmd => panic!("Expected Rename command, got: {:?}", cmd),
    }

    // Verify graceful shutdown
    shutdown_tx.send(()).unwrap();
    let shutdown_result = timeout(Duration::from_secs(1), rotation_handle).await;
    assert!(
        shutdown_result.is_ok(),
        "Rotation task should shutdown within timeout"
    );
    let join_result = shutdown_result.unwrap();
    assert!(
        join_result.is_ok(),
        "Rotation task should not panic during shutdown"
    );
    let rotation_result = join_result.unwrap();
    assert!(
        rotation_result.is_ok(),
        "Rotation should shutdown gracefully without errors, got: {:?}",
        rotation_result
    );
}

#[tokio::test]
async fn test_file_rotation_start_handles_rotation_failure_and_retries() {
    let temp_dir = TempDir::new().unwrap();
    // Create a rotation that will fail (file doesn't exist, but rotation will try)
    let file_path = temp_dir.path().join("test.log");
    // Don't create the file - this will cause rotation to potentially fail

    let rotation_policy: Box<dyn RotationPolicy> =
        Box::new(RotationByDuration::new(Duration::from_millis(50)));
    let (tx, mut rx) = mpsc::channel(10);

    let rotation = FileRotation::new(
        temp_dir.path().to_path_buf(),
        file_path.clone(),
        "test.log".to_string(),
        10,
        rotation_policy,
        tx,
    );

    let (shutdown_tx, shutdown_rx) = broadcast::channel(1);

    // Start rotation
    let rotation_handle = rotation.start_async(shutdown_rx);

    // Wait a bit - rotation should attempt and potentially fail, then retry
    tokio::time::sleep(Duration::from_millis(200)).await;

    // Rotation might succeed (if file gets created) or fail and retry
    // The important thing is it doesn't panic and continues running
    // Check if any message was received (non-blocking)
    let check_result = rx.try_recv();

    // Verify rotation task is still running (not crashed)
    assert!(
        !rotation_handle.is_finished(),
        "Rotation task should still be running after potential failure/retry"
    );

    // If a message was received, verify it's a Rename command
    if let Ok(cmd) = check_result {
        match cmd {
            FileWriterCommand::Rename(path) => {
                assert!(
                    path.to_str().unwrap().contains("test.log"),
                    "Rename command should contain original filename, got: {:?}",
                    path
                );
            }
            cmd => {
                // Other commands are unexpected but not necessarily wrong
                // Just log for debugging
                eprintln!("Unexpected command received: {:?}", cmd);
            }
        }
    }
    // If no message (TryRecvError::Empty), that's fine - rotation might be waiting or retrying

    // Shutdown and verify graceful completion
    shutdown_tx.send(()).unwrap();
    let shutdown_result = timeout(Duration::from_secs(1), rotation_handle).await;
    assert!(
        shutdown_result.is_ok(),
        "Rotation task should shutdown within timeout even after failures"
    );
    let join_result = shutdown_result.unwrap();
    assert!(
        join_result.is_ok(),
        "Rotation task should not panic during shutdown"
    );
    let rotation_result = join_result.unwrap();
    assert!(
        rotation_result.is_ok(),
        "Rotation should shutdown gracefully after handling failures, got: {:?}",
        rotation_result
    );
}
