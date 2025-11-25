use crate::messages::{PlayerIdentity, ReliableMessage, UnreliableMessage};
use crate::server::GameServer;
use anyhow::{Context, Result};
use glam::Vec3;
use std::sync::Arc;
use std::time::Instant;
use wtransport::endpoint::IncomingSession;
use wtransport::{Connection, RecvStream, SendStream};

impl GameServer {
    /// Handle a new WebTransport connection
    pub async fn handle_connection(
        server: Arc<GameServer>,
        session: IncomingSession,
    ) -> Result<()> {
        // Accept the session request
        let session_request = session.await?;
        tracing::info!("Accepting new connection request");

        let connection = session_request
            .accept()
            .await
            .context("Failed to accept session")?;

        tracing::info!("Connection established");

        // Spawn concurrent handlers for different stream types
        let conn = Arc::new(connection);

        tokio::select! {
            result = Self::handle_reliable_streams(server.clone(), conn.clone()) => {
                if let Err(e) = result {
                    tracing::error!("Reliable stream handler error: {}", e);
                }
            }
            result = Self::handle_datagrams(server.clone(), conn.clone()) => {
                if let Err(e) = result {
                    tracing::error!("Datagram handler error: {}", e);
                }
            }
        }

        tracing::info!("Connection closed: {:?}", conn.remote_address());
        Ok(())
    }

    /// Handle reliable bi-directional streams
    async fn handle_reliable_streams(
        server: Arc<GameServer>,
        connection: Arc<Connection>,
    ) -> Result<()> {
        let player_id: Option<String> = None;

        loop {
            // Accept next bidirectional stream
            let (send_stream, recv_stream) = match connection.accept_bi().await {
                Ok(streams) => streams,
                Err(e) => {
                    tracing::warn!("Failed to accept bi stream: {}", e);
                    break;
                }
            };

            let server = server.clone();
            let pid = player_id.clone();

            // Spawn handler for this stream
            tokio::spawn(async move {
                if let Err(e) =
                    Self::handle_reliable_message(server, send_stream, recv_stream, pid).await
                {
                    tracing::error!("Error handling reliable message: {}", e);
                }
            });
        }

        // Cleanup on disconnect
        if let Some(hex_id) = player_id {
            server.remove_player(&hex_id);
            tracing::info!("Player {} disconnected", hex_id);
        }

        Ok(())
    }

    /// Handle a single reliable message
    async fn handle_reliable_message(
        server: Arc<GameServer>,
        mut send_stream: SendStream,
        mut recv_stream: RecvStream,
        _player_id: Option<String>,
    ) -> Result<()> {
        // Read message data
        let data = Self::read_stream(&mut recv_stream).await?;
        server
            .metrics
            .bytes_received
            .fetch_add(data.len() as u64, std::sync::atomic::Ordering::Relaxed);
        server
            .metrics
            .messages_received
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);

        // Deserialize message
        let msg: ReliableMessage =
            bincode::deserialize(&data).context("Failed to deserialize reliable message")?;

        tracing::debug!("Received reliable message: {:?}", msg);

