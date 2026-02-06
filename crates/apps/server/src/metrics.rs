use crate::server::ServerMetrics;
use std::sync::atomic::Ordering;
use std::sync::Arc;

impl ServerMetrics {
    /// Get Prometheus-compatible metrics as a string
    #[allow(dead_code)]
    pub fn to_prometheus(&self) -> String {
        format!(
            "# HELP connected_players Number of connected players\n\
             # TYPE connected_players gauge\n\
             connected_players {}\n\
             \n\
             # HELP messages_sent Total messages sent\n\
             # TYPE messages_sent counter\n\
             messages_sent {}\n\
             \n\
             # HELP messages_received Total messages received\n\
             # TYPE messages_received counter\n\
             messages_received {}\n\
             \n\
             # HELP bytes_sent Total bytes sent\n\
             # TYPE bytes_sent counter\n\
             bytes_sent {}\n\
             \n\
             # HELP bytes_received Total bytes received\n\
             # TYPE bytes_received counter\n\
             bytes_received {}\n\
             \n\
             # HELP position_violations Total position validation violations\n\
             # TYPE position_violations counter\n\
             position_violations {}\n",
            self.connected_players.load(Ordering::Relaxed),
            self.messages_sent.load(Ordering::Relaxed),
            self.messages_received.load(Ordering::Relaxed),
            self.bytes_sent.load(Ordering::Relaxed),
            self.bytes_received.load(Ordering::Relaxed),
            self.position_violations.load(Ordering::Relaxed),
        )
    }

    /// Print metrics to console
    pub fn print_stats(&self) {
        tracing::info!(
            "Players: {} | Msgs Sent: {} | Msgs Recv: {} | Bytes Sent: {} | Bytes Recv: {} | Violations: {}",
            self.connected_players.load(Ordering::Relaxed),
            self.messages_sent.load(Ordering::Relaxed),
            self.messages_received.load(Ordering::Relaxed),
            self.bytes_sent.load(Ordering::Relaxed),
            self.bytes_received.load(Ordering::Relaxed),
            self.position_violations.load(Ordering::Relaxed),
        );
    }
}

/// Start metrics reporting task
pub async fn start_metrics_reporter(metrics: Arc<ServerMetrics>) {
    let mut interval = tokio::time::interval(std::time::Duration::from_secs(30));

    loop {
        interval.tick().await;
        metrics.print_stats();
    }
}
