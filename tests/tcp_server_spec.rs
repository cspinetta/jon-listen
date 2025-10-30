use log::{debug, info};

use jon_listen::settings::*;
use jon_listen::writer::file_writer::FileWriterCommand;

use std::path::PathBuf;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use std::sync::Arc;
use std::thread;

use futures::StreamExt;
use std::io::Write;
use std::sync::mpsc::sync_channel;
use tokio::net::TcpListener;
use tokio_util::codec::{FramedRead, LinesCodec};

fn settings_template() -> Settings {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("Time went backwards");
    let filename = format!("writer_test_{}.log", now.subsec_nanos());
    let server = ServerConfig {
        protocol: ProtocolType::TCP,
        host: "0.0.0.0".to_string(),
        port: 9999,
    };
    let rotation_policy_config = RotationPolicyConfig {
        count: 10,
        policy: RotationPolicyType::ByDuration,
        duration: Option::default(),
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
    };
    Settings {
        debug: false,
        threads: 1,
        buffer_bound: 20,
        server,
        filewriter: file_config,
    }
}

#[test]
fn it_receives_multiple_messages() {
    pretty_env_logger::init();

    let settings = Arc::new(settings_template());
    let msgs: Vec<String> = (0..100).map(|i| format!("Message # {}\n", i)).collect();

    info!("Settings: {:?}", settings);

    let (file_writer_tx, file_writer_rx) = sync_channel(settings.buffer_bound);
    let (addr_tx, addr_rx) = std::sync::mpsc::channel();

    // Start minimal TCP server using Tokio 1
    thread::spawn(move || {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        rt.block_on(async move {
            let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
            addr_tx.send(listener.local_addr().unwrap()).unwrap();
            loop {
                let (stream, _) = listener.accept().await.unwrap();
                let tx = file_writer_tx.clone();
                tokio::spawn(async move {
                    let mut reader = FramedRead::new(stream, LinesCodec::new());
                    while let Some(line) = reader.next().await {
                        match line {
                            Ok(l) => {
                                let mut v = l.into_bytes();
                                v.push(b'\n');
                                let _ = tx.send(FileWriterCommand::Write(v));
                            }
                            Err(_) => break,
                        }
                    }
                });
            }
        });
    });

    let server_addr = addr_rx.recv().unwrap();

    {
        let mut conn = std::net::TcpStream::connect(server_addr).unwrap();

        for msg in &msgs {
            let _ = conn.write(msg.as_ref());
        }
    }

    for msg in &msgs {
        let msg: &[u8] = msg.as_ref();
        let received_msg = file_writer_rx.recv_timeout(Duration::from_secs(4));
        debug!(
            "Received: {:?} . It should be {:?}",
            received_msg,
            msg.to_ascii_lowercase()
        );
        assert!(received_msg.is_ok());
        assert!(matches!(received_msg, Ok(FileWriterCommand::Write(ref v)) if v.as_slice() == msg));
    }

    info!("Received {} messages successfully", msgs.len());
}
