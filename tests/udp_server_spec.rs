use log::info;

use jon_listen::listener::udp_server::UdpService;
use jon_listen::settings::*;
use jon_listen::writer::file_writer::FileWriterCommand;

use std::path::PathBuf;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use std::net::SocketAddr;
use std::sync::Arc;
use std::thread;

use std::sync::mpsc::sync_channel;
use tokio::net::UdpSocket;

fn settings_template() -> Settings {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("Time went backwards");
    let filename = format!("writer_test_{}.log", now.subsec_nanos());
    let server = ServerConfig {
        protocol: ProtocolType::UDP,
        host: "0.0.0.0".to_string(),
        port: 0,
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
    let msgs: Vec<String> = (0..100).map(|i| format!("Message # {}", i)).collect();

    info!("Settings: {:?}", settings);

    let (file_writer_tx, file_writer_rx) = sync_channel(settings.buffer_bound);
    let (addr_tx, addr_rx) = std::sync::mpsc::channel();

    let settings_ref = settings.clone();
    let join = thread::spawn(move || {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        rt.block_on(async move {
            let bind_addr = format!("{}:{}", settings_ref.server.host, settings_ref.server.port)
                .parse::<SocketAddr>()
                .unwrap();
            let socket = UdpSocket::bind(bind_addr).await.unwrap();
            let local = socket.local_addr().unwrap();
            addr_tx.send(local).unwrap();
            let mut service = UdpService::new(socket, file_writer_tx, 1, settings_ref);
            // Run until aborted by dropping the runtime after assertions
            let _ = service.run().await;
        });
    });

    let mut server_addr = addr_rx.recv().unwrap();
    if server_addr.ip().is_unspecified() {
        server_addr = SocketAddr::from(([127, 0, 0, 1], server_addr.port()));
    }

    let any_addr = "127.0.0.1:0".to_string().parse::<SocketAddr>().unwrap();
    let client = std::net::UdpSocket::bind(any_addr).unwrap();

    for msg in &msgs {
        client.send_to(msg.as_ref(), server_addr).unwrap();
    }

    for msg in &msgs {
        let msg: &[u8] = msg.as_ref();
        let received_msg = file_writer_rx.recv_timeout(Duration::from_secs(4));
        assert!(received_msg.is_ok());
        assert!(matches!(received_msg, Ok(FileWriterCommand::Write(ref v)) if v.as_slice() == msg));
    }

    info!("Received {} messages successfully", msgs.len());

    // End the background task by ending the runtime thread
    drop(join);
}
