use metrics::{counter, gauge};

/// Track TCP connection metrics
pub mod tcp {
    use super::*;

    pub fn connection_accepted() {
        counter!("tcp_connections_total", 1);
    }

    pub fn connection_active(count: usize) {
        gauge!("tcp_connections_active", count as f64);
    }

    pub fn connection_rejected() {
        counter!("tcp_connections_rejected", 1);
    }
}

/// Track UDP metrics
pub mod udp {
    use super::*;

    pub fn datagram_received() {
        counter!("udp_datagrams_received_total", 1);
    }
}
