#[macro_use]
extern crate log;
extern crate pretty_env_logger;
extern crate config;
extern crate serde;

#[macro_use]
extern crate serde_derive;

extern crate futures;
#[macro_use]
extern crate tokio_core;
#[macro_use]
extern crate tokio_io;

mod server;
mod file_provider;
mod settings;

use std::{env, io};
use std::net::SocketAddr;
use std::sync::Arc;

use futures::{Future, Poll};
use tokio_core::net::UdpSocket;
use tokio_core::reactor::Core;

use settings::Settings;
use server::UdpServer;
use file_provider::FileProvider;

fn main() {
    pretty_env_logger::init().unwrap();

    info!("Starting server...");

    let settings = Settings::new();

    start_server(Arc::new(settings));

}

fn start_server(settings: Arc<Settings>) {

    let addr = format!("{}:{}", settings.server.host, settings.server.port).parse::<SocketAddr>().unwrap();

    // Create the event loop that will drive this server, and also bind the
    // socket we'll be listening to.
    let mut l = Core::new().unwrap();
    let handle = l.handle();
    let socket = UdpSocket::bind(&addr, &handle).unwrap();
    info!("Listening on: {}", socket.local_addr().unwrap());

    let file_provider = FileProvider::new(settings.file.filepath.clone());

    // Next we'll create a future to spawn (the one we defined above) and then
    // we'll run the event loop by running the future.

    l.run(UdpServer::new(socket, file_provider)).unwrap();
}

