
use ::settings::{Settings, ProtocolType};
use listener::tcp_server::TcpServer;
use listener::udp_server::UdpServer;
use std::sync::Arc;
use std::thread::{self, JoinHandle};
use std::sync::mpsc::SyncSender;
use ::writer::file_writer::FileWriterCommand;

pub mod udp_server;
pub mod tcp_server;


pub struct Listener;

impl Listener {
    pub fn start(settings: Arc<Settings>, sender: SyncSender<FileWriterCommand>) -> Vec<JoinHandle<()>> {
        match settings.server.protocol {
            ProtocolType::TCP => TcpServer::start(settings.clone(), sender),
            ProtocolType::UDP => UdpServer::start(settings.clone(), sender)
        }
    }
}