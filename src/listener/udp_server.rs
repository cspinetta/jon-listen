

use tokio_core::net::UdpSocket;
use tokio_core::reactor::Core;
use net2;
use net2::unix::UnixUdpBuilderExt;

use std::io;
use std::net::SocketAddr;

use futures::{Future, Poll};

use ::writer::file_writer::FileWriterCommand;
use ::settings::Settings;

use std::thread::{self, JoinHandle};
use std::sync::Arc;
use std::sync::mpsc::SyncSender;

pub struct UdpServer;

impl UdpServer {

    pub fn start(settings: Arc<Settings>, sender: SyncSender<FileWriterCommand>) -> Vec<JoinHandle<()>> {

        let addr = format!("{}:{}", settings.server.host, settings.server.port).parse::<SocketAddr>().unwrap();
        let addr = Arc::new(addr);

        let mut threads: Vec<JoinHandle<()>> = Vec::new();

        for i in 0..settings.threads {
            let settings_ref = settings.clone();
            let tx_file_writer = sender.clone();
            let addr_ref = addr.clone();
            threads.push(thread::spawn(move || {
                info!("Spawning thread {}", i);

                let mut l = Core::new().unwrap();
                let handle = l.handle();

                let udp_socket = net2::UdpBuilder::new_v4().unwrap()
                    .reuse_port(true).unwrap()
                    .bind(addr_ref.as_ref()).unwrap();

                let socket = UdpSocket::from_socket(udp_socket, &handle).unwrap(); // UdpSocket::bind(&addr_ref, &handle).unwrap();
                l.run(UdpService::new(socket, tx_file_writer, i, settings_ref)).unwrap();
            }));
        }

        info!("Listening at {} via UDP with {} threads...", addr, settings.threads);

        threads
    }
}

pub struct UdpService {
    pub id: i32,
    pub name: String,
    pub socket: UdpSocket,
    pub buf: Vec<u8>,
    pub to_send: Option<(usize, SocketAddr)>,
    pub tx_file_writer: SyncSender<FileWriterCommand>,
    settings: Arc<Settings>,
    count: i32
}

impl UdpService {

    pub fn new(s: UdpSocket, tx_file_writer: SyncSender<FileWriterCommand>, id: i32, settings: Arc<Settings>) -> Self {

        UdpService {
            id,
            name: format!("server-udp-{}", id),
            socket: s,
            to_send: None,
            buf: vec![0u8; 15000],
            tx_file_writer,
            settings,
            count: 0
        }
    }

}

impl Future for UdpService {
    type Item = ();
    type Error = io::Error;

    fn poll(&mut self) -> Poll<(), io::Error> {
        loop {
            let (size, _): (usize, SocketAddr) = try_nb!(self.socket.recv_from(&mut self.buf));
            self.count += 1;
            if self.settings.debug {
                info!("Poll datagram from server {}. Count: {}", self.id, self.count);
                self.tx_file_writer.send(FileWriterCommand::WriteDebug(self.name.clone(), self.buf[..size].to_vec(), self.count));
            } else {
                debug!("Poll datagram from server {}. Count: {}", self.id, self.count);
                self.tx_file_writer.send(FileWriterCommand::Write(self.buf[..size].to_vec()));
            }
        }
    }
}
