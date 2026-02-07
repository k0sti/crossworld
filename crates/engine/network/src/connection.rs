//! Connection state management.
//!
//! This module provides abstractions for tracking connection lifecycle,
//! handling reconnection, and managing connection events.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::watch;

/// Connection states.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectionState {
    /// Not connected.
    Disconnected,
    /// Attempting to connect.
    Connecting,
    /// Connection established.
    Connected,
    /// Attempting to reconnect after disconnection.
    Reconnecting,
    /// Connection permanently failed.
    Failed,
}

impl std::fmt::Display for ConnectionState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Disconnected => write!(f, "Disconnected"),
            Self::Connecting => write!(f, "Connecting"),
            Self::Connected => write!(f, "Connected"),
            Self::Reconnecting => write!(f, "Reconnecting"),
            Self::Failed => write!(f, "Failed"),
        }
    }
}

/// Events emitted by the connection manager.
#[derive(Debug, Clone)]
pub enum ConnectionEvent {
    /// Connection established.
    Connected { remote_address: String },
    /// Connection lost.
    Disconnected { reason: String },
    /// Reconnection attempt started.
    ReconnectStarted { attempt: u32, max_attempts: u32 },
    /// Reconnection attempt failed.
    ReconnectFailed { attempt: u32, error: String },
    /// All reconnection attempts exhausted.
    ReconnectExhausted,
    /// Connection error occurred.
    Error { error: String },
}

/// Information about the current connection.
#[derive(Debug, Clone)]
pub struct ConnectionInfo {
    /// Current connection state.
    pub state: ConnectionState,
    /// Remote address (if connected).
    pub remote_address: Option<String>,
    /// Time when connection was established.
    pub connected_at: Option<Instant>,
    /// Number of reconnection attempts made.
    pub reconnect_attempts: u32,
    /// Round-trip time in milliseconds.
    pub rtt_ms: Option<u64>,
    /// Bytes sent.
    pub bytes_sent: u64,
    /// Bytes received.
    pub bytes_received: u64,
}

/// Reconnection strategy configuration.
#[derive(Debug, Clone)]
pub struct ReconnectConfig {
    /// Enable automatic reconnection.
    pub enabled: bool,
    /// Maximum number of reconnection attempts.
    pub max_attempts: u32,
    /// Initial delay between attempts.
    pub initial_delay: Duration,
    /// Maximum delay between attempts.
    pub max_delay: Duration,
    /// Backoff multiplier.
    pub backoff_multiplier: f64,
    /// Add jitter to delays.
    pub jitter: bool,
}

impl Default for ReconnectConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            max_attempts: 5,
            initial_delay: Duration::from_millis(1000),
            max_delay: Duration::from_secs(30),
            backoff_multiplier: 2.0,
            jitter: true,
        }
    }
}

impl ReconnectConfig {
    /// Disable automatic reconnection.
    pub fn disabled() -> Self {
        Self {
            enabled: false,
            ..Default::default()
        }
    }

    /// Calculate delay for a given attempt number.
    pub fn delay_for_attempt(&self, attempt: u32) -> Duration {
        let base_delay = self.initial_delay.as_millis() as f64
            * self
                .backoff_multiplier
                .powi(attempt.saturating_sub(1) as i32);

        let delay_ms = base_delay.min(self.max_delay.as_millis() as f64);

        let final_delay_ms = if self.jitter {
            // Add up to 25% jitter
            let jitter = delay_ms * 0.25 * rand_jitter();
            delay_ms + jitter
        } else {
            delay_ms
        };

        Duration::from_millis(final_delay_ms as u64)
    }
}

/// Simple pseudo-random jitter (0.0 to 1.0) without external dependency.
fn rand_jitter() -> f64 {
    use std::time::SystemTime;
    let nanos = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default()
        .subsec_nanos();
    (nanos % 1000) as f64 / 1000.0
}

/// Manages connection state and reconnection logic.
pub struct ConnectionManager {
    state: Arc<watch::Sender<ConnectionState>>,
    state_rx: watch::Receiver<ConnectionState>,
    reconnect_config: ReconnectConfig,
    stats: Arc<ConnectionStats>,
    remote_address: Arc<parking_lot::RwLock<Option<String>>>,
    connected_at: Arc<parking_lot::RwLock<Option<Instant>>>,
    reconnect_attempts: Arc<AtomicU64>,
}

struct ConnectionStats {
    bytes_sent: AtomicU64,
    bytes_received: AtomicU64,
    rtt_ms: AtomicU64,
}

impl ConnectionManager {
    /// Create a new connection manager.
    pub fn new(reconnect_config: ReconnectConfig) -> Self {
        let (state, state_rx) = watch::channel(ConnectionState::Disconnected);
        Self {
            state: Arc::new(state),
            state_rx,
            reconnect_config,
            stats: Arc::new(ConnectionStats {
                bytes_sent: AtomicU64::new(0),
                bytes_received: AtomicU64::new(0),
                rtt_ms: AtomicU64::new(0),
            }),
            remote_address: Arc::new(parking_lot::RwLock::new(None)),
            connected_at: Arc::new(parking_lot::RwLock::new(None)),
            reconnect_attempts: Arc::new(AtomicU64::new(0)),
        }
    }

