use crate::messages::{PlayerIdentity, ReliableMessage, UnreliableMessage};
use crate::server::GameServer;
use anyhow::Result;
use crossworld_network::transport::{ReliableChannel, Transport, UnreliableChannel};
use crossworld_network::webtransport::WebTransportConnection;
use dashmap::DashMap;
use glam::Vec3;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use tokio::sync::Mutex;

/// Tracks the player associated with a connection
struct ConnectionContext {
    player_id: Option<String>,
}

impl GameServer {
    /// Handle a new WebTransport connection using the network crate's Transport abstraction
    pub async fn handle_transport(
        server: Arc<GameServer>,
        transport: WebTransportConnection,
        connections: Arc<DashMap<String, Arc<WebTransportConnection>>>,
    ) -> Result<()> {
        let remote_addr = transport
            .remote_address()
            .unwrap_or_else(|| "unknown".to_string());
        tracing::info!("Connection established from {}", remote_addr);

        // Generate a unique connection ID
        let connection_id = format!("conn_{}", uuid_v4());

        // Wrap transport in Arc for sharing across tasks
        let transport = Arc::new(transport);

        // Track player ID for this connection
        let context = Arc::new(Mutex::new(ConnectionContext { player_id: None }));

        // Spawn concurrent handlers for reliable and unreliable channels
        let reliable_server = server.clone();
        let reliable_transport = transport.clone();
        let reliable_context = context.clone();
        let reliable_conn_id = connection_id.clone();
        let reliable_connections = connections.clone();

        let unreliable_server = server.clone();
        let unreliable_transport = transport.clone();
        let unreliable_context = context.clone();

        tokio::select! {
            result = Self::handle_reliable_channel(
                reliable_server,
                reliable_transport,
                reliable_context,
                reliable_conn_id,
                reliable_connections,
            ) => {
                if let Err(e) = result {
                    tracing::error!("Reliable channel handler error: {}", e);
                }
            }
            result = Self::handle_unreliable_channel(
                unreliable_server,
                unreliable_transport,
                unreliable_context,
            ) => {
                if let Err(e) = result {
                    tracing::error!("Unreliable channel handler error: {}", e);
                }
            }
        }

        // Cleanup on disconnect
        let ctx = context.lock().await;
        if let Some(ref hex_id) = ctx.player_id {
            server.remove_player(hex_id);
            connections.remove(hex_id);
            tracing::info!("Player {} disconnected", hex_id);
        }

        tracing::info!("Connection closed: {}", remote_addr);
        Ok(())
    }

    /// Handle the reliable channel using the network crate's ReliableChannel trait
    async fn handle_reliable_channel(
        server: Arc<GameServer>,
        transport: Arc<WebTransportConnection>,
        context: Arc<Mutex<ConnectionContext>>,
        _connection_id: String,
        connections: Arc<DashMap<String, Arc<WebTransportConnection>>>,
    ) -> Result<()> {
        let reliable = transport.reliable();

        loop {
            // Receive next reliable message using the ReliableChannel trait
            let msg = match reliable.receive().await {
                Ok(msg) => msg,
                Err(e) => {
                    tracing::debug!("Reliable channel closed: {}", e);
                    break;
                }
            };

            server
                .metrics
                .messages_received
                .fetch_add(1, Ordering::Relaxed);

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

                        // Track this connection's player ID
                        {
                            let mut ctx = context.lock().await;
                            ctx.player_id = Some(hex_id.clone());
                        }

                        // Register connection for broadcasting
                        connections.insert(hex_id.clone(), transport.clone());

                        // Send success response using ReliableChannel
                        let response = ReliableMessage::ServerCommand {
                            command: "join_success".to_string(),
                            args: vec![hex_id],
                        };
                        if let Err(e) = reliable.send(response).await {
                            tracing::warn!("Failed to send join response: {}", e);
                        }
                        server.metrics.messages_sent.fetch_add(1, Ordering::Relaxed);
                    } else {
                        tracing::warn!("Server full, rejecting player {}", npub);

                        let response = ReliableMessage::Kick {
                            reason: "Server is full".to_string(),
                        };
                        if let Err(e) = reliable.send(response).await {
                            tracing::warn!("Failed to send kick response: {}", e);
                        }
                        server.metrics.messages_sent.fetch_add(1, Ordering::Relaxed);
                    }
                }

