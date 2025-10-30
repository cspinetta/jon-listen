use crate::listener::tcp_server::TcpServer;
use crate::listener::udp_server::UdpServer;
use crate::settings::{ProtocolType, Settings};
use crate::writer::file_writer::FileWriterCommand;
use std::sync::mpsc::SyncSender;
use std::sync::Arc;
use std::thread::JoinHandle;

pub mod tcp_server;
pub mod udp_server;

pub struct Listener;

impl Listener {
    pub fn start(
        settings: Arc<Settings>,
        sender: SyncSender<FileWriterCommand>,
    ) -> Vec<JoinHandle<()>> {
        match settings.server.protocol {
            ProtocolType::TCP => TcpServer::start(settings.clone(), sender),
            ProtocolType::UDP => UdpServer::start(settings.clone(), sender),
        }
    }
}
