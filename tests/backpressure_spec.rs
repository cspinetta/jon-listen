use jon_listen::settings::BackpressurePolicy;
use jon_listen::writer::backpressure::BackpressureAwareSender;
use jon_listen::writer::file_writer::FileWriterCommand;
use tokio::sync::mpsc;
use tokio::time::{sleep, Duration};

/// Helper to create a bounded channel for testing
fn create_test_channel(
    capacity: usize,
) -> (
    mpsc::Sender<FileWriterCommand>,
    mpsc::Receiver<FileWriterCommand>,
) {
    mpsc::channel(capacity)
}

#[tokio::test]
async fn test_new_with_block_policy() {
    let (tx, _rx) = create_test_channel(10);
    let sender = BackpressureAwareSender::new(tx, BackpressurePolicy::Block);

    assert_eq!(sender.backpressure_events(), 0);
    assert_eq!(sender.dropped_messages(), 0);
}

#[tokio::test]
async fn test_new_with_discard_policy() {
    let (tx, _rx) = create_test_channel(10);
    let sender = BackpressureAwareSender::new(tx, BackpressurePolicy::Discard);

    assert_eq!(sender.backpressure_events(), 0);
    assert_eq!(sender.dropped_messages(), 0);
}

#[tokio::test]
async fn test_send_successful_when_channel_has_capacity_block() {
    let (tx, mut rx) = create_test_channel(10);
    let sender = BackpressureAwareSender::new(tx, BackpressurePolicy::Block);

    let command = FileWriterCommand::Write(b"test message".to_vec());
    let result = sender.send(command.clone()).await;

    assert!(result.is_ok());
    assert_eq!(sender.backpressure_events(), 0);

    // Verify message was received
    let received = rx.recv().await;
    assert!(received.is_some());
    assert_eq!(received.unwrap(), command);
}

#[tokio::test]
async fn test_send_successful_when_channel_has_capacity_discard() {
    let (tx, mut rx) = create_test_channel(10);
    let sender = BackpressureAwareSender::new(tx, BackpressurePolicy::Discard);

    let command = FileWriterCommand::Write(b"test message".to_vec());
    let result = sender.send(command.clone()).await;

    assert!(result.is_ok());
    assert_eq!(sender.backpressure_events(), 0);
    assert_eq!(sender.dropped_messages(), 0);

    // Verify message was received
    let received = rx.recv().await;
    assert!(received.is_some());
    assert_eq!(received.unwrap(), command);
}

#[tokio::test]
async fn test_send_blocks_when_channel_full_block_policy() {
    let (tx, rx) = create_test_channel(1); // Small capacity
    let sender = BackpressureAwareSender::new(tx, BackpressurePolicy::Block);

    // Fill the channel
    let command1 = FileWriterCommand::Write(b"first".to_vec());
    sender.send(command1.clone()).await.unwrap();

    // This should block until space is available
    let command2 = FileWriterCommand::Write(b"second".to_vec());
    let start = std::time::Instant::now();

    // Spawn a task to consume the message after a delay, which will unblock the send
    let sender_clone = sender.clone();
    let mut rx_clone = rx;
    let send_handle = tokio::spawn(async move { sender_clone.send(command2).await });

    // Wait a bit to ensure send is blocking
    sleep(Duration::from_millis(10)).await;

    // Consume the first message to make space - this should unblock the send
    let _ = rx_clone.recv().await;

    // Now the send should complete
    let result = send_handle.await.unwrap();
    assert!(result.is_ok());
    assert!(start.elapsed() >= Duration::from_millis(10));

    // Verify the second message was received
    let received = rx_clone.recv().await;
    assert!(received.is_some());
    assert_eq!(
        received.unwrap(),
        FileWriterCommand::Write(b"second".to_vec())
    );
}

#[tokio::test]
async fn test_send_discards_when_channel_full_discard_policy() {
    let (tx, mut rx) = create_test_channel(1); // Small capacity
    let sender = BackpressureAwareSender::new(tx, BackpressurePolicy::Discard);

    // Fill the channel
    let command1 = FileWriterCommand::Write(b"first".to_vec());
    sender.send(command1.clone()).await.unwrap();

    // Try to send another - should be discarded
    let command2 = FileWriterCommand::Write(b"second".to_vec());
    let result = sender.send(command2).await;

    assert!(result.is_ok()); // Returns Ok even though message was dropped
    assert_eq!(sender.backpressure_events(), 1);
    assert_eq!(sender.dropped_messages(), 1);

    // Verify only first message was received
    let received = rx.recv().await;
    assert!(received.is_some());
    assert_eq!(received.unwrap(), command1);

    // Second message should not be in channel
    let timeout_result = tokio::time::timeout(Duration::from_millis(10), rx.recv()).await;
    assert!(timeout_result.is_err()); // Should timeout - no message
}

#[tokio::test]
async fn test_send_error_when_channel_closed() {
    let (tx, rx) = create_test_channel(10);
    let sender = BackpressureAwareSender::new(tx, BackpressurePolicy::Block);

    // Close the receiver
    drop(rx);

    // Small delay to ensure channel closure propagates
    sleep(Duration::from_millis(10)).await;

    let command = FileWriterCommand::Write(b"test".to_vec());
    let result = sender.send(command).await;

    assert!(result.is_err());
}

#[tokio::test]
async fn test_backpressure_events_counter() {
    let (tx, mut rx) = create_test_channel(1);
    let sender = BackpressureAwareSender::new(tx, BackpressurePolicy::Discard);

    // Fill channel
    sender
        .send(FileWriterCommand::Write(b"first".to_vec()))
        .await
        .unwrap();

    // Trigger backpressure multiple times
    for _ in 0..5 {
        sender
            .send(FileWriterCommand::Write(b"overflow".to_vec()))
            .await
            .unwrap();
    }

    assert_eq!(sender.backpressure_events(), 5);

    // Consume the first message
    let _ = rx.recv().await;
}