    /// Get the current connection state.
    pub fn state(&self) -> ConnectionState {
        *self.state_rx.borrow()
    }

    /// Subscribe to state changes.
    pub fn subscribe(&self) -> watch::Receiver<ConnectionState> {
        self.state_rx.clone()
    }

    /// Get connection info.
    pub fn info(&self) -> ConnectionInfo {
        ConnectionInfo {
            state: self.state(),
            remote_address: self.remote_address.read().clone(),
            connected_at: *self.connected_at.read(),
            reconnect_attempts: self.reconnect_attempts.load(Ordering::Relaxed) as u32,
            rtt_ms: {
                let rtt = self.stats.rtt_ms.load(Ordering::Relaxed);
                if rtt > 0 {
                    Some(rtt)
                } else {
                    None
                }
            },
            bytes_sent: self.stats.bytes_sent.load(Ordering::Relaxed),
            bytes_received: self.stats.bytes_received.load(Ordering::Relaxed),
        }
    }

    /// Transition to connecting state.
    pub fn set_connecting(&self) {
        let _ = self.state.send(ConnectionState::Connecting);
    }

    /// Transition to connected state.
    pub fn set_connected(&self, remote_address: String) {
        *self.remote_address.write() = Some(remote_address);
        *self.connected_at.write() = Some(Instant::now());
        self.reconnect_attempts.store(0, Ordering::Relaxed);
        let _ = self.state.send(ConnectionState::Connected);
    }

    /// Transition to disconnected state.
    pub fn set_disconnected(&self) {
        *self.remote_address.write() = None;
        *self.connected_at.write() = None;
        let _ = self.state.send(ConnectionState::Disconnected);
    }

    /// Transition to reconnecting state.
    pub fn set_reconnecting(&self) {
        self.reconnect_attempts.fetch_add(1, Ordering::Relaxed);
        let _ = self.state.send(ConnectionState::Reconnecting);
    }

    /// Transition to failed state.
    pub fn set_failed(&self) {
        let _ = self.state.send(ConnectionState::Failed);
    }

    /// Record bytes sent.
    pub fn record_sent(&self, bytes: u64) {
        self.stats.bytes_sent.fetch_add(bytes, Ordering::Relaxed);
    }

    /// Record bytes received.
    pub fn record_received(&self, bytes: u64) {
        self.stats
            .bytes_received
            .fetch_add(bytes, Ordering::Relaxed);
    }

    /// Update RTT measurement.
    pub fn update_rtt(&self, rtt_ms: u64) {
        self.stats.rtt_ms.store(rtt_ms, Ordering::Relaxed);
    }

    /// Check if reconnection should be attempted.
    pub fn should_reconnect(&self) -> bool {
        if !self.reconnect_config.enabled {
            return false;
        }

        let attempts = self.reconnect_attempts.load(Ordering::Relaxed) as u32;
        attempts < self.reconnect_config.max_attempts
    }

    /// Get the delay before the next reconnection attempt.
    pub fn next_reconnect_delay(&self) -> Duration {
        let attempts = self.reconnect_attempts.load(Ordering::Relaxed) as u32;
        self.reconnect_config.delay_for_attempt(attempts)
    }

    /// Get reconnection configuration.
    pub fn reconnect_config(&self) -> &ReconnectConfig {
        &self.reconnect_config
    }

    /// Reset reconnection attempt counter.
    pub fn reset_reconnect_attempts(&self) {
        self.reconnect_attempts.store(0, Ordering::Relaxed);
    }
}

impl Clone for ConnectionManager {
    fn clone(&self) -> Self {
        Self {
            state: self.state.clone(),
            state_rx: self.state_rx.clone(),
            reconnect_config: self.reconnect_config.clone(),
            stats: self.stats.clone(),
            remote_address: self.remote_address.clone(),
            connected_at: self.connected_at.clone(),
            reconnect_attempts: self.reconnect_attempts.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_reconnect_delay_calculation() {
        let config = ReconnectConfig {
            enabled: true,
            max_attempts: 5,
            initial_delay: Duration::from_millis(100),
            max_delay: Duration::from_secs(10),
            backoff_multiplier: 2.0,
            jitter: false,
        };

        // First attempt: 100ms
        assert_eq!(config.delay_for_attempt(1), Duration::from_millis(100));

        // Second attempt: 200ms
        assert_eq!(config.delay_for_attempt(2), Duration::from_millis(200));

        // Third attempt: 400ms
        assert_eq!(config.delay_for_attempt(3), Duration::from_millis(400));
    }

    #[test]
    fn test_connection_state_transitions() {
        let manager = ConnectionManager::new(ReconnectConfig::default());

        assert_eq!(manager.state(), ConnectionState::Disconnected);

        manager.set_connecting();
        assert_eq!(manager.state(), ConnectionState::Connecting);

        manager.set_connected("127.0.0.1:4433".to_string());
        assert_eq!(manager.state(), ConnectionState::Connected);

        let info = manager.info();
        assert_eq!(info.remote_address, Some("127.0.0.1:4433".to_string()));

        manager.set_disconnected();
        assert_eq!(manager.state(), ConnectionState::Disconnected);
    }
}
