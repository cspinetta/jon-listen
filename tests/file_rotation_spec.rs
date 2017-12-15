
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
use jon_listen::writer::file_rotation::*;
use jon_listen::writer::rotation_policy::*;
use jon_listen::settings::*;

use tokio_core::net::UdpSocket;
use tokio_core::reactor::Core;
use net2::unix::UnixUdpBuilderExt;
use std::time::{SystemTime, UNIX_EPOCH, Duration};
use std::path::PathBuf;

use std::net::SocketAddr;
use std::thread::{self, JoinHandle};
use std::sync::Arc;

use futures::sync::oneshot;
use futures::{Future, Poll};
use std::sync::mpsc::{sync_channel, SyncSender, Receiver};


fn settings_template() -> Settings {
    let now = SystemTime::now().duration_since(UNIX_EPOCH).expect("Time went backwards");
    let filename = format!("writer_test_{}.log", now.subsec_nanos());
    let server = Server { host: "0.0.0.0".to_string(), port: 0 };
    let rotation_policy_type = RotationPolicyType::ByDuration;
    let file_config = FileConfig { filedir: PathBuf::from(r"/tmp/"), filename, rotations: 10, duration: Option::Some(1), rotation_policy_type };
    Settings { debug: false, threads: 1, buffer_bound: 20, server, file_writer: file_config }
}

#[test]
fn it_rotate_by_duration() {
    pretty_env_logger::init().unwrap();

    let settings = settings_template();

    info!("Settings: {:?}", settings);

    let (file_writer_tx, file_writer_rx) = sync_channel(settings.buffer_bound);

    let mut file_path = settings.file_writer.filedir.clone();
    file_path.push(settings.file_writer.filename.clone());

    let rotation_policy: Box<RotationPolicy> = match settings.file_writer.rotation_policy_type {
        RotationPolicyType::ByDuration => Box::new(RotationByDuration::new(Duration::from_secs(settings.file_writer.duration.unwrap()))),
        RotationPolicyType::ByDay => Box::new(RotationByDay::new())
    };

    let file_rotation = FileRotation::new(
        settings.file_writer.filedir.clone(), file_path.clone(),
        settings.file_writer.filename.clone(), settings.file_writer.rotations, rotation_policy, file_writer_tx.clone());

    let rotation_handle: JoinHandle<Result<(), String>> = file_rotation.start_async();

    let received_msg = file_writer_rx.recv_timeout(Duration::from_secs(settings.file_writer.duration.unwrap() + 5));
    assert!(received_msg.is_ok());
    assert!(matches!(received_msg, Ok(FileWriterCommand::Rename(new_filename))));

//    settings.file_writer.join().unwrap();
}
