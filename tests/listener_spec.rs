use jon_listen::listener::Listener;
use jon_listen::settings::{BackpressurePolicy, *};
use jon_listen::writer::file_writer::FileWriterCommand;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::sync::{broadcast, mpsc};
use tokio::time::timeout;

fn settings_template(protocol: ProtocolType) -> Settings {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("Time went backwards");
    let filename = format!("writer_test_{}.log", now.subsec_nanos());
    let server = ServerConfig {
        protocol,
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

#[tokio::test]
async fn test_listener_routes_to_tcp_server() {
    let tcp_settings = Arc::new(settings_template(ProtocolType::TCP));
    let (tcp_tx, _tcp_rx) = mpsc::channel::<FileWriterCommand>(10);
    let tcp_sender = jon_listen::writer::backpressure::BackpressureAwareSender::new(
        tcp_tx,
        BackpressurePolicy::Block,
    );
    let (tcp_shutdown_tx, tcp_shutdown_rx) = broadcast::channel::<()>(1);

    let tcp_listener_handle =
        tokio::spawn(
            async move { Listener::start(tcp_settings, tcp_sender, tcp_shutdown_rx).await },
        );

    tokio::time::sleep(Duration::from_millis(50)).await;

    let shutdown_sent = tcp_shutdown_tx.send(()).is_ok();
    assert!(shutdown_sent, "Shutdown signal should be sent successfully");

    let tcp_result = timeout(Duration::from_secs(1), tcp_listener_handle).await;
    assert!(
        tcp_result.is_ok(),
        "TCP listener should complete within timeout"
    );
    let tcp_listener_result = tcp_result.unwrap();
    assert!(
        tcp_listener_result.is_ok(),
        "TCP listener task should not panic"
    );
    let tcp_server_result = tcp_listener_result.unwrap();
    assert!(
        tcp_server_result.is_ok(),
        "TCP server should shutdown gracefully, got: {:?}",
        tcp_server_result
    );
}

#[tokio::test]
async fn test_listener_routes_to_udp_server() {
    let udp_settings = Arc::new(settings_template(ProtocolType::UDP));
    let (udp_tx, _udp_rx) = mpsc::channel::<FileWriterCommand>(10);
    let udp_sender = jon_listen::writer::backpressure::BackpressureAwareSender::new(
        udp_tx,
        BackpressurePolicy::Block,
    );
    let (udp_shutdown_tx, udp_shutdown_rx) = broadcast::channel::<()>(1);

    let udp_listener_handle =
        tokio::spawn(
            async move { Listener::start(udp_settings, udp_sender, udp_shutdown_rx).await },
        );

    tokio::time::sleep(Duration::from_millis(50)).await;

    let shutdown_sent = udp_shutdown_tx.send(()).is_ok();
    assert!(shutdown_sent, "Shutdown signal should be sent successfully");

    let udp_result = timeout(Duration::from_secs(1), udp_listener_handle).await;
    assert!(
        udp_result.is_ok(),
        "UDP listener should complete within timeout"
    );
    let udp_listener_result = udp_result.unwrap();
    assert!(
        udp_listener_result.is_ok(),
        "UDP listener task should not panic"
    );
    let udp_server_result = udp_listener_result.unwrap();
    assert!(
        udp_server_result.is_ok(),
        "UDP server should shutdown gracefully, got: {:?}",
        udp_server_result
    );
}

#[tokio::test]
async fn test_listener_passes_shutdown_receiver_correctly() {
    let tcp_settings = Arc::new(settings_template(ProtocolType::TCP));
    let (tcp_tx, _tcp_rx) = mpsc::channel::<FileWriterCommand>(10);
    let tcp_sender = jon_listen::writer::backpressure::BackpressureAwareSender::new(
        tcp_tx,
        BackpressurePolicy::Block,
    );
    let (shutdown_tx, shutdown_rx) = broadcast::channel::<()>(1);

    let listener_handle =
        tokio::spawn(async move { Listener::start(tcp_settings, tcp_sender, shutdown_rx).await });

    tokio::time::sleep(Duration::from_millis(100)).await;

    assert!(
        !listener_handle.is_finished(),
        "Listener should still be running before shutdown signal"
    );

    shutdown_tx.send(()).unwrap();

    let result = timeout(Duration::from_secs(1), listener_handle).await;
    assert!(
        result.is_ok(),
        "Listener should complete after shutdown signal"
    );
    let listener_result = result.unwrap();
    assert!(listener_result.is_ok(), "Listener task should not panic");
    let server_result = listener_result.unwrap();
    assert!(
        server_result.is_ok(),
        "Server should shutdown gracefully when shutdown receiver receives signal, got: {:?}",
        server_result
    );
}
