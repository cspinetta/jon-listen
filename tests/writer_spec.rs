use log::info;

use jon_listen::settings::{BackpressurePolicy, *};
use jon_listen::writer::file_writer::FileWriterCommand;
use jon_listen::writer::file_writer::*;

use std::fs::{self, File};

use std::io::prelude::*;
use std::io::BufReader;

use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::broadcast;

fn settings_template() -> Settings {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("Time went backwards");
    let filename = format!("writer_test_{}.log", now.subsec_nanos());
    let server = ServerConfig {
        protocol: ProtocolType::UDP,
        host: "0.0.0.0".to_string(),
        port: 0,
        max_connections: 1000,
    };
    let rotation_policy_config = RotationPolicyConfig {
        count: 10,
        policy: RotationPolicyType::ByDuration,
        duration: Option::Some(9999999),
    };
    let formatting_config = FormattingConfig {
        startingmsg: false,
        endingmsg: false,
    };
    let file_config = FileWriterConfig {
        filedir: PathBuf::from(r"/tmp/"),
        filename,
        rotation: rotation_policy_config,
        formatting: formatting_config,
        backpressure_policy: BackpressurePolicy::Block,
    };
    Settings {
        debug: false,
        threads: 1,
        buffer_bound: 20,
        server,
        filewriter: file_config,
        metrics_port: 9090,
    }
}

#[tokio::test]
async fn it_writes_multiple_messages() {
    pretty_env_logger::init();

    let settings = settings_template();
    let msgs: Vec<String> = (0..100).map(|i| format!("Message # {}", i)).collect();

    info!("Settings: {:?}", settings);

    let mut file_writer = FileWriter::new(settings.buffer_bound, settings.filewriter.clone())
        .await
        .expect("Failed to create FileWriter");

    let file_writer_tx = file_writer.tx.clone();

    // Create shutdown channels for the test (won't be used, but required by API)
    let (shutdown_tx, _) = broadcast::channel(1);
    let shutdown_rx1 = shutdown_tx.subscribe();
    let shutdown_rx2 = shutdown_tx.subscribe();

    // Start Writer
    let _join_handle = tokio::spawn(async move {
        file_writer
            .start(shutdown_rx1, shutdown_rx2)
            .await
            .expect("FileWriter failed");
    });

    // Send messages
    info!("Sending {} messages", msgs.len());
    for msg in &msgs {
        let _ = file_writer_tx
            .send(FileWriterCommand::Write(msg.as_bytes().to_vec()))
            .await;
    }

    // Give the writer time to process messages
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    let mut file_path = settings.filewriter.filedir.clone();
    file_path.push(settings.filewriter.filename.clone());

    info!("Log file {:?}", file_path);

    {
        let file = File::open(file_path.clone())
            .unwrap_or_else(|_| panic!("Open the log file {:?}", file_path));
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

    let _ = fs::remove_file(file_path);
}
