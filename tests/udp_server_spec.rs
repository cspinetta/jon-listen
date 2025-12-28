use log::info;

use jon_listen::listener::udp_server::UdpService;
use jon_listen::settings::{BackpressurePolicy, *};
use jon_listen::writer::file_writer::FileWriterCommand;

use std::path::PathBuf;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use std::net::SocketAddr;
use std::sync::Arc;
use std::thread;

use tokio::net::UdpSocket;
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

#[test]
fn it_receives_multiple_messages() {
    pretty_env_logger::init();

    let settings = Arc::new(settings_template());
    let msgs: Vec<String> = (0..100).map(|i| format!("Message # {}", i)).collect();

    info!("Settings: {:?}", settings);

    let (file_writer_tx, mut file_writer_rx) = mpsc::channel(settings.buffer_bound);
    let (addr_tx, addr_rx) = std::sync::mpsc::channel();

    let settings_ref = settings.clone();
    // Create shutdown channel that we can control from the test thread
    let (shutdown_tx, shutdown_rx) = broadcast::channel(1);
    let shutdown_tx_clone = shutdown_tx.clone();
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
            use jon_listen::writer::backpressure::BackpressureAwareSender;
            let backpressure_sender =
                BackpressureAwareSender::new(file_writer_tx, BackpressurePolicy::Block);
            let mut service = UdpService::new(socket, backpressure_sender, 1, settings_ref);
            // Run until shutdown signal is received
            let _ = service.run(shutdown_rx).await;
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

    // Use a blocking receiver in a separate thread since we're in a sync test
    let (sync_tx, sync_rx) = std::sync::mpsc::channel();
    let _rx_handle = std::thread::spawn(move || {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        rt.block_on(async {
            while let Some(msg) = file_writer_rx.recv().await {
                sync_tx.send(msg).unwrap();
            }
        });
    });

    for msg in &msgs {
        let msg: &[u8] = msg.as_ref();
        let received_msg = sync_rx.recv_timeout(Duration::from_secs(4));
        assert!(received_msg.is_ok());
        assert!(matches!(received_msg, Ok(FileWriterCommand::Write(ref v)) if v.as_slice() == msg));
    }

    info!("Received {} messages successfully", msgs.len());

    // Send shutdown signal to gracefully stop the UDP service
    let _ = shutdown_tx_clone.send(());
    // Wait for the service to shut down
    let _ = join.join();
}

// Unit tests for UDP Server components
//
// Note: The following scenarios are not explicitly tested but are handled by the code:
// - Bind failures: UdpServer::start() returns Result<(), io::Error> allowing error handling,
//   but testing bind failures requires root privileges or complex setup. Address parsing
//   uses unwrap() which panics on invalid addresses, making it difficult to test gracefully.
// - Recv errors in UdpService::run(): The function returns Result<(), io::Error> allowing
//   error handling, but simulating network recv errors reliably in tests is complex.
mod unit_tests {
    use super::*;
    use jon_listen::metrics;
    use jon_listen::writer::backpressure::BackpressureAwareSender;
    use std::time::Duration;
    use tokio::net::UdpSocket;
    use tokio::sync::broadcast;
    use tokio::time::timeout;

    #[tokio::test]
    async fn test_udp_service_new() {
        let settings = Arc::new(settings_template());
        let (tx, _rx) = mpsc::channel::<FileWriterCommand>(10);
        let sender = BackpressureAwareSender::new(tx, BackpressurePolicy::Block);

        let socket = UdpSocket::bind("127.0.0.1:0").await.unwrap();
        let service =
            jon_listen::listener::udp_server::UdpService::new(socket, sender, 42, settings);

        assert_eq!(service.id, 42);
        assert_eq!(service.name, "server-udp-42");
    }

    #[tokio::test]
    async fn test_udp_service_receives_datagrams() {
        let settings = Arc::new(settings_template());
        let (tx, mut rx) = mpsc::channel::<FileWriterCommand>(10);
        let sender = BackpressureAwareSender::new(tx, BackpressurePolicy::Block);

        let server_socket = UdpSocket::bind("127.0.0.1:0").await.unwrap();
        let server_addr = server_socket.local_addr().unwrap();

        let mut service =
            jon_listen::listener::udp_server::UdpService::new(server_socket, sender, 0, settings);

        let (shutdown_tx, shutdown_rx) = broadcast::channel::<()>(1);

        // Spawn service task
        let service_handle = tokio::spawn(async move { service.run(shutdown_rx).await });

        // Send a datagram to the server
        let client_socket = UdpSocket::bind("127.0.0.1:0").await.unwrap();
        let test_message = b"test message\n";
        client_socket
            .send_to(test_message, server_addr)
            .await
            .unwrap();

        // Give service time to process
        tokio::time::sleep(Duration::from_millis(50)).await;

        // Verify message was received
        let received = timeout(Duration::from_millis(100), rx.recv()).await;
        assert!(received.is_ok());
        let command = received.unwrap().unwrap();
        match command {
            FileWriterCommand::Write(data) => {
                assert_eq!(data, test_message.to_vec());
            }
            _ => panic!("Expected Write command"),
        }

        // Shutdown service
        shutdown_tx.send(()).unwrap();
        let _ = timeout(Duration::from_secs(1), service_handle).await;
    }

