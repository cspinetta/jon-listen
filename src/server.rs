
//extern crate futures;
//#[macro_use]
//extern crate tokio_core;
//#[macro_use]
//extern crate tokio_io;

use std::{env, io};
use std::net::SocketAddr;

use futures::{Future, Poll};
use tokio_core::net::UdpSocket;
use tokio_core::reactor::Core;

use std::fs::File;
use std::io::prelude::*;
use std::fs::OpenOptions;

pub struct UdpServer {
    pub socket: UdpSocket,
    pub buf: Vec<u8>,
    pub to_send: Option<(usize, SocketAddr)>,
    pub file: File,
}
impl UdpServer {

    pub fn new(s: UdpSocket) -> Self {

        let filepath = "foo.txt";
        let mut options = OpenOptions::new();
        let mut file: File = options.append(true).create(true).open(filepath).unwrap();
//        let metadata = file.metadata()?;
//        println!("File metadata: {:?}", metadata);

        UdpServer {
            socket: s,
            to_send: None,
            buf: vec![0u8; 1600],
            file
        }
    }

}

impl Future for UdpServer {
    type Item = ();
    type Error = io::Error;

    fn poll(&mut self) -> Poll<(), io::Error> {
        println!("Enter to UdpServer::poll()");

        loop {
            let (size, peer): (usize, SocketAddr) = try_nb!(self.socket.recv_from(&mut self.buf));
            self.file.write(&self.buf[..size])?;
            self.file.flush()?;
            self.file.sync_data()?;
        }
    }
}
