
use std::{env, io};
use std::net::SocketAddr;

use futures::{Future, Poll};
use tokio_core::net::UdpSocket;
use tokio_core::reactor::Core;

use std::fs::File;
use std::io::prelude::*;

use ::file_writer::FileWriter;

use std::sync::atomic::{AtomicPtr, Ordering};
use std::sync::Arc;
use std::sync::Mutex;


pub struct UdpServer {
    pub socket: UdpSocket,
    pub buf: Vec<u8>,
    pub to_send: Option<(usize, SocketAddr)>,
    pub file_writer: Arc<FileWriter>,
}

impl UdpServer {

    pub fn new(s: UdpSocket, file_writer: Arc<FileWriter>) -> Self {

        UdpServer {
            socket: s,
            to_send: None,
            buf: vec![0u8; 15000],
            file_writer
        }
    }

}

impl Future for UdpServer {
    type Item = ();
    type Error = io::Error;

    fn poll(&mut self) -> Poll<(), io::Error> {
        info!("Enter to UdpServer::poll()");

        loop {
            let (size, peer): (usize, SocketAddr) = try_nb!(self.socket.recv_from(&mut self.buf));
            self.file_writer.write(Arc::new(&self.buf[..size]));
        }
    }
}
