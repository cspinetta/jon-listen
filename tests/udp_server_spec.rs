
#[macro_use]
extern crate log;
extern crate pretty_env_logger;
#[macro_use]
extern crate matches;

extern crate tokio_core;
#[macro_use]
extern crate tokio_io;

extern crate futures;

extern crate jon_listen;
extern crate net2;


use jon_listen::listener::udp_server;
use jon_listen::writer::file_writer::FileWriterCommand;
use jon_listen::settings::*;

use tokio_core::net::UdpSocket;
use tokio_core::reactor::Core;
use net2::unix::UnixUdpBuilderExt;
use std::time::{SystemTime, UNIX_EPOCH, Duration};
use std::path::PathBuf;

use std::net::SocketAddr;
use std::thread;
use std::sync::Arc;

use futures::sync::oneshot;
use futures::{Future, Poll};
use std::sync::mpsc::{sync_channel, SyncSender, Receiver};


fn settings_template() -> Settings {
    let now = SystemTime::now().duration_since(UNIX_EPOCH).expect("Time went backwards");
    let filename = format!("udp_server_test_{:?}", now);
    let server = Server { host: "0.0.0.0".to_string(), port: 0 };
    let rotation_policy_type = RotationPolicyType::ByDuration;
    let file_config = FileConfig { filedir: PathBuf::from(r"/tmp/"), filename, rotations: 10, duration: Option::default(), rotation_policy_type };
    Settings { debug: false, threads: 1, buffer_bound: 20, server, file_writer: file_config }
}

#[test]
fn it_receives_messages() {
    pretty_env_logger::init().unwrap();

    let settings = Arc::new(settings_template());
    info!("Settings: {:?}", settings);

    let settings_ref = settings.clone();

    let (tx_file_writer, rx_file_writer) = sync_channel(settings.buffer_bound);
    let addr = Arc::new(format!("{}:{}", settings.server.host, settings.server.port).parse::<SocketAddr>().unwrap());
    let addr_ref = addr.clone();

    let (server_addr_tx, server_addr_rx) = oneshot::channel();

    let join_handle = thread::spawn(move || {

        let mut l = Core::new().unwrap();
        let handle = l.handle();

        let socket = tokio_core::net::UdpSocket::bind(addr_ref.as_ref(), &handle).unwrap();
        server_addr_tx.complete(socket.local_addr().unwrap());

        l.run(udp_server::UdpServer::new(socket, tx_file_writer, 1, settings_ref)).unwrap();
    });

    let server_addr = server_addr_rx.wait().unwrap();

    let any_addr = "127.0.0.1:0".to_string().parse::<SocketAddr>().unwrap();
    let client = std::net::UdpSocket::bind(&any_addr).unwrap();

    let payload = "hello\n".as_ref();

    client.send_to(&payload, &server_addr).unwrap();

    let message = rx_file_writer.recv_timeout(Duration::from_secs(4));

    info!("Message received: {:?}", &message);
    assert!(message.is_ok());
    assert!(matches!(message, Ok(FileWriterCommand::Write(ref v)) if v.as_slice() == payload));
}
