use std::net::SocketAddr;
use std::sync::Arc;
use std::thread::{self, JoinHandle};

use crate::settings::Settings;
use futures::StreamExt;
use tokio::net::{TcpListener, TcpStream};
use tokio_util::codec::{FramedRead, LinesCodec};

use crate::writer::file_writer::FileWriterCommand;
use std::sync::mpsc::SyncSender;

use log::{debug, info};
use std::io;

pub struct TcpServer;

impl TcpServer {
    pub fn start(
        settings: Arc<Settings>,
        sender: SyncSender<FileWriterCommand>,
    ) -> Vec<JoinHandle<()>> {
        let addr = format!("{}:{}", settings.server.host, settings.server.port)
            .parse::<SocketAddr>()
            .unwrap();
        let addr = Arc::new(addr);

        let mut threads = Vec::new();

        let settings_ref = settings.clone();
        let sender_ref = sender.clone();
        let addr_ref = addr.clone();

        threads.push(thread::spawn(move || {
            info!("Spawning TCP thread");

            let runtime = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("failed to build runtime");

            runtime.block_on(async move {
                let listener = TcpListener::bind(*addr_ref).await.expect("bind tcp");
                loop {
                    tokio::select! {
                        res = listener.accept() => {
                            match res {
                                Ok((stream, _peer)) => {
                                    let svc = TcpListenerService::new(0, sender_ref.clone(), settings_ref.clone());
                                    tokio::spawn(handle_client(stream, svc));
                                }
                                Err(e) => {
                                    eprintln!("accept error: {}", e);
                                }
                            }
                        }
                        _ = tokio::signal::ctrl_c() => {
                            info!("TCP server received shutdown signal");
                            break;
                        }
                    }
                }
            });
        }));

        info!(
            "Listening at {} via TCP with {} threads...",
            addr, settings.threads
        );

        threads
    }
}

async fn handle_client(stream: TcpStream, service: TcpListenerService) {
    let mut reader = FramedRead::new(stream, LinesCodec::new());
    while let Some(line) = reader.next().await {
        match line {
            Ok(l) => {
                let _ = service.handle(l).await;
            }
            Err(e) => {
                eprintln!("read error: {}", e);
                break;
            }
        }
    }
}

#[allow(dead_code)]
struct TcpListenerService {
    pub id: i32,
    pub name: String,
    pub tx_file_writer: SyncSender<FileWriterCommand>,
    settings: Arc<Settings>,
}

impl TcpListenerService {
    pub fn new(
        id: i32,
        tx_file_writer: SyncSender<FileWriterCommand>,
        settings: Arc<Settings>,
    ) -> Self {
        TcpListenerService {
            id,
            name: format!("server-tcp-{}", id),
            tx_file_writer,
            settings,
        }
    }

    pub async fn handle(&self, req: String) -> Result<(), io::Error> {
        debug!("Received a log line in {}", self.name);
        self.tx_file_writer
            .send(FileWriterCommand::Write(req.into_bytes()))
            .map_err(|e| io::Error::other(format!("send error: {}", e)))
    }
}
