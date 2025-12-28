use log::info;

use jon_listen::settings::*;
use jon_listen::writer::file_rotation::*;
use jon_listen::writer::file_writer::FileWriterCommand;
use jon_listen::writer::rotation_policy::*;

use std::path::PathBuf;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use tokio::sync::{broadcast, mpsc};

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
        duration: Option::Some(1),
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
async fn it_rotate_by_duration() {
    pretty_env_logger::init();

    let settings = settings_template();

    info!("Settings: {:?}", settings);

    let (file_writer_tx, mut file_writer_rx) = mpsc::channel(settings.buffer_bound);

    let mut file_path = settings.filewriter.filedir.clone();
    file_path.push(settings.filewriter.filename.clone());

    let rotation_policy: Box<dyn RotationPolicy> = match settings.filewriter.rotation.policy {
        RotationPolicyType::ByDuration => Box::new(RotationByDuration::new(Duration::from_secs(
            settings.filewriter.rotation.duration.unwrap(),
        ))),
        RotationPolicyType::ByDay => Box::new(RotationByDay::new()),
    };

    let file_rotation = FileRotation::new(
        settings.filewriter.filedir.clone(),
        file_path.clone(),
        settings.filewriter.filename.clone(),
        settings.filewriter.rotation.count,
        rotation_policy,
        file_writer_tx.clone(),
    );

    // Create shutdown channel for the test (won't be used, but required by API)
    let (shutdown_tx, _) = broadcast::channel(1);
    let shutdown_rx = shutdown_tx.subscribe();

    let _rotation_handle = file_rotation.start_async(shutdown_rx);

    let received_msg = tokio::time::timeout(
        Duration::from_secs(settings.filewriter.rotation.duration.unwrap() + 5),
        file_writer_rx.recv(),
    )
    .await;
    assert!(received_msg.is_ok());
    let msg = received_msg.unwrap();
    assert!(msg.is_some());
    assert!(matches!(
        msg,
        Some(FileWriterCommand::Rename(_new_filename))
    ));

    //    settings.file_writer.join().unwrap();
}
