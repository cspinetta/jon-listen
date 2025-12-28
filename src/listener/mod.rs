use crate::listener::tcp_server::TcpServer;
use crate::listener::udp_server::UdpServer;
use crate::settings::{ProtocolType, Settings};
use crate::writer::backpressure::BackpressureAwareSender;
use std::io;
use std::sync::Arc;
use tokio::sync::broadcast;

pub mod metrics;
pub mod tcp_server;
pub mod udp_server;

pub struct Listener;

impl Listener {
    pub async fn start(
        settings: Arc<Settings>,
        sender: BackpressureAwareSender,
        shutdown_rx: broadcast::Receiver<()>,
    ) -> Result<(), io::Error> {
        match settings.server.protocol {
            ProtocolType::TCP => TcpServer::start(settings, sender, shutdown_rx).await,
            ProtocolType::UDP => UdpServer::start(settings, sender, shutdown_rx).await,
        }
    }
}
