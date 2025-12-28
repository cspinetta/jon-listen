use anyhow::{Context, Result};
use log::info;
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

use jon_listen::{metrics, settings::Settings, App};

#[tokio::main]
async fn main() -> Result<()> {
    pretty_env_logger::init();

    info!("Starting jon-listen app...");

    let settings = Settings::load().context("Failed to load settings")?;
    let metrics_port = settings.metrics_port;
    let settings = Arc::new(settings);

    // Create shutdown broadcast channel
    let (shutdown_tx, _) = tokio::sync::broadcast::channel(1);

    // Initialize metrics and start Prometheus HTTP server
    metrics::init(metrics_port)
        .map_err(|e| anyhow::anyhow!("Failed to initialize metrics: {}", e))?;
    let metrics_shutdown = shutdown_tx.subscribe();
    tokio::spawn(start_metrics_server(metrics_port, metrics_shutdown));

    // Clone shutdown sender for signal handler
    let shutdown_tx_clone = shutdown_tx.clone();

    // Spawn signal handler task and track it
    let signal_handle = tokio::spawn(async move {
        #[cfg(unix)]
        {
            use tokio::signal::unix::{signal, SignalKind};
            let mut sigterm = signal(SignalKind::terminate())
                .map_err(|e| anyhow::anyhow!("Failed to install SIGTERM handler: {}", e))?;
            tokio::select! {
                _ = tokio::signal::ctrl_c() => {
                    info!("Received Ctrl+C, initiating shutdown...");
                }
                _ = sigterm.recv() => {
                    info!("Received SIGTERM, initiating shutdown...");
                }
            }
        }
        #[cfg(not(unix))]
        {
            let _ = tokio::signal::ctrl_c().await;
            info!("Received Ctrl+C, initiating shutdown...");
        }

        // Send shutdown signal to all components
        shutdown_tx_clone
            .send(())
            .map_err(|e| anyhow::anyhow!("Failed to send shutdown signal: {}", e))?;
        Ok::<(), anyhow::Error>(())
    });

    // Log if signal handler task fails
    tokio::spawn(async move {
        match signal_handle.await {
            Ok(Ok(())) => {
                // Signal handler completed successfully
            }
            Ok(Err(err)) => {
                eprintln!("Signal handler task failed: {:#}", err);
            }
            Err(join_err) => {
                eprintln!("Signal handler task panicked: {:?}", join_err);
            }
        }
    });

    // Start the app with shutdown receiver
    App::start_up(settings, shutdown_tx.subscribe())
        .await
        .context("Application startup failed")?;

    info!("Application shutdown complete");
    Ok(())
}

/// Start HTTP server for Prometheus metrics scraping
async fn start_metrics_server(port: u16, mut shutdown_rx: tokio::sync::broadcast::Receiver<()>) {
    let addr = format!("0.0.0.0:{}", port);
    let listener = match TcpListener::bind(&addr).await {
        Ok(l) => l,
        Err(e) => {
            eprintln!("Failed to bind metrics server to {}: {}", addr, e);
            return;
        }
    };

    info!("Metrics server listening on http://{}/metrics", addr);

    loop {
        tokio::select! {
            result = listener.accept() => {
                match result {
                    Ok((mut stream, _)) => {
                        tokio::spawn(async move {
                            let mut buffer = [0; 1024];
                            if let Ok(size) = stream.read(&mut buffer).await {
                                let request = String::from_utf8_lossy(&buffer[..size]);
                                if request.starts_with("GET /metrics") {
                                    let metrics = if let Some(handle) = metrics::get_handle() {
                                        handle.render()
                                    } else {
                                        String::from("# Metrics not available\n")
                                    };
                                    let response = format!(
                                        "HTTP/1.1 200 OK\r\nContent-Type: text/plain; version=0.0.4\r\n\r\n{}\r\n",
                                        metrics
                                    );
                                    let _ = stream.write_all(response.as_bytes()).await;
                                } else {
                                    let response = "HTTP/1.1 404 Not Found\r\n\r\n";
                                    let _ = stream.write_all(response.as_bytes()).await;
                                }
                            }
                        });
                    }
                    Err(e) => {
                        eprintln!("Metrics server accept error: {}", e);
                    }
                }
            }
            _ = shutdown_rx.recv() => {
                info!("Metrics server shutting down");
                break;
            }
        }
    }
}
