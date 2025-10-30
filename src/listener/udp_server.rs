use tokio::net::UdpSocket;

use std::io;
use std::net::SocketAddr;

use crate::settings::Settings;
use crate::writer::file_writer::FileWriterCommand;

use log::{debug, info};
use std::sync::mpsc::SyncSender;
use std::sync::Arc;
use std::thread::{self, JoinHandle};

pub struct UdpServer;

impl UdpServer {
    pub fn start(
        settings: Arc<Settings>,
        sender: SyncSender<FileWriterCommand>,
    ) -> Vec<JoinHandle<()>> {
        let addr = format!("{}:{}", settings.server.host, settings.server.port)
            .parse::<SocketAddr>()
            .unwrap();
        let addr = Arc::new(addr);

        let mut threads: Vec<JoinHandle<()>> = Vec::new();

        let settings_ref = settings.clone();
        let tx_file_writer = sender.clone();
        let addr_ref = addr.clone();
        threads.push(thread::spawn(move || {
            info!("Spawning UDP thread");

            let runtime = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("failed to build runtime");

            runtime.block_on(async move {
                let socket = UdpSocket::bind(*addr_ref).await.expect("bind udp");
                let mut service = UdpService::new(socket, tx_file_writer, 0, settings_ref);
                loop {
                    tokio::select! {
                        res = service.run() => {
                            let _ = res; // if run returns, continue to check shutdown
                        }
                        _ = tokio::signal::ctrl_c() => {
                            info!("UDP server received shutdown signal");
                            break;
                        }
                    }
                }
            });
        }));

        info!(
            "Listening at {} via UDP with {} threads...",
            addr, settings.threads
        );

        threads
    }
}

pub struct UdpService {
    pub id: i32,
    pub name: String,
    pub socket: UdpSocket,
    pub buf: Vec<u8>,
    pub writer_sender: SyncSender<FileWriterCommand>,
    settings: Arc<Settings>,
    count: i32,
}

impl UdpService {
    pub fn new(
        s: UdpSocket,
        writer_sender: SyncSender<FileWriterCommand>,
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

    pub async fn run(&mut self) -> Result<(), io::Error> {
        loop {
            let (size, _peer) = self.socket.recv_from(&mut self.buf).await?;
            if self.settings.debug {
                self.count += 1;
                info!(
                    "Poll datagram from server {}. Count: {}",
                    self.name, self.count
                );
                let _ = self.writer_sender.send(FileWriterCommand::WriteDebug(
                    self.name.clone(),
                    self.buf[..size].to_vec(),
                    self.count,
                ));
            } else {
                debug!("Poll datagram from server {}.", self.name);
                let _ = self
                    .writer_sender
                    .send(FileWriterCommand::Write(self.buf[..size].to_vec()));
            }
        }
    }
}
