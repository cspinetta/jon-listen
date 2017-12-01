#![feature(test)]

extern crate test;
extern crate futures;
#[macro_use]
extern crate tokio_core;
#[macro_use]
extern crate tokio_io;

use std::io;
use std::net::SocketAddr;
use std::thread;

use futures::sync::oneshot;
use futures::sync::mpsc;
use futures::{Future, Poll};
use test::Bencher;
use tokio_core::net::UdpSocket;
use tokio_core::reactor::Core;

#[path="../src/server.rs"]
mod server;

use server::UdpServer;

#[bench]
fn udp_echo_latency(b: &mut Bencher) {
    let any_addr = "127.0.0.1:0".to_string();
    let any_addr = any_addr.parse::<SocketAddr>().unwrap();

    let (stop_c, stop_p) = oneshot::channel::<()>();
    let (tx, rx) = oneshot::channel();

    let child = thread::spawn(move || {
        let mut l = Core::new().unwrap();
        let handle = l.handle();

        let socket = tokio_core::net::UdpSocket::bind(&any_addr, &handle).unwrap();
        tx.complete(socket.local_addr().unwrap());

        let server = UdpServer::new(socket);
        let server = server.select(stop_p.map_err(|_| panic!()));
        let server = server.map_err(|_| ());
        l.run(server).unwrap()
    });


    let client = std::net::UdpSocket::bind(&any_addr).unwrap();

    let server_addr = rx.wait().unwrap();
    let mut buf = [0u8; 1000];

    // warmup phase; for some reason initial couple of
    // runs are much slower
    //
    // TODO: Describe the exact reasons; caching? branch predictor? lazy closures?
    for _ in 0..8 {
        client.send_to(&buf, &server_addr).unwrap();
        let _ = client.recv_from(&mut buf).unwrap();
    }

    b.iter(|| {
        client.send_to(&buf, &server_addr).unwrap();
        let _ = client.recv_from(&mut buf).unwrap();
    });

    stop_c.complete(());
    child.join().unwrap();
}