        // Process message
        match msg {
            ReliableMessage::Join {
                npub,
                display_name,
                avatar_url,
                position,
            } => {
                let hex_id = Self::npub_to_hex(&npub)?;
                let pos = Vec3::from_array(position);

                let identity = PlayerIdentity {
                    npub: npub.clone(),
                    hex_id: hex_id.clone(),
                    display_name,
                    avatar_url,
                };

                if server.add_player(hex_id.clone(), identity, pos) {
                    tracing::info!("Player {} joined at {:?}", npub, pos);

                    // Send success response
                    let response = ReliableMessage::ServerCommand {
                        command: "join_success".to_string(),
                        args: vec![hex_id],
                    };
                    Self::send_reliable(&mut send_stream, &response).await?;
                } else {
                    tracing::warn!("Server full, rejecting player {}", npub);

                    let response = ReliableMessage::Kick {
                        reason: "Server is full".to_string(),
                    };
                    Self::send_reliable(&mut send_stream, &response).await?;
                }
            }

            ReliableMessage::Leave { npub } => {
                let hex_id = Self::npub_to_hex(&npub)?;
                server.remove_player(&hex_id);
                tracing::info!("Player {} left", npub);
            }

            ReliableMessage::ChatMessage {
                from,
                content,
                timestamp: _,
            } => {
                // Validate message
                if content.len() > 1000 {
                    tracing::warn!("Chat message too long from {}", from);
                    return Ok(());
                }

                tracing::info!("Chat from {}: {}", from, content);

                // Broadcast to all players (in a real implementation)
                // For now, just log it
            }

            ReliableMessage::GameEvent { event_type, data } => {
                tracing::debug!("Game event: {} ({} bytes)", event_type, data.len());
                // Handle game-specific events
            }

            _ => {
                tracing::warn!("Unexpected message type");
            }
        }

        Ok(())
    }

    /// Handle unreliable datagrams (position updates)
    async fn handle_datagrams(server: Arc<GameServer>, connection: Arc<Connection>) -> Result<()> {
        loop {
            // Receive datagram
            let data = match connection.receive_datagram().await {
                Ok(data) => data,
                Err(e) => {
                    tracing::warn!("Datagram receive error: {}", e);
                    break;
                }
            };

            let n = data.len();

            server
                .metrics
                .bytes_received
                .fetch_add(n as u64, std::sync::atomic::Ordering::Relaxed);
            server
                .metrics
                .messages_received
                .fetch_add(1, std::sync::atomic::Ordering::Relaxed);

            // Deserialize message
            let msg: UnreliableMessage = match bincode::deserialize(&data) {
                Ok(m) => m,
                Err(e) => {
                    tracing::warn!("Failed to deserialize datagram: {}", e);
                    continue;
                }
            };

            // Process message
            match msg {
                UnreliableMessage::Position {
                    x,
                    y,
                    z,
                    rx: _,
                    ry: _,
                    rz: _,
                    rw: _,
                    seq,
                } => {
                    // For now, we need to know which player this is from
                    // In a real implementation, you'd track connection -> player mapping
                    // For simplicity, we'll skip this for now
                    tracing::trace!("Position update: ({}, {}, {}) seq={}", x, y, z, seq);
                }

                UnreliableMessage::Ping { timestamp } => {
                    // Send pong response
                    let pong = UnreliableMessage::Pong { timestamp };
                    if let Ok(pong_data) = bincode::serialize(&pong) {
                        let _ = connection.send_datagram(&pong_data);
                    }
                }

                UnreliableMessage::Pong { timestamp } => {
                    let latency = Instant::now()
                        .duration_since(
                            Instant::now() - std::time::Duration::from_millis(timestamp),
                        )
                        .as_millis();
                    tracing::trace!("Pong received, latency: {}ms", latency);
                }

                _ => {}
            }
        }

        Ok(())
    }

    /// Read entire stream into buffer
    async fn read_stream(stream: &mut RecvStream) -> Result<Vec<u8>> {
        let mut data = Vec::new();
        let mut buf = [0u8; 4096];

        loop {
            match stream.read(&mut buf).await? {
                Some(n) => {
                    data.extend_from_slice(&buf[..n]);
                }
                None => break,
            }
        }

        Ok(data)
    }

    /// Send reliable message
    async fn send_reliable(stream: &mut SendStream, msg: &ReliableMessage) -> Result<()> {
        let data = bincode::serialize(msg)?;
        stream.write_all(&data).await?;
        Ok(())
    }

    /// Convert npub to hex ID (placeholder - needs nostr-sdk integration)
    fn npub_to_hex(npub: &str) -> Result<String> {
        // TODO: Proper npub decoding using nostr-sdk
        // For now, just use a simple hash
        Ok(format!(
            "{:x}",
            npub.bytes().fold(0u64, |acc, b| acc.wrapping_add(b as u64))
        ))
    }
}
