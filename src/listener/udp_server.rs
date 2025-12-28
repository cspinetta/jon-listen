use tokio::net::UdpSocket;

use std::io;
use std::net::SocketAddr;

use crate::listener::metrics;
use crate::settings::Settings;
use crate::writer::backpressure::BackpressureAwareSender;
use crate::writer::file_writer::FileWriterCommand;

use log::{debug, error, info};
use std::sync::Arc;
use tokio::sync::broadcast;

pub struct UdpServer;

impl UdpServer {
    pub async fn start(
        settings: Arc<Settings>,
        sender: BackpressureAwareSender,
        shutdown_rx: broadcast::Receiver<()>,
    ) -> Result<(), io::Error> {
        let addr = format!("{}:{}", settings.server.host, settings.server.port)
            .parse::<SocketAddr>()
            .unwrap();

        info!("Listening at {} via UDP...", addr);

        let socket = UdpSocket::bind(addr).await?;
        let mut service = UdpService::new(socket, sender, 0, settings);

        // Pass shutdown receiver to run() so it can check for shutdown signals
        match service.run(shutdown_rx).await {
            Ok(()) => {
                info!("UDP server shutting down gracefully");
            }
            Err(e) => {
                error!("UDP service error: {}", e);
            }
        }

        Ok(())
    }
}

pub struct UdpService {
    pub id: i32,
    pub name: String,
    pub socket: UdpSocket,
    pub buf: Vec<u8>,
    pub writer_sender: BackpressureAwareSender,
    settings: Arc<Settings>,
    count: i32,
}

impl UdpService {
    pub fn new(
        s: UdpSocket,
        writer_sender: BackpressureAwareSender,
        id: i32,
        settings: Arc<Settings>,
    ) -> Self {
        UdpService {
            id,
            name: format!("server-udp-{}", id),
            socket: s,
            buf: vec![0u8; 15000],
            writer_sender,
            settings,
            count: 0, // For debug only
        }
    }

    pub async fn run(&mut self, mut shutdown_rx: broadcast::Receiver<()>) -> Result<(), io::Error> {
        loop {
            tokio::select! {
                res = self.socket.recv_from(&mut self.buf) => {
                    let (size, _peer) = res?;
                    metrics::udp::datagram_received();
                    crate::metrics::messages::received();
                    if self.settings.debug {
                        self.count += 1;
                        info!(
                            "Poll datagram from server {}. Count: {}",
                            self.name, self.count
                        );
                        let _ = self
                            .writer_sender
                            .send(FileWriterCommand::WriteDebug(
                                self.name.clone(),
                                self.buf[..size].to_vec(),
                                self.count,
                            ))
                            .await;
                    } else {
                        debug!("Poll datagram from server {}.", self.name);
                        let _ = self
                            .writer_sender
                            .send(FileWriterCommand::Write(self.buf[..size].to_vec()))
                            .await;
                    }
                }
                _ = shutdown_rx.recv() => {
                    info!("UdpService received shutdown signal");
                    break;
                }
            }
        }
        Ok(())
    }
}
