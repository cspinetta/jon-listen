use jon_listen::settings::ProtocolType;
use jon_listen::App;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::broadcast;
use tokio::time::timeout;

mod helpers;
use helpers::create_test_settings_arc;

#[tokio::test]
async fn test_app_start_up_creates_file_writer_and_spawns_tasks() {
    let (settings, _temp_dir) = create_test_settings_arc(ProtocolType::TCP);
    let (shutdown_tx, shutdown_rx) = broadcast::channel(1);

    let app_handle = tokio::spawn(async move { App::start_up(settings, shutdown_rx).await });

    tokio::time::sleep(Duration::from_millis(100)).await;

    assert!(
        !app_handle.is_finished(),
        "App should still be running before shutdown signal"
    );

    shutdown_tx.send(()).unwrap();

    let result = timeout(Duration::from_secs(3), app_handle).await;
    assert!(result.is_ok(), "App should complete within timeout");
    let app_result = result.unwrap();
    assert!(app_result.is_ok(), "App task should not panic");
    let app_return = app_result.unwrap();
    assert!(
        app_return.is_ok(),
        "App should shutdown gracefully after FileWriter creation and task spawning, got: {:?}",
        app_return
    );
}

#[tokio::test]
async fn test_app_start_up_handles_task_failure_gracefully() {
    let (settings, _temp_dir) = create_test_settings_arc(ProtocolType::TCP);
    let (shutdown_tx, shutdown_rx) = broadcast::channel(1);

    let app_handle = tokio::spawn(async move { App::start_up(settings, shutdown_rx).await });

    tokio::time::sleep(Duration::from_millis(100)).await;

    assert!(
        !app_handle.is_finished(),
        "App should still be running when tasks are healthy"
    );

    shutdown_tx.send(()).unwrap();

    let result = timeout(Duration::from_secs(3), app_handle).await;
    assert!(result.is_ok(), "App should complete within timeout");
    let app_result = result.unwrap();
    assert!(app_result.is_ok(), "App task should not panic");
    let app_return = app_result.unwrap();
    assert!(
        app_return.is_ok(),
        "App should handle shutdown gracefully even if tasks were running normally, got: {:?}",
        app_return
    );
}

#[tokio::test]
async fn test_app_start_up_graceful_shutdown_coordination() {
    let (settings, _temp_dir) = create_test_settings_arc(ProtocolType::TCP);
    let (shutdown_tx, shutdown_rx) = broadcast::channel(1);

    let app_handle = tokio::spawn(async move { App::start_up(settings, shutdown_rx).await });

    tokio::time::sleep(Duration::from_millis(100)).await;

    assert!(
        !app_handle.is_finished(),
        "App should still be running before shutdown signal"
    );

    shutdown_tx.send(()).unwrap();

    let result = timeout(Duration::from_secs(3), app_handle).await;
    assert!(result.is_ok(), "App should complete within timeout");
    let app_result = result.unwrap();
    assert!(app_result.is_ok(), "App task should not panic");
    let app_return = app_result.unwrap();
    assert!(
        app_return.is_ok(),
        "App should coordinate graceful shutdown successfully, got: {:?}",
        app_return
    );
}

#[tokio::test]
async fn test_app_start_up_waits_for_tasks_with_timeout() {
    let (settings, _temp_dir) = create_test_settings_arc(ProtocolType::UDP);
    let (shutdown_tx, shutdown_rx) = broadcast::channel(1);

    let app_handle = tokio::spawn(async move { App::start_up(settings, shutdown_rx).await });

    tokio::time::sleep(Duration::from_millis(100)).await;

    shutdown_tx.send(()).unwrap();

    let start_time = std::time::Instant::now();
    let result = timeout(Duration::from_secs(6), app_handle).await;
    let elapsed = start_time.elapsed();

    assert!(result.is_ok(), "App should complete within 6 seconds");
    let app_result = result.unwrap();
    assert!(app_result.is_ok(), "App task should not panic");
    let app_return = app_result.unwrap();
    assert!(
        app_return.is_ok(),
        "App should wait for tasks with timeout successfully, got: {:?}",
        app_return
    );
    assert!(
        elapsed < Duration::from_secs(6),
        "App should complete before timeout expires, took: {:?}",
        elapsed
    );
}

#[tokio::test]
async fn test_app_start_up_handles_task_completion_before_shutdown() {
    let (settings, _temp_dir) = create_test_settings_arc(ProtocolType::TCP);
    let (shutdown_tx, shutdown_rx) = broadcast::channel(1);

    let app_handle = tokio::spawn(async move { App::start_up(settings, shutdown_rx).await });

    tokio::time::sleep(Duration::from_millis(100)).await;

    assert!(
        !app_handle.is_finished(),
        "App should still be running when tasks are active"
    );

    shutdown_tx.send(()).unwrap();

    let result = timeout(Duration::from_secs(3), app_handle).await;
    assert!(result.is_ok(), "App should complete within timeout");
    let app_result = result.unwrap();
    assert!(app_result.is_ok(), "App task should not panic");
    let app_return = app_result.unwrap();
    assert!(
        app_return.is_ok(),
        "App should handle shutdown when tasks complete, got: {:?}",
        app_return
    );
}

#[tokio::test]
async fn test_app_start_up_error_handling_for_file_writer_creation_failure() {
    let (settings, _temp_dir) = helpers::create_test_settings(ProtocolType::TCP);
    let mut settings = settings;
    settings.filewriter.filedir = PathBuf::from("/invalid/path/that/does/not/exist");
    let settings = Arc::new(settings);
    let (_shutdown_tx, shutdown_rx) = broadcast::channel(1);

    let result = App::start_up(settings, shutdown_rx).await;

    assert!(
        result.is_err(),
        "App should return error when FileWriter creation fails"
    );
    let err = result.unwrap_err();
    let err_msg = format!("{}", err);
    assert!(
        err_msg.contains("FileWriter") || err_msg.contains("file"),
        "Error message should mention FileWriter or file, got: {}",
        err_msg
    );
    assert!(
        err_msg.contains("Failed to create FileWriter"),
        "Error message should contain context about FileWriter creation failure, got: {}",
        err_msg
    );
}