#[tokio::test]
async fn test_dropped_messages_counter() {
    let (tx, mut rx) = create_test_channel(1);
    let sender = BackpressureAwareSender::new(tx, BackpressurePolicy::Discard);

    // Fill channel
    sender
        .send(FileWriterCommand::Write(b"first".to_vec()))
        .await
        .unwrap();

    // Drop 3 messages
    for _ in 0..3 {
        sender
            .send(FileWriterCommand::Write(b"overflow".to_vec()))
            .await
            .unwrap();
    }

    assert_eq!(sender.dropped_messages(), 3);

    // With Block policy, dropped_messages should remain 0
    let (tx2, _rx2) = create_test_channel(1);
    let sender_block = BackpressureAwareSender::new(tx2, BackpressurePolicy::Block);
    sender_block
        .send(FileWriterCommand::Write(b"first".to_vec()))
        .await
        .unwrap();

    // This will block, not drop, so counter should stay 0
    assert_eq!(sender_block.dropped_messages(), 0);

    // Consume the first message
    let _ = rx.recv().await;
}

#[tokio::test]
async fn test_reset_backpressure_events() {
    let (tx, mut rx) = create_test_channel(1);
    let sender = BackpressureAwareSender::new(tx, BackpressurePolicy::Discard);

    // Fill channel and trigger backpressure
    sender
        .send(FileWriterCommand::Write(b"first".to_vec()))
        .await
        .unwrap();
    sender
        .send(FileWriterCommand::Write(b"overflow".to_vec()))
        .await
        .unwrap();

    assert_eq!(sender.backpressure_events(), 1);

    let old_value = sender.reset_backpressure_events();
    assert_eq!(old_value, 1);
    assert_eq!(sender.backpressure_events(), 0);

    // Consume the first message
    let _ = rx.recv().await;
}

#[tokio::test]
async fn test_reset_dropped_messages() {
    let (tx, mut rx) = create_test_channel(1);
    let sender = BackpressureAwareSender::new(tx, BackpressurePolicy::Discard);

    // Fill channel and drop messages
    sender
        .send(FileWriterCommand::Write(b"first".to_vec()))
        .await
        .unwrap();
    sender
        .send(FileWriterCommand::Write(b"overflow1".to_vec()))
        .await
        .unwrap();
    sender
        .send(FileWriterCommand::Write(b"overflow2".to_vec()))
        .await
        .unwrap();

    assert_eq!(sender.dropped_messages(), 2);

    let old_value = sender.reset_dropped_messages();
    assert_eq!(old_value, 2);
    assert_eq!(sender.dropped_messages(), 0);

    // Consume the first message
    let _ = rx.recv().await;
}

#[tokio::test]
async fn test_clone_shares_counters() {
    let (tx, mut rx) = create_test_channel(1);
    let sender1 = BackpressureAwareSender::new(tx, BackpressurePolicy::Discard);
    let sender2 = sender1.clone();

    // Fill channel with sender1
    sender1
        .send(FileWriterCommand::Write(b"first".to_vec()))
        .await
        .unwrap();

    // Drop message with sender2
    sender2
        .send(FileWriterCommand::Write(b"overflow".to_vec()))
        .await
        .unwrap();

    // Both should see the same counters
    assert_eq!(sender1.backpressure_events(), 1);
    assert_eq!(sender2.backpressure_events(), 1);
    assert_eq!(sender1.dropped_messages(), 1);
    assert_eq!(sender2.dropped_messages(), 1);

    // Consume the first message
    let _ = rx.recv().await;
}

#[tokio::test]
async fn test_rate_limited_logging() {
    // This test verifies that logging happens at most once per interval
    // We can't easily test the actual logging, but we can verify the behavior
    // by checking that backpressure events are tracked correctly

    let (tx, mut rx) = create_test_channel(1);
    let sender = BackpressureAwareSender::new(tx, BackpressurePolicy::Discard);

    // Fill channel
    sender
        .send(FileWriterCommand::Write(b"first".to_vec()))
        .await
        .unwrap();

    // Trigger multiple backpressure events quickly
    for _ in 0..10 {
        sender
            .send(FileWriterCommand::Write(b"overflow".to_vec()))
            .await
            .unwrap();
    }

    // All events should be tracked
    assert_eq!(sender.backpressure_events(), 10);
    assert_eq!(sender.dropped_messages(), 10);

    // Consume the first message
    let _ = rx.recv().await;
}

#[tokio::test]
async fn test_different_command_types() {
    let (tx, mut rx) = create_test_channel(10);
    let sender = BackpressureAwareSender::new(tx, BackpressurePolicy::Block);

    // Test Write command
    let write_cmd = FileWriterCommand::Write(b"test".to_vec());
    sender.send(write_cmd.clone()).await.unwrap();
    assert_eq!(rx.recv().await.unwrap(), write_cmd);

    // Test WriteDebug command
    let debug_cmd = FileWriterCommand::WriteDebug("test".to_string(), b"debug".to_vec(), 1);
    sender.send(debug_cmd.clone()).await.unwrap();
    assert_eq!(rx.recv().await.unwrap(), debug_cmd);

    // Test Rename command
    use std::path::PathBuf;
    let rename_cmd = FileWriterCommand::Rename(PathBuf::from("/tmp/test.log"));
    sender.send(rename_cmd.clone()).await.unwrap();
    assert_eq!(rx.recv().await.unwrap(), rename_cmd);
}
