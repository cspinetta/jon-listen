
#![feature(try_trait)]
#![feature(custom_derive)]
#![feature(custom_attribute)]

#[macro_use]
extern crate log;
extern crate pretty_env_logger;

#[macro_use]
extern crate serde_derive;
extern crate serde;
extern crate toml;
extern crate config;

extern crate net2;

extern crate chrono;
extern crate time;
extern crate glob;
extern crate regex;

extern crate futures;
extern crate tokio_core;
#[macro_use]
extern crate tokio_io;

mod listener;
mod writer;
pub mod settings;

use std::net::SocketAddr;
use std::thread;
use std::sync::Arc;

use tokio_core::net::UdpSocket;
use tokio_core::reactor::Core;

use net2::unix::UnixUdpBuilderExt;

use settings::Settings;
use listener::udp_server::UdpServer;
use writer::file_writer::FileWriter;

pub fn start_server(settings: Arc<Settings>) {

    let addr = format!("{}:{}", settings.server.host, settings.server.port).parse::<SocketAddr>().unwrap();
    let addr = Arc::new(addr);

    let mut file_writer = FileWriter::new(settings.buffer_bound, settings.file_writer.clone());
    let mut threads = Vec::new();

    for i in 0..settings.threads {
        let settings_ref = settings.clone();
        let tx_file_writer = file_writer.tx.clone();
        let addr_ref = addr.clone();
        threads.push(thread::spawn(move || {
            info!("Spawning thread {}", i);

            let mut l = Core::new().unwrap();
            let handle = l.handle();

            let udp_socket = net2::UdpBuilder::new_v4().unwrap()
                .reuse_port(true).unwrap()
                .bind(addr_ref.as_ref()).unwrap();

            let socket = UdpSocket::from_socket(udp_socket, &handle).unwrap(); // UdpSocket::bind(&addr_ref, &handle).unwrap();
            l.run(UdpServer::new(socket, tx_file_writer, i, settings_ref)).unwrap();
        }));
    }

    info!("Listening on {} with {} threads...", addr, settings.threads);

    file_writer.start();
    for t in threads {
        t.join().unwrap();
    }

}