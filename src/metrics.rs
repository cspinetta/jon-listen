use std::sync::Arc;

/// Global handle for rendering Prometheus metrics
static METRICS_HANDLE: std::sync::OnceLock<Arc<metrics_exporter_prometheus::PrometheusHandle>> =
    std::sync::OnceLock::new();

/// Initialize metrics and set up Prometheus exporter
pub fn init(metrics_port: u16) -> Result<(), Box<dyn std::error::Error>> {
    // Set up Prometheus exporter
    // Build the recorder first to get the handle, then install it
    let recorder = metrics_exporter_prometheus::PrometheusBuilder::new().build();
    let handle = recorder.handle();

    // Store handle globally before installing (since install consumes the recorder)
    METRICS_HANDLE
        .set(Arc::new(handle))
        .map_err(|_| "Metrics handle already initialized")?;

    // Install the recorder as the global metrics recorder
    metrics::set_boxed_recorder(Box::new(recorder))
        .map_err(|e| format!("Failed to set global recorder: {}", e))?;

    // Metrics are registered automatically when first used
    // No need to describe them explicitly in metrics 0.16

    log::info!(
        "Metrics initialized. Prometheus metrics available at http://0.0.0.0:{}/metrics",
        metrics_port
    );
    Ok(())
}

/// Get the Prometheus handle for rendering metrics
pub fn get_handle() -> Option<Arc<metrics_exporter_prometheus::PrometheusHandle>> {
    METRICS_HANDLE.get().cloned()
}

/// Track message metrics (used across multiple components)
pub mod messages {
    use metrics::counter;

    pub fn received() {
        counter!("messages_received_total", 1);
    }

    pub fn written() {
        counter!("messages_written_total", 1);
    }

    pub fn dropped() {
        counter!("messages_dropped_total", 1);
    }
}
