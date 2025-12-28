pub mod error;
pub mod listener;
pub mod metrics;
pub mod settings;
pub mod writer;

use std::sync::Arc;

use anyhow::{Context, Result};
use listener::Listener;
use log::info;
use settings::Settings;
use tokio::sync::broadcast;
use writer::file_writer::FileWriter;

// use std::borrow::Borrow; // not needed

pub struct App;

impl App {
    pub async fn start_up(
        settings: Arc<Settings>,
        shutdown_rx: broadcast::Receiver<()>,
    ) -> Result<()> {
        let mut file_writer = FileWriter::new(settings.buffer_bound, settings.filewriter.clone())
            .await
            .context("Failed to create FileWriter")?;

        // Clone the sender before moving file_writer and wrap with backpressure-aware sender
        use crate::writer::backpressure::BackpressureAwareSender;
        let file_writer_tx = BackpressureAwareSender::new(
            file_writer.tx.clone(),
            settings.filewriter.backpressure_policy.clone(),
        );
        let settings_clone = settings.clone();

        // Clone shutdown receiver for each component
        let listener_shutdown = shutdown_rx.resubscribe();
        let file_writer_shutdown = shutdown_rx.resubscribe();
        let rotation_shutdown = shutdown_rx.resubscribe();
        let mut shutdown_rx = shutdown_rx;

        // Spawn listener as a concurrent task
        let mut listener_handle = tokio::spawn(async move {
            Listener::start(settings_clone, file_writer_tx, listener_shutdown)
                .await
                .context("Listener failed")?;
            Ok::<(), anyhow::Error>(())
        });

        // Spawn file writer as a concurrent task
        let mut file_writer_handle = tokio::spawn(async move {
            file_writer
                .start(file_writer_shutdown, rotation_shutdown)
                .await
                .context("FileWriter failed")?;
            Ok::<(), anyhow::Error>(())
        });

        // Wait for shutdown signal or component failure
        let shutdown_received = tokio::select! {
            result = &mut listener_handle => {
                match result {
                    Ok(_) => {
                        info!("Listener task completed unexpectedly");
                        false // Component completed, not a graceful shutdown
                    }
                    Err(e) => {
                        eprintln!("Listener task failed: {:#}", e);
                        false // Component failed, not a graceful shutdown
                    }
                }
            }
            result = &mut file_writer_handle => {
                match result {
                    Ok(_) => {
                        info!("FileWriter task completed unexpectedly");
                        false // Component completed, not a graceful shutdown
                    }
                    Err(e) => {
                        eprintln!("FileWriter task failed: {:#}", e);
                        false // Component failed, not a graceful shutdown
                    }
                }
            }
            _ = shutdown_rx.recv() => {
                info!("Shutdown signal received in App");
                true // Graceful shutdown initiated
            }
        };

        // If shutdown was received, wait for both tasks to complete gracefully
        if shutdown_received {
            info!("Waiting for components to shut down gracefully...");

            // Give components a moment to process shutdown signal
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

            // Wait for tasks to complete, with timeout
            let shutdown_timeout = tokio::time::Duration::from_secs(5);
            let start = tokio::time::Instant::now();

            // Wait for listener to complete (if not already done)
            if !listener_handle.is_finished() {
                tokio::select! {
                    result = &mut listener_handle => {
                        match result {
                            Ok(_) => info!("Listener task completed gracefully"),
                            Err(e) => eprintln!("Listener task join error: {:#}", e),
                        }
                    }
                    _ = tokio::time::sleep(shutdown_timeout) => {
                        eprintln!("Warning: Listener shutdown timeout reached");
                    }
                }
            } else {
                // Task already completed, just get the result
                match listener_handle.await {
                    Ok(_) => info!("Listener task already completed"),
                    Err(e) => eprintln!("Listener task join error: {:#}", e),
                }
            }

            // Wait for file writer to complete (if not already done)
            let remaining_timeout = shutdown_timeout.saturating_sub(start.elapsed());
            if !file_writer_handle.is_finished() {
                tokio::select! {
                    result = &mut file_writer_handle => {
                        match result {
                            Ok(_) => info!("FileWriter task completed gracefully"),
                            Err(e) => eprintln!("FileWriter task join error: {:#}", e),
                        }
                    }
                    _ = tokio::time::sleep(remaining_timeout) => {
                        eprintln!("Warning: FileWriter shutdown timeout reached");
                    }
                }
            } else {
                // Task already completed, just get the result
                match file_writer_handle.await {
                    Ok(_) => info!("FileWriter task already completed"),
                    Err(e) => eprintln!("FileWriter task join error: {:#}", e),
                }
            }
        }

        Ok(())
    }
}
