#![feature(test)]

#[macro_use]
extern crate log;
extern crate pretty_env_logger;
#[macro_use]
extern crate matches;

extern crate test;

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

use test::Bencher;

use std::net::SocketAddr;
use std::thread;
use std::sync::Arc;

use futures::sync::oneshot;
use futures::{Future, Poll};
use std::sync::mpsc::{sync_channel, SyncSender, Receiver};


fn settings_template() -> Settings {
    let now = SystemTime::now().duration_since(UNIX_EPOCH).expect("Time went backwards");
    let filename = format!("writer_test_{}.log", now.subsec_nanos());
    let server = ServerConfig { protocol: ProtocolType::UDP, host: "0.0.0.0".to_string(), port: 9999 };
    let rotation_policy_config = RotationPolicyConfig { count: 10, policy: RotationPolicyType::ByDuration, duration: Option::Some(9999) };
    let formatting_config = FormattingConfig { startingmsg: false, endingmsg: false };
    let file_config = FileWriterConfig { filedir: PathBuf::from(r"/tmp/"), filename, rotation: rotation_policy_config, formatting: formatting_config };
    Settings { debug: false, threads: 1, buffer_bound: 20, server, filewriter: file_config }
}

#[bench]
fn app_latency(b: &mut Bencher) {
    pretty_env_logger::init().unwrap();

    let settings = Arc::new(settings_template());
    let settings_ref = settings.clone();

    info!("Settings: {:?}", settings);

    let server_join = thread::spawn(move || {
        jon_listen::start_up(settings_ref);
    });

    let server_addr = format!("{}:{}", settings.server.host, settings.server.port).parse::<SocketAddr>().unwrap();
    let any_addr = "127.0.0.1:0".to_string().parse::<SocketAddr>().unwrap();
    let client = std::net::UdpSocket::bind(&any_addr).unwrap();

    b.iter(|| {
        let msg = "ckdlsncldnclnclcs".to_string();
        client.send_to(msg.as_ref(), &server_addr).unwrap();
    });

}