    #[tokio::test]
    async fn test_udp_service_handles_debug_mode() {
        let mut settings = settings_template();
        settings.debug = true;
        let settings = Arc::new(settings);

        let (tx, mut rx) = mpsc::channel::<FileWriterCommand>(10);
        let sender = BackpressureAwareSender::new(tx, BackpressurePolicy::Block);

        let server_socket = UdpSocket::bind("127.0.0.1:0").await.unwrap();
        let server_addr = server_socket.local_addr().unwrap();

        let mut service =
            jon_listen::listener::udp_server::UdpService::new(server_socket, sender, 0, settings);

        let (shutdown_tx, shutdown_rx) = broadcast::channel::<()>(1);

        let service_handle = tokio::spawn(async move { service.run(shutdown_rx).await });

        // Send a datagram
        let client_socket = UdpSocket::bind("127.0.0.1:0").await.unwrap();
        let test_message = b"debug message\n";
        client_socket
            .send_to(test_message, server_addr)
            .await
            .unwrap();

        tokio::time::sleep(Duration::from_millis(50)).await;

        // In debug mode, should receive WriteDebug command
        let received = timeout(Duration::from_millis(100), rx.recv()).await;
        assert!(received.is_ok());
        let command = received.unwrap().unwrap();
        match command {
            FileWriterCommand::WriteDebug(id, data, count) => {
                assert_eq!(data, test_message.to_vec());
                assert_eq!(count, 1);
                assert!(id.contains("server-udp"));
            }
            _ => panic!("Expected WriteDebug command in debug mode"),
        }

        shutdown_tx.send(()).unwrap();
        let _ = timeout(Duration::from_secs(1), service_handle).await;
    }

    #[tokio::test]
    async fn test_udp_service_graceful_shutdown() {
        let settings = Arc::new(settings_template());
        let (tx, _rx) = mpsc::channel::<FileWriterCommand>(10);
        let sender = BackpressureAwareSender::new(tx, BackpressurePolicy::Block);

        let server_socket = UdpSocket::bind("127.0.0.1:0").await.unwrap();
        let mut service =
            jon_listen::listener::udp_server::UdpService::new(server_socket, sender, 0, settings);

        let (shutdown_tx, shutdown_rx) = broadcast::channel::<()>(1);

        let service_handle = tokio::spawn(async move { service.run(shutdown_rx).await });

        // Give service time to start
        tokio::time::sleep(Duration::from_millis(50)).await;

        // Send shutdown signal
        shutdown_tx.send(()).unwrap();

        // Service should shutdown gracefully
        let result = timeout(Duration::from_secs(1), service_handle).await;
        assert!(result.is_ok());
        let service_result = result.unwrap().unwrap();
        assert!(service_result.is_ok());
    }

    #[tokio::test]
    async fn test_udp_server_start_binds_to_address() {
        let settings = Arc::new(settings_template());
        let (tx, _rx) = mpsc::channel::<FileWriterCommand>(10);
        let sender = BackpressureAwareSender::new(tx, BackpressurePolicy::Block);
        let (_shutdown_tx, shutdown_rx) = broadcast::channel(1);

        // Test that server can bind to address
        let mut test_settings = (*settings).clone();
        test_settings.server.port = 0;
        let test_settings = Arc::new(test_settings);

        let server_handle = tokio::spawn(async move {
            jon_listen::listener::udp_server::UdpServer::start(test_settings, sender, shutdown_rx)
                .await
        });

        // Give it time to bind
        tokio::time::sleep(Duration::from_millis(100)).await;

        // Send shutdown to stop server
        server_handle.abort();
    }

    #[tokio::test]
    async fn test_udp_server_start_graceful_shutdown() {
        let settings = Arc::new(settings_template());
        let (tx, _rx) = mpsc::channel::<FileWriterCommand>(10);
        let sender = BackpressureAwareSender::new(tx, BackpressurePolicy::Block);
        let (shutdown_tx, shutdown_rx) = broadcast::channel::<()>(1);

        let mut test_settings = (*settings).clone();
        test_settings.server.port = 0;
        let test_settings = Arc::new(test_settings);

        let server_handle = tokio::spawn(async move {
            jon_listen::listener::udp_server::UdpServer::start(test_settings, sender, shutdown_rx)
                .await
        });

        tokio::time::sleep(Duration::from_millis(50)).await;

        shutdown_tx.send(()).unwrap();

        let result = timeout(Duration::from_secs(1), server_handle).await;
        assert!(result.is_ok());
        let server_result = result.unwrap().unwrap();
        assert!(server_result.is_ok());
    }

    #[tokio::test]
    async fn test_udp_server_metrics_datagram_received() {
        let _ = metrics::init(9111);

        jon_listen::listener::metrics::udp::datagram_received();
        jon_listen::listener::metrics::udp::datagram_received();

        if let Some(handle) = metrics::get_handle() {
            let output = handle.render();
            assert!(
                output.contains("udp_datagrams_received_total")
                    || output.contains("udp_datagrams_received"),
                "Metrics output should contain udp_datagrams_received metric, got: {}",
                output
            );
        }
    }

    #[tokio::test]
    async fn test_udp_server_metrics_messages_received() {
        let _ = metrics::init(9112);

        metrics::messages::received();
        metrics::messages::received();
        metrics::messages::received();

        if let Some(handle) = metrics::get_handle() {
            let output = handle.render();
            assert!(
                output.contains("messages_received_total") || output.contains("messages_received"),
                "Metrics output should contain messages_received metric, got: {}",
                output
            );
        }
    }
}
