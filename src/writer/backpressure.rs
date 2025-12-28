use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::mpsc;

use crate::metrics::messages;
use crate::settings::BackpressurePolicy;
use crate::writer::file_writer::FileWriterCommand;
use crate::writer::metrics;

/// Helper for sending messages to FileWriter with backpressure detection and logging
pub struct BackpressureAwareSender {
    sender: mpsc::Sender<FileWriterCommand>,
    backpressure_policy: BackpressurePolicy,
    backpressure_events: Arc<AtomicU64>,
    dropped_messages: Arc<AtomicU64>, // Only used when policy is Discard
    last_log_time: Arc<std::sync::Mutex<Instant>>,
    log_interval: Duration,
}

impl BackpressureAwareSender {
    pub fn new(
        sender: mpsc::Sender<FileWriterCommand>,
        backpressure_policy: BackpressurePolicy,
    ) -> Self {
        Self {
            sender,
            backpressure_policy,
            backpressure_events: Arc::new(AtomicU64::new(0)),
            dropped_messages: Arc::new(AtomicU64::new(0)),
            last_log_time: Arc::new(std::sync::Mutex::new(Instant::now())),
            log_interval: Duration::from_secs(5), // Log at most once every 5 seconds
        }
    }

    /// Send a message with backpressure detection.
    /// If the channel is full, behavior depends on the configured backpressure policy:
    /// - Block: Waits until space is available (provides natural backpressure)
    /// - Discard: Drops the message and logs a warning
    ///
    /// Logs to stderr (not through FileWriter) to avoid feedback loops.
    pub async fn send(
        &self,
        command: FileWriterCommand,
    ) -> Result<(), mpsc::error::SendError<FileWriterCommand>> {
        // Try to send without blocking first
        match self.sender.try_send(command.clone()) {
            Ok(()) => {
                // Note: tokio::sync::mpsc::Sender doesn't expose queue depth directly
                // The capacity() method returns remaining capacity, not current depth
                // To track queue depth accurately, we would need a wrapper that counts sends/receives
                Ok(())
            }
            Err(mpsc::error::TrySendError::Full(_)) => {
                // Channel is full - handle based on policy
                metrics::backpressure::event();
                match self.backpressure_policy {
                    BackpressurePolicy::Block => {
                        // Increment backpressure event counter
                        let events = self.backpressure_events.fetch_add(1, Ordering::Relaxed) + 1;

                        // Rate-limited logging to stderr (not through FileWriter channel)
                        let should_log = {
                            let mut last_log = self.last_log_time.lock().unwrap();
                            let now = Instant::now();
                            if now.duration_since(*last_log) >= self.log_interval {
                                *last_log = now;
                                true
                            } else {
                                false
                            }
                        };

                        if should_log {
                            eprintln!(
                                "WARNING: FileWriter channel is full (capacity: {}). {} backpressure events detected. \
                                 Message ingestion will block until FileWriter processes messages and clears space. \
                                 This indicates backpressure - FileWriter may be slower than message rate.",
                                self.sender.capacity(),
                                events
                            );
                        }

                        // Block until there's space - this provides natural backpressure
                        self.sender.send(command).await
                    }
                    BackpressurePolicy::Discard => {
                        // Increment counters
                        let events = self.backpressure_events.fetch_add(1, Ordering::Relaxed) + 1;
                        let dropped = self.dropped_messages.fetch_add(1, Ordering::Relaxed) + 1;

                        // Rate-limited logging to stderr
                        let should_log = {
                            let mut last_log = self.last_log_time.lock().unwrap();
                            let now = Instant::now();
                            if now.duration_since(*last_log) >= self.log_interval {
                                *last_log = now;
                                true
                            } else {
                                false
                            }
                        };

                        if should_log {
                            eprintln!(
                                "WARNING: FileWriter channel is full (capacity: {}). {} backpressure events detected. \
                                 {} messages dropped so far. Message discarded (backpressure_policy=Discard). \
                                 This indicates backpressure - FileWriter may be slower than message rate.",
                                self.sender.capacity(),
                                events,
                                dropped
                            );
                        }

                        // Return success even though we dropped the message
                        // This allows the caller to continue processing
                        messages::dropped();
                        Ok(())
                    }
                }
            }
            Err(mpsc::error::TrySendError::Closed(msg)) => {
                // Channel closed - log once and return error
                eprintln!("ERROR: FileWriter channel closed. Message dropped.");
                Err(mpsc::error::SendError(msg))
            }
        }
    }

    /// Get the number of backpressure events detected
    pub fn backpressure_events(&self) -> u64 {
        self.backpressure_events.load(Ordering::Relaxed)
    }

    /// Get the number of dropped messages (only meaningful when policy is Discard)
    pub fn dropped_messages(&self) -> u64 {
        self.dropped_messages.load(Ordering::Relaxed)
    }

    /// Reset the backpressure event counter (useful for metrics)
    pub fn reset_backpressure_events(&self) -> u64 {
        self.backpressure_events.swap(0, Ordering::Relaxed)
    }

    /// Reset the dropped messages counter (useful for metrics)
    pub fn reset_dropped_messages(&self) -> u64 {
        self.dropped_messages.swap(0, Ordering::Relaxed)
    }
}

impl Clone for BackpressureAwareSender {
    fn clone(&self) -> Self {
        Self {
            sender: self.sender.clone(),
            backpressure_policy: self.backpressure_policy.clone(),
            backpressure_events: Arc::clone(&self.backpressure_events),
            dropped_messages: Arc::clone(&self.dropped_messages),
            last_log_time: Arc::clone(&self.last_log_time),
            log_interval: self.log_interval,
        }
    }
}
