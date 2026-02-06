//! WebTransport implementation of the transport traits.
//!
//! This module provides a concrete implementation using the `wtransport` crate.
//! Enable with the `webtransport` feature.

use crate::connection::{ConnectionManager, ConnectionState, ReconnectConfig};
use crate::error::{NetworkError, NetworkResult};
use crate::message::{self, ReliableMessage, UnreliableMessage};
use crate::transport::{
    ReliableChannel, Transport, TransportConfig, TransportConnector, TransportListener,
    UnreliableChannel,
};
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use tokio::sync::Mutex;
use wtransport::Connection;

/// WebTransport-based transport implementation.
pub struct WebTransportConnection {
    connection: Arc<Connection>,
    reliable: WebTransportReliable,
    unreliable: WebTransportUnreliable,
    manager: ConnectionManager,
}

impl WebTransportConnection {
    /// Create from an existing wtransport Connection.
    pub fn new(connection: Connection, manager: ConnectionManager) -> Self {
        let connection = Arc::new(connection);

        let remote_addr = connection.remote_address().to_string();

        manager.set_connected(remote_addr);

        Self {
            connection: connection.clone(),
            reliable: WebTransportReliable {
                connection: connection.clone(),
                manager: manager.clone(),
            },
            unreliable: WebTransportUnreliable {
                connection: connection.clone(),
                manager: manager.clone(),
            },
            manager,
        }
    }

    /// Get the underlying wtransport Connection.
    pub fn inner(&self) -> &Arc<Connection> {
        &self.connection
    }

    /// Get the connection manager.
    pub fn manager(&self) -> &ConnectionManager {
        &self.manager
    }
}

impl Transport for WebTransportConnection {
    type Reliable = WebTransportReliable;
    type Unreliable = WebTransportUnreliable;

    fn reliable(&self) -> &Self::Reliable {
        &self.reliable
    }

    fn unreliable(&self) -> &Self::Unreliable {
        &self.unreliable
    }

    fn is_connected(&self) -> bool {
        self.manager.state() == ConnectionState::Connected
    }

    fn remote_address(&self) -> Option<String> {
        Some(self.connection.remote_address().to_string())
    }

    fn close(&self) -> Pin<Box<dyn Future<Output = NetworkResult<()>> + Send + '_>> {
        Box::pin(async move {
            self.manager.set_disconnected();
            // wtransport connections close when dropped
            Ok(())
        })
    }
}

/// Reliable channel using WebTransport bidirectional streams.
pub struct WebTransportReliable {
    connection: Arc<Connection>,
    manager: ConnectionManager,
}

impl ReliableChannel for WebTransportReliable {
    fn send(
        &self,
        message: ReliableMessage,
    ) -> Pin<Box<dyn Future<Output = NetworkResult<()>> + Send + '_>> {
        Box::pin(async move {
            // Serialize message
            let data = message::serialize(&message)?;

            // Open a new bidirectional stream for this message
            // open_bi() returns an OpeningBiStream which needs to be awaited again
            let opening = self
                .connection
                .open_bi()
                .await
                .map_err(|e| NetworkError::SendFailed(e.to_string()))?;

            let (mut send_stream, _recv_stream) = opening
                .await
                .map_err(|e| NetworkError::SendFailed(e.to_string()))?;

            // Write data
            send_stream
                .write_all(&data)
                .await
                .map_err(|e| NetworkError::SendFailed(e.to_string()))?;

            // Finish the send side
            send_stream
                .finish()
                .await
                .map_err(|e| NetworkError::SendFailed(e.to_string()))?;

            self.manager.record_sent(data.len() as u64);
            Ok(())
        })
    }

    fn receive(&self) -> Pin<Box<dyn Future<Output = NetworkResult<ReliableMessage>> + Send + '_>> {
        Box::pin(async move {
            // Accept a bidirectional stream
            let (_send_stream, mut recv_stream) = self
                .connection
                .accept_bi()
                .await
                .map_err(|e| NetworkError::ReceiveFailed(e.to_string()))?;

            // Read all data
            let mut data = Vec::new();
            let mut buf = [0u8; 4096];
            while let Some(n) = recv_stream
                .read(&mut buf)
                .await
                .map_err(|e| NetworkError::ReceiveFailed(e.to_string()))?
            {
                data.extend_from_slice(&buf[..n]);
            }

            self.manager.record_received(data.len() as u64);

            // Deserialize
            message::deserialize(&data)
        })
    }

    fn close(&self) -> Pin<Box<dyn Future<Output = NetworkResult<()>> + Send + '_>> {
        Box::pin(async move { Ok(()) })
    }
}

