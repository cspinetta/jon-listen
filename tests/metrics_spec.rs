use jon_listen::metrics;
use std::time::Duration;

#[test]
fn test_metrics_init_initializes_prometheus_recorder() {
    let result = metrics::init(9091);

    if result.is_ok() {
        let handle = metrics::get_handle();
        assert!(
            handle.is_some(),
            "Handle should be available after successful initialization"
        );

        metrics::messages::received();
        let output = handle.unwrap().render();
        assert!(
            !output.is_empty(),
            "Metrics output should not be empty after recording a metric"
        );
        assert!(
            output.contains("messages_received_total"),
            "Metrics output should contain the recorded metric"
        );
    } else {
        let handle = metrics::get_handle();
        assert!(
            handle.is_some(),
            "Handle should exist even if init failed (already initialized)"
        );
    }
}

#[test]
fn test_metrics_init_error_handling_for_double_initialization() {
    let first_result = metrics::init(9092);

    if first_result.is_ok() {
        let second_result = metrics::init(9092);
        assert!(
            second_result.is_err(),
            "Second initialization should fail with error"
        );
        let err_msg = format!("{}", second_result.unwrap_err());
        assert!(
            err_msg.contains("already initialized") || err_msg.contains("initialized"),
            "Error message should indicate double initialization, got: {}",
            err_msg
        );

        let handle = metrics::get_handle();
        assert!(
            handle.is_some(),
            "Handle should still be available after double initialization error"
        );
    } else {
        let second_result = metrics::init(9092);
        assert!(
            second_result.is_err(),
            "Double initialization should fail when metrics already initialized"
        );
    }
}

#[test]
fn test_metrics_get_handle_returns_handle_after_init() {
    let _ = metrics::init(9093);

    let handle = metrics::get_handle();
    assert!(
        handle.is_some(),
        "get_handle should return Some after initialization"
    );

    metrics::messages::received();
    let output = handle.unwrap().render();
    assert!(
        !output.is_empty(),
        "Metrics handle should produce non-empty output after recording metrics"
    );
    assert!(
        output.contains("messages_received_total"),
        "Metrics output should contain recorded metric"
    );
}

#[test]
fn test_metrics_messages_received_increments_counter() {
    let _ = metrics::init(9094);

    metrics::messages::received();
    metrics::messages::received();
    metrics::messages::received();

    let handle = metrics::get_handle();
    assert!(
        handle.is_some(),
        "Handle should be available for metrics verification"
    );
    let output = handle.unwrap().render();
    assert!(
        output.contains("messages_received_total"),
        "Metrics output should contain messages_received_total counter"
    );
}

#[test]
fn test_metrics_messages_written_increments_counter() {
    let _ = metrics::init(9095);

    metrics::messages::written();
    metrics::messages::written();

    let handle = metrics::get_handle();
    assert!(
        handle.is_some(),
        "Handle should be available for metrics verification"
    );
    let output = handle.unwrap().render();
    assert!(
        output.contains("messages_written_total"),
        "Metrics output should contain messages_written_total counter"
    );
}

#[test]
fn test_metrics_messages_dropped_increments_counter() {
    let _ = metrics::init(9096);

    metrics::messages::dropped();

    let handle = metrics::get_handle();
    assert!(
        handle.is_some(),
        "Handle should be available for metrics verification"
    );
    let output = handle.unwrap().render();
    assert!(
        output.contains("messages_dropped_total"),
        "Metrics output should contain messages_dropped_total counter"
    );
}

#[test]
fn test_metrics_tcp_connection_accepted_increments_counter() {
    let _ = metrics::init(9097);

    jon_listen::listener::metrics::tcp::connection_accepted();
    jon_listen::listener::metrics::tcp::connection_accepted();

    let handle = metrics::get_handle();
    assert!(
        handle.is_some(),
        "Handle should be available for metrics verification"
    );
    let output = handle.unwrap().render();
    assert!(
        output.contains("tcp_connections_total") || output.contains("tcp_connections"),
        "Metrics output should contain tcp_connections_total counter"
    );
}

#[test]
fn test_metrics_tcp_connection_active_sets_gauge() {
    let _ = metrics::init(9098);

    jon_listen::listener::metrics::tcp::connection_active(5);
    jon_listen::listener::metrics::tcp::connection_active(10);

    let handle = metrics::get_handle();
    assert!(
        handle.is_some(),
        "Handle should be available for metrics verification"
    );
    let output = handle.unwrap().render();
    assert!(
        output.contains("tcp_connections_active"),
        "Metrics output should contain tcp_connections_active gauge"
    );
}

