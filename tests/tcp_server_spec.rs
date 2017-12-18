
#[macro_use]
extern crate log;
extern crate pretty_env_logger;
#[macro_use]
extern crate matches;

extern crate tokio_core;
extern crate tokio_io;

extern crate futures;

extern crate jon_listen;
extern crate net2;


use jon_listen::listener::tcp_server::start_tcp_server;
use jon_listen::writer::file_writer::FileWriterCommand;
use jon_listen::settings::*;

use std::time::{SystemTime, UNIX_EPOCH, Duration};
use std::path::PathBuf;

use std::net::SocketAddr;
use std::thread;
use std::sync::Arc;

use std::sync::mpsc::sync_channel;
use std::io::Write;


fn settings_template() -> Settings {
    let now = SystemTime::now().duration_since(UNIX_EPOCH).expect("Time went backwards");
    let filename = format!("writer_test_{}.log", now.subsec_nanos());
    let server = ServerConfig { protocol: ProtocolType::TCP, host: "0.0.0.0".to_string(), port: 9999 };
    let rotation_policy_config = RotationPolicyConfig { count: 10, policy: RotationPolicyType::ByDuration, duration: Option::default() };
    let formatting_config = FormattingConfig { startingmsg: false, endingmsg: false };
    let file_config = FileWriterConfig { filedir: PathBuf::from(r"/tmp/"), filename, rotation: rotation_policy_config, formatting: formatting_config };
    Settings { debug: false, threads: 1, buffer_bound: 20, server, filewriter: file_config }
}

#[test]
fn it_receives_multiple_messages() {
    pretty_env_logger::init().unwrap();

    let settings = Arc::new(settings_template());
    let msgs: Vec<String> = (0..100).map(|i| format!("Message # {}\n", i)).collect();

    info!("Settings: {:?}", settings);

    let server_addr = format!("{}:{}", settings.server.host, settings.server.port).parse::<SocketAddr>().unwrap();
    let (file_writer_tx, file_writer_rx) = sync_channel(settings.buffer_bound);
    let threads = start_tcp_server(settings.clone(), file_writer_tx.clone());

    // To force server to get ready
    thread::sleep_ms(1);

    {
        let mut conn = std::net::TcpStream::connect(server_addr).unwrap();

        for msg in &msgs {
            conn.write(msg.as_ref());
        }
    }

    for msg in &msgs {
        let msg: &[u8] = msg.as_ref();
        let received_msg = file_writer_rx.recv_timeout(Duration::from_secs(4));
        debug!("Received: {:?} . It should be {:?}", received_msg, msg.to_ascii_lowercase());
        assert!(received_msg.is_ok());
        assert!(matches!(received_msg, Ok(FileWriterCommand::Write(ref v)) if v.as_slice() == msg));
    }

    info!("Received {} messages successfully", msgs.len());
}