/// Unreliable channel using WebTransport datagrams.
pub struct WebTransportUnreliable {
    connection: Arc<Connection>,
    manager: ConnectionManager,
}

impl UnreliableChannel for WebTransportUnreliable {
    fn send(
        &self,
        message: UnreliableMessage,
    ) -> Pin<Box<dyn Future<Output = NetworkResult<()>> + Send + '_>> {
        Box::pin(async move {
            let data = message::serialize(&message)?;

            self.connection
                .send_datagram(&data)
                .map_err(|e| NetworkError::SendFailed(e.to_string()))?;

            self.manager.record_sent(data.len() as u64);
            Ok(())
        })
    }

    fn receive(
        &self,
    ) -> Pin<Box<dyn Future<Output = NetworkResult<UnreliableMessage>> + Send + '_>> {
        Box::pin(async move {
            let data = self
                .connection
                .receive_datagram()
                .await
                .map_err(|e| NetworkError::ReceiveFailed(e.to_string()))?;

            self.manager.record_received(data.len() as u64);
            message::deserialize(&data)
        })
    }

    fn max_datagram_size(&self) -> usize {
        self.connection.max_datagram_size().unwrap_or(1200)
    }
}

/// Server-side WebTransport listener.
pub struct WebTransportListener {
    endpoint: wtransport::Endpoint<wtransport::endpoint::endpoint_side::Server>,
    reconnect_config: ReconnectConfig,
}

impl WebTransportListener {
    /// Create a new WebTransport server listener.
    pub async fn bind(config: TransportConfig) -> NetworkResult<Self> {
        let cert_path = config
            .cert_path
            .ok_or_else(|| NetworkError::Tls("Certificate path required".to_string()))?;

        let key_path = config
            .key_path
            .ok_or_else(|| NetworkError::Tls("Key path required".to_string()))?;

        let identity = wtransport::Identity::load_pemfiles(&cert_path, &key_path)
            .await
            .map_err(|e| NetworkError::Tls(e.to_string()))?;

        let server_config =
            wtransport::ServerConfig::builder()
                .with_bind_address(config.address.parse().map_err(
                    |e: std::net::AddrParseError| NetworkError::ConnectionFailed(e.to_string()),
                )?)
                .with_identity(identity)
                .build();

        let endpoint = wtransport::Endpoint::server(server_config)
            .map_err(|e| NetworkError::Transport(e.to_string()))?;

        Ok(Self {
            endpoint,
            reconnect_config: ReconnectConfig::disabled(), // Server doesn't reconnect
        })
    }

    /// Accept the next incoming session and return it for custom handling.
    ///
    /// This returns the raw IncomingSession which can be awaited and accepted.
    pub async fn accept_session(&self) -> wtransport::endpoint::IncomingSession {
        self.endpoint.accept().await
    }
}

impl TransportListener for WebTransportListener {
    type Transport = WebTransportConnection;

    fn accept(&self) -> Pin<Box<dyn Future<Output = NetworkResult<Self::Transport>> + Send + '_>> {
        Box::pin(async move {
            let session = self.accept_session().await;

            let session_request = session
                .await
                .map_err(|e| NetworkError::ConnectionFailed(e.to_string()))?;

            let connection = session_request
                .accept()
                .await
                .map_err(|e| NetworkError::ConnectionFailed(e.to_string()))?;

            let manager = ConnectionManager::new(self.reconnect_config.clone());
            Ok(WebTransportConnection::new(connection, manager))
        })
    }

    fn local_address(&self) -> NetworkResult<String> {
        self.endpoint
            .local_addr()
            .map(|a| a.to_string())
            .map_err(|e| NetworkError::Transport(e.to_string()))
    }
}

/// Client-side WebTransport connector.
pub struct WebTransportConnector {
    reconnect_config: ReconnectConfig,
}

impl WebTransportConnector {
    /// Create a new WebTransport connector.
    pub fn new(reconnect_config: ReconnectConfig) -> Self {
        Self { reconnect_config }
    }
}