#[test]
fn test_metrics_tcp_connection_rejected_increments_counter() {
    let _ = metrics::init(9099);

    jon_listen::listener::metrics::tcp::connection_rejected();

    let handle = metrics::get_handle();
    assert!(
        handle.is_some(),
        "Handle should be available for metrics verification"
    );
    let output = handle.unwrap().render();
    assert!(
        output.contains("tcp_connections_rejected"),
        "Metrics output should contain tcp_connections_rejected counter"
    );
}

#[test]
fn test_metrics_udp_datagram_received_increments_counter() {
    let _ = metrics::init(9100);

    jon_listen::listener::metrics::udp::datagram_received();
    jon_listen::listener::metrics::udp::datagram_received();

    let handle = metrics::get_handle();
    assert!(
        handle.is_some(),
        "Handle should be available for metrics verification"
    );
    let output = handle.unwrap().render();
    assert!(
        output.contains("udp_datagrams_received_total")
            || output.contains("udp_datagrams_received"),
        "Metrics output should contain udp_datagrams_received_total counter"
    );
}

#[test]
fn test_metrics_backpressure_event_increments_counter() {
    let _ = metrics::init(9101);

    jon_listen::writer::metrics::backpressure::event();
    jon_listen::writer::metrics::backpressure::event();

    let handle = metrics::get_handle();
    assert!(
        handle.is_some(),
        "Handle should be available for metrics verification"
    );
    let output = handle.unwrap().render();
    assert!(
        output.contains("backpressure_events_total"),
        "Metrics output should contain backpressure_events_total counter"
    );
}

#[test]
fn test_metrics_file_write_record_latency() {
    let _ = metrics::init(9102);

    let duration = Duration::from_millis(100);
    jon_listen::writer::metrics::file_write::record_latency(duration);
    jon_listen::writer::metrics::file_write::record_latency(Duration::from_millis(200));

    let handle = metrics::get_handle();
    assert!(
        handle.is_some(),
        "Handle should be available for metrics verification"
    );
    let output = handle.unwrap().render();
    assert!(
        output.contains("file_write_latency_seconds"),
        "Metrics output should contain file_write_latency_seconds histogram after recording latency values"
    );
}

#[test]
fn test_metrics_file_write_write_timer_measures_latency() {
    let _ = metrics::init(9103);

    let timer = jon_listen::writer::metrics::file_write::WriteTimer::start();
    std::thread::sleep(Duration::from_millis(50));
    timer.finish();

    let handle = metrics::get_handle();
    assert!(
        handle.is_some(),
        "Handle should be available for metrics verification"
    );
    let output = handle.unwrap().render();
    assert!(
        output.contains("file_write_latency_seconds"),
        "Metrics output should contain file_write_latency_seconds histogram after WriteTimer::finish"
    );
}

#[test]
fn test_metrics_rotation_event_increments_counter() {
    let _ = metrics::init(9104);

    jon_listen::writer::metrics::rotation::event();

    let handle = metrics::get_handle();
    assert!(
        handle.is_some(),
        "Handle should be available for metrics verification"
    );
    let output = handle.unwrap().render();
    assert!(
        output.contains("file_rotation_events_total"),
        "Metrics output should contain file_rotation_events_total counter"
    );
}

#[test]
fn test_metrics_rotation_error_increments_counter() {
    let _ = metrics::init(9105);

    jon_listen::writer::metrics::rotation::error();

    let handle = metrics::get_handle();
    assert!(
        handle.is_some(),
        "Handle should be available for metrics verification"
    );
    let output = handle.unwrap().render();
    assert!(
        output.contains("file_rotation_errors_total"),
        "Metrics output should contain file_rotation_errors_total counter"
    );
}

#[test]
fn test_metrics_handle_render_format() {
    let _ = metrics::init(9106);

    metrics::messages::received();
    jon_listen::listener::metrics::tcp::connection_accepted();
    jon_listen::writer::metrics::backpressure::event();

    let handle = metrics::get_handle();
    assert!(
        handle.is_some(),
        "Handle should be available for metrics verification"
    );
    let output = handle.unwrap().render();

    assert!(!output.is_empty(), "Metrics output should not be empty");
    assert!(
        output.contains("messages_received_total")
            || output.contains("tcp_connections_total")
            || output.contains("tcp_connections")
            || output.contains("backpressure_events_total"),
        "Metrics output should contain at least one of the recorded metric names"
    );
}
