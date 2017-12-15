
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


use jon_listen::writer::file_writer::*;
use jon_listen::listener::udp_server;
use jon_listen::writer::file_writer::FileWriterCommand;
use jon_listen::settings::*;


use std::fs::{self, File};

use std::io::{self, BufReader};
use std::io::prelude::*;

use std::time::{SystemTime, UNIX_EPOCH, Duration};
use std::path::PathBuf;

use std::sync::mpsc::{sync_channel, SyncSender, Receiver};


fn settings_template() -> Settings {
    let now = SystemTime::now().duration_since(UNIX_EPOCH).expect("Time went backwards");
    let filename = format!("writer_test_{}.log", now.subsec_nanos());
    let server = Server { host: "0.0.0.0".to_string(), port: 0 };
    let rotation_policy_type = RotationPolicyType::ByDuration;
    let file_config = FileConfig { filedir: PathBuf::from(r"/tmp/"), filename, rotations: 10, duration: Option::Some(9999999), rotation_policy_type };
    Settings { debug: false, threads: 1, buffer_bound: 20, server, file_writer: file_config }
}

#[test]
fn it_writes_multiple_messages() {
    pretty_env_logger::init().unwrap();

    let settings = settings_template();
    let msgs: Vec<String> = (0..100).map(|i| format!("Message # {}", i)).collect();

    info!("Settings: {:?}", settings);

    let mut file_writer = FileWriter::new(settings.buffer_bound, settings.file_writer.clone());

    let file_writer_tx = file_writer.tx.clone();

    // Start Writer
    let join_handle = file_writer.start_async();

    // Send messages
    info!("Sending {} messages", msgs.len());
    for msg in &msgs {
        file_writer_tx.send(FileWriterCommand::Write(msg.as_bytes().to_vec()));
    }

    let mut file_path = settings.file_writer.filedir.clone();
    file_path.push(settings.file_writer.filename.clone());

    info!("Log file {:?}", file_path);

    {
        let file = File::open(file_path.clone()).expect(format!("Open the log file {:?}", file_path).as_ref());
        let file_reader = BufReader::new(file);

        let mut msg_iter = msgs.iter();
        for line in file_reader.lines() {
            let next = msg_iter.next();
            assert!(next.is_some());
            assert!(line.is_ok());

            let line_writer = line.unwrap();
            let line_writer = line_writer.as_bytes();

            let line_file = next.unwrap();
            let line_file = line_file.as_bytes();

            assert_eq!(line_writer, line_file);

//            println!("{:?} - {:?}", String::from_utf8_lossy(line_writer), String::from_utf8_lossy(line_file));
        }
    }

    fs::remove_file(file_path);
}