impl Default for WebTransportConnector {
    fn default() -> Self {
        Self::new(ReconnectConfig::default())
    }
}

impl TransportConnector for WebTransportConnector {
    type Transport = WebTransportConnection;

    fn connect(
        &self,
        config: TransportConfig,
    ) -> Pin<Box<dyn Future<Output = NetworkResult<Self::Transport>> + Send + '_>> {
        let reconnect_config = self.reconnect_config.clone();

        Box::pin(async move {
            let manager = ConnectionManager::new(reconnect_config);
            manager.set_connecting();

            // Build client configuration
            let client_config = if config.allow_insecure {
                wtransport::ClientConfig::builder()
                    .with_bind_default()
                    .with_no_cert_validation()
                    .build()
            } else {
                wtransport::ClientConfig::builder()
                    .with_bind_default()
                    .with_native_certs()
                    .build()
            };

            let endpoint = wtransport::Endpoint::client(client_config)
                .map_err(|e| NetworkError::Transport(e.to_string()))?;

            let url = if config.address.starts_with("https://") {
                config.address.clone()
            } else {
                format!("https://{}", config.address)
            };

            let connection = tokio::time::timeout(
                std::time::Duration::from_millis(config.connect_timeout_ms),
                endpoint.connect(&url),
            )
            .await
            .map_err(|_| NetworkError::Timeout)?
            .map_err(|e| NetworkError::ConnectionFailed(e.to_string()))?;

            Ok(WebTransportConnection::new(connection, manager))
        })
    }
}

/// Reconnecting WebTransport client wrapper.
///
/// Automatically handles reconnection with exponential backoff.
pub struct ReconnectingTransport {
    connector: WebTransportConnector,
    config: TransportConfig,
    transport: Arc<Mutex<Option<WebTransportConnection>>>,
    manager: ConnectionManager,
}

impl ReconnectingTransport {
    /// Create a new reconnecting transport.
    pub fn new(config: TransportConfig, reconnect_config: ReconnectConfig) -> Self {
        Self {
            connector: WebTransportConnector::new(reconnect_config.clone()),
            config,
            transport: Arc::new(Mutex::new(None)),
            manager: ConnectionManager::new(reconnect_config),
        }
    }

    /// Connect (or reconnect) to the server.
    pub async fn connect(&self) -> NetworkResult<()> {
        let mut transport_guard = self.transport.lock().await;

        if let Some(ref t) = *transport_guard {
            if t.is_connected() {
                return Ok(());
            }
        }

        self.manager.set_connecting();

        let transport = self.connector.connect(self.config.clone()).await?;
        *transport_guard = Some(transport);

        if let Some(ref t) = *transport_guard {
            if let Some(addr) = t.remote_address() {
                self.manager.set_connected(addr);
            }
        }

        Ok(())
    }

    /// Attempt reconnection with backoff.
    pub async fn reconnect(&self) -> NetworkResult<()> {
        while self.manager.should_reconnect() {
            self.manager.set_reconnecting();

            let delay = self.manager.next_reconnect_delay();
            tracing::info!(
                "Reconnection attempt {} in {:?}",
                self.manager.info().reconnect_attempts,
                delay
            );

            tokio::time::sleep(delay).await;

            match self.connect().await {
                Ok(()) => {
                    tracing::info!("Reconnected successfully");
                    return Ok(());
                }
                Err(e) => {
                    tracing::warn!("Reconnection failed: {}", e);
                }
            }
        }

        self.manager.set_failed();
        Err(NetworkError::MaxReconnectsExceeded)
    }

    /// Get the connection manager.
    pub fn manager(&self) -> &ConnectionManager {
        &self.manager
    }

    /// Disconnect and clear the transport.
    pub async fn disconnect(&self) {
        let mut transport_guard = self.transport.lock().await;
        if let Some(t) = transport_guard.take() {
            let _ = t.close().await;
        }
        self.manager.set_disconnected();
    }

    /// Execute a function with access to the transport if connected.
    pub async fn with_transport<F, R>(&self, f: F) -> Option<R>
    where
        F: FnOnce(&WebTransportConnection) -> R,
    {
        let guard = self.transport.lock().await;
        guard.as_ref().map(f)
    }
}
