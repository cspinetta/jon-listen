use std::net::SocketAddr;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use crate::settings::Settings;
use futures::StreamExt;
use tokio::net::{TcpListener, TcpStream};
use tokio_util::codec::{FramedRead, LinesCodec};

use crate::listener::metrics;
use crate::writer::backpressure::BackpressureAwareSender;
use crate::writer::file_writer::FileWriterCommand;
use tokio::sync::broadcast;

use log::{debug, info, warn};
use std::io;

pub struct TcpServer;

impl TcpServer {
    pub async fn start(
        settings: Arc<Settings>,
        sender: BackpressureAwareSender,
        shutdown_rx: broadcast::Receiver<()>,
    ) -> Result<(), io::Error> {
        let addr = format!("{}:{}", settings.server.host, settings.server.port)
            .parse::<SocketAddr>()
            .unwrap();

        info!("Listening at {} via TCP...", addr);
        info!("Maximum connections: {}", settings.server.max_connections);

        let listener = TcpListener::bind(addr).await?;
        let mut shutdown_rx = shutdown_rx;
        let connection_count = Arc::new(AtomicUsize::new(0));

        loop {
            tokio::select! {
                res = listener.accept() => {
                    match res {
                        Ok((stream, peer)) => {
                            let current_connections = connection_count.load(Ordering::Relaxed);
                            if current_connections >= settings.server.max_connections {
                                warn!(
                                    "Max connections ({}) reached, rejecting connection from {}",
                                    settings.server.max_connections,
                                    peer
                                );
                                metrics::tcp::connection_rejected();
                                // Close the connection immediately
                                drop(stream);
                                continue;
                            }

                            // Increment connection count
                            connection_count.fetch_add(1, Ordering::Relaxed);
                            metrics::tcp::connection_accepted();
                            metrics::tcp::connection_active(connection_count.load(Ordering::Relaxed));

                            // Clone here - they're cheap (Arc and Sender are just pointers)
                            let svc = TcpListenerService::new(0, sender.clone(), settings.clone());
                            // Create a shutdown receiver for this client
                            let client_shutdown = shutdown_rx.resubscribe();
                            let connection_count_clone = connection_count.clone();

                            tokio::spawn(async move {
                                handle_client(stream, svc, client_shutdown).await;
                                // Decrement connection count when client disconnects
                                let new_count = connection_count_clone.fetch_sub(1, Ordering::Relaxed) - 1;
                                metrics::tcp::connection_active(new_count);
                            });
                        }
                        Err(e) => {
                            eprintln!("accept error: {}", e);
                        }
                    }
                }
                _ = shutdown_rx.recv() => {
                    info!("TCP server received shutdown signal");
                    break;
                }
            }
        }

        info!("TCP server shutting down gracefully");
        Ok(())
    }
}

async fn handle_client(
    stream: TcpStream,
    service: TcpListenerService,
    mut shutdown_rx: broadcast::Receiver<()>,
) {
    let mut reader = FramedRead::new(stream, LinesCodec::new());
    loop {
        tokio::select! {
            line = reader.next() => {
                match line {
                    Some(Ok(l)) => {
                        let _ = service.handle(l).await;
                    }
                    Some(Err(e)) => {
                        eprintln!("read error: {}", e);
                        break;
                    }
                    None => {
                        // EOF - client disconnected
                        break;
                    }
                }
            }
            _ = shutdown_rx.recv() => {
                info!("Client handler received shutdown signal, closing connection");
                break;
            }
        }
    }
}

#[allow(dead_code)]
struct TcpListenerService {
    pub id: i32,
    pub name: String,
    pub tx_file_writer: BackpressureAwareSender,
    settings: Arc<Settings>,
}

impl TcpListenerService {
    pub fn new(id: i32, tx_file_writer: BackpressureAwareSender, settings: Arc<Settings>) -> Self {
        TcpListenerService {
            id,
            name: format!("server-tcp-{}", id),
            tx_file_writer,
            settings,
        }
    }

    pub async fn handle(&self, req: String) -> Result<(), io::Error> {
        debug!("Received a log line in {}", self.name);
        crate::metrics::messages::received();
        self.tx_file_writer
            .send(FileWriterCommand::Write(req.into_bytes()))
            .await
            .map_err(|e| io::Error::other(format!("send error: {}", e)))
    }
}
