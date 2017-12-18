
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


use jon_listen::writer::file_writer::FileWriterCommand;
use jon_listen::writer::file_rotation::*;
use jon_listen::writer::rotation_policy::*;
use jon_listen::settings::*;

use std::time::{SystemTime, UNIX_EPOCH, Duration};
use std::path::PathBuf;

use std::thread::JoinHandle;

use std::sync::mpsc::sync_channel;


fn settings_template() -> Settings {
    let now = SystemTime::now().duration_since(UNIX_EPOCH).expect("Time went backwards");
    let filename = format!("writer_test_{}.log", now.subsec_nanos());
    let server = ServerConfig { protocol: ProtocolType::UDP, host: "0.0.0.0".to_string(), port: 0 };
    let rotation_policy_config = RotationPolicyConfig { count: 10, policy: RotationPolicyType::ByDuration, duration: Option::Some(1) };
    let formatting_config = FormattingConfig { startingmsg: false, endingmsg: false };
    let file_config = FileWriterConfig { filedir: PathBuf::from(r"/tmp/"), filename, rotation: rotation_policy_config, formatting: formatting_config };
    Settings { debug: false, threads: 1, buffer_bound: 20, server, filewriter: file_config }
}

#[test]
fn it_rotate_by_duration() {
    pretty_env_logger::init().unwrap();

    let settings = settings_template();

    info!("Settings: {:?}", settings);

    let (file_writer_tx, file_writer_rx) = sync_channel(settings.buffer_bound);

    let mut file_path = settings.filewriter.filedir.clone();
    file_path.push(settings.filewriter.filename.clone());

    let rotation_policy: Box<RotationPolicy> = match settings.filewriter.rotation.policy {
        RotationPolicyType::ByDuration => Box::new(RotationByDuration::new(Duration::from_secs(settings.filewriter.rotation.duration.unwrap()))),
        RotationPolicyType::ByDay => Box::new(RotationByDay::new())
    };

    let file_rotation = FileRotation::new(
        settings.filewriter.filedir.clone(), file_path.clone(),
        settings.filewriter.filename.clone(), settings.filewriter.rotation.count, rotation_policy, file_writer_tx.clone());

    let rotation_handle: JoinHandle<Result<(), String>> = file_rotation.start_async();

    let received_msg = file_writer_rx.recv_timeout(Duration::from_secs(settings.filewriter.rotation.duration.unwrap() + 5));
    assert!(received_msg.is_ok());
    assert!(matches!(received_msg, Ok(FileWriterCommand::Rename(new_filename))));

//    settings.file_writer.join().unwrap();
}