                ReliableMessage::Leave { npub } => {
                    let hex_id = Self::npub_to_hex(&npub)?;
                    server.remove_player(&hex_id);
                    connections.remove(&hex_id);

                    // Clear the player ID from context
                    {
                        let mut ctx = context.lock().await;
                        ctx.player_id = None;
                    }

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
                        continue;
                    }

                    tracing::info!("Chat from {}: {}", from, content);

                    // TODO: Broadcast to all players using their reliable channels
                }

                ReliableMessage::GameEvent { event_type, data } => {
                    tracing::debug!("Game event: {} ({} bytes)", event_type, data.len());
                    // Handle game-specific events
                }

                _ => {
                    tracing::warn!("Unexpected message type");
                }
            }
        }

        Ok(())
    }

    /// Handle the unreliable channel using the network crate's UnreliableChannel trait
    async fn handle_unreliable_channel(
        server: Arc<GameServer>,
        transport: Arc<WebTransportConnection>,
        context: Arc<Mutex<ConnectionContext>>,
    ) -> Result<()> {
        let unreliable = transport.unreliable();

        loop {
            // Receive next unreliable message using the UnreliableChannel trait
            let msg = match unreliable.receive().await {
                Ok(msg) => msg,
                Err(e) => {
                    tracing::debug!("Unreliable channel closed: {}", e);
                    break;
                }
            };

            server
                .metrics
                .messages_received
                .fetch_add(1, Ordering::Relaxed);

            // Process message
            match msg {
                UnreliableMessage::Position {
                    x,
                    y,
                    z,
                    rx,
                    ry,
                    rz,
                    rw,
                    seq,
                } => {
                    // Get the player ID from the connection context
                    let player_id = {
                        let ctx = context.lock().await;
                        ctx.player_id.clone()
                    };

                    if let Some(hex_id) = player_id {
                        let position = Vec3::new(x, y, z);
                        let rotation = glam::Quat::from_xyzw(rx, ry, rz, rw);

                        // Validate position if enabled
                        if server.config.position_validation
                            && !server.validate_position(&hex_id, position)
                        {
                            server.handle_position_violation(&hex_id);
                            continue;
                        }

                        // Update player position
                        server.update_player_position(&hex_id, position, rotation, Vec3::ZERO);

                        tracing::trace!(
                            "Position update from {}: ({}, {}, {}) seq={}",
                            hex_id,
                            x,
                            y,
                            z,
                            seq
                        );
                    } else {
                        tracing::trace!(
                            "Position update from unknown player: ({}, {}, {}) seq={}",
                            x,
                            y,
                            z,
                            seq
                        );
                    }
                }

                UnreliableMessage::Ping { timestamp } => {
                    // Send pong response using UnreliableChannel
                    let pong = UnreliableMessage::Pong { timestamp };
                    if let Err(e) = unreliable.send(pong).await {
                        tracing::warn!("Failed to send pong: {}", e);
                    }
                    server.metrics.messages_sent.fetch_add(1, Ordering::Relaxed);
                }

                UnreliableMessage::Pong { timestamp } => {
                    // Calculate latency (timestamp is server-sent time in ms)
                    let now_ms = std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_millis() as u64;
                    let latency = now_ms.saturating_sub(timestamp);
                    tracing::trace!("Pong received, latency: {}ms", latency);

                    // Update RTT in connection manager
                    transport.manager().update_rtt(latency);
                }

                UnreliableMessage::Batch { .. } => {
                    // Server doesn't process incoming batches from clients
                    tracing::warn!("Received unexpected batch message from client");
                }
            }
        }

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

/// Generate a simple UUID v4-like string
fn uuid_v4() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    format!("{:016x}{:08x}", now.as_nanos(), rand_u32())
}

/// Simple pseudo-random u32 for UUID generation
fn rand_u32() -> u32 {
    use std::time::SystemTime;
    let nanos = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default()
        .subsec_nanos();
    nanos.wrapping_mul(1103515245).wrapping_add(12345)
}
