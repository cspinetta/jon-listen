
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
    let msg_to_send = "hello\n".as_ref();

    info!("Settings: {:?}", settings);

    let (file_writer_tx, file_writer_rx) = sync_channel(settings.buffer_bound);
    let (server_addr_tx, server_addr_rx) = oneshot::channel();
    let (stop_c, stop_p) = oneshot::channel::<()>();

    let server_addr = format!("{}:{}", settings.server.host, settings.server.port).parse::<SocketAddr>().unwrap();
    let settings_ref = settings.clone();

    let server_join = thread::spawn(move || {

        let mut l = Core::new().unwrap();
        let handle = l.handle();

        let socket = tokio_core::net::UdpSocket::bind(&server_addr, &handle).unwrap();
        server_addr_tx.complete(socket.local_addr().unwrap());

        let server = udp_server::UdpServer::new(socket, file_writer_tx, 1, settings_ref);
        let server = server.select(stop_p.map_err(|_| panic!()));
        let server = server.map_err(|_| ());

        l.run(server).unwrap();
    });

    let server_addr = server_addr_rx.wait().unwrap();

    let any_addr = "127.0.0.1:0".to_string().parse::<SocketAddr>().unwrap();
    let client = std::net::UdpSocket::bind(&any_addr).unwrap();

    client.send_to(&msg_to_send, &server_addr).unwrap();

    let received_msg = file_writer_rx.recv_timeout(Duration::from_secs(4));

    info!("Received message: {:?}", &received_msg);

    assert!(received_msg.is_ok());
    assert!(matches!(received_msg, Ok(FileWriterCommand::Write(ref v)) if v.as_slice() == msg_to_send));

    stop_c.complete(());
    server_join.join().unwrap();
}
