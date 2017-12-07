
use std::{env, io};
use std::net::SocketAddr;

use futures::{Future, Poll};
use tokio_core::net::UdpSocket;
use tokio_core::reactor::Core;

use std::fs::File;
use std::io::prelude::*;

use ::file_provider::FileProvider;

use std::sync::atomic::{AtomicPtr, Ordering};
use std::sync::Arc;
use std::sync::Mutex;


pub struct UdpServer {
    pub socket: UdpSocket,
    pub buf: Vec<u8>,
    pub to_send: Option<(usize, SocketAddr)>,
    pub file_provider: FileProvider,
}
impl UdpServer {

    pub fn new(s: UdpSocket, file_provider: FileProvider) -> Self {

        UdpServer {
            socket: s,
            to_send: None,
            buf: vec![0u8; 15000],
            file_provider
        }
    }

}

impl Future for UdpServer {
    type Item = ();
    type Error = io::Error;

    fn poll(&mut self) -> Poll<(), io::Error> {
        debug!("Enter to UdpServer::poll()");

        loop {
            let (size, peer): (usize, SocketAddr) = try_nb!(self.socket.recv_from(&mut self.buf));
            let mut file: Arc<Mutex<File>> = self.file_provider.get().clone();
            let mut file = file.lock().unwrap();
            (*file).write(&self.buf[..size])?;
            (*file).flush()?;
            (*file).sync_data()?;
        }
    }
}
