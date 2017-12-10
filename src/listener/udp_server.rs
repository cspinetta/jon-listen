
use std::{env, io};
use std::net::SocketAddr;

use futures::{Future, Poll};
use tokio_core::net::UdpSocket;
use tokio_core::reactor::Core;

use std::fs::File;
use std::io::prelude::*;

use ::writer::file_writer::FileWriterCommand;

use std::sync::atomic::{AtomicPtr, Ordering};
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::mpsc::{SyncSender, RecvError};


pub struct UdpServer {
    pub id: i32,
    pub name: String,
    pub socket: UdpSocket,
    pub buf: Vec<u8>,
    pub to_send: Option<(usize, SocketAddr)>,
    pub tx_file_writer: SyncSender<FileWriterCommand>,
    count: i32
}

impl UdpServer {

    pub fn new(s: UdpSocket, tx_file_writer: SyncSender<FileWriterCommand>, id: i32) -> Self {

        UdpServer {
            id,
            name: format!("server-udp-{}", id),
            socket: s,
            to_send: None,
            buf: vec![0u8; 15000],
            tx_file_writer,
            count: 0
        }
    }

}

impl Future for UdpServer {
    type Item = ();
    type Error = io::Error;

    fn poll(&mut self) -> Poll<(), io::Error> {
        loop {
            let (size, peer): (usize, SocketAddr) = try_nb!(self.socket.recv_from(&mut self.buf));
            self.count += 1;
            debug!("Poll datagram from server {}. Count: {}", self.id, self.count);
//            self.tx_file_writer.send(FileWriterCommand::Write(self.buf[..size].to_vec()));
            self.tx_file_writer.send(FileWriterCommand::WriteDebug(self.name.clone(), self.buf[..size].to_vec(), self.count));
        }
    }
}
