use crate::messages::{CompactPosition, UnreliableMessage};
use crate::server::GameServer;
use anyhow::Result;
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use wtransport::Connection;

impl GameServer {
    /// Start position broadcasting loop
    pub async fn start_position_broadcaster(
        server: Arc<GameServer>,
        connections: Arc<dashmap::DashMap<String, Arc<Connection>>>,
    ) {
        let mut interval = tokio::time::interval(Duration::from_millis(
            1000 / server.config.position_broadcast_rate as u64,
        ));

        loop {
            interval.tick().await;

            if let Err(e) =
                Self::broadcast_positions_once(server.clone(), connections.clone()).await
            {
                tracing::error!("Error broadcasting positions: {}", e);
            }
        }
    }

    /// Broadcast positions once
    async fn broadcast_positions_once(
        server: Arc<GameServer>,
        connections: Arc<dashmap::DashMap<String, Arc<Connection>>>,
    ) -> Result<()> {
        // Collect all player positions
        let positions: Vec<CompactPosition> = server
            .players
            .iter()
            .map(|entry| {
                let player = entry.value();
                CompactPosition {
                    id: entry.key().clone(),
                    pos: [
                        player.state.position.x,
                        player.state.position.y,
                        player.state.position.z,
                    ],
                    rot: [
                        player.state.rotation.x,
                        player.state.rotation.y,
                        player.state.rotation.z,
                        player.state.rotation.w,
                    ],
                    vel: [
                        player.state.velocity.x,
                        player.state.velocity.y,
                        player.state.velocity.z,
                    ],
                    anim: player.state.animation_state as u8,
                }
            })
            .collect();

        // No players, nothing to broadcast
        if positions.is_empty() {
            return Ok(());
        }

        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;

        // Broadcast to each connected player
        for entry in connections.iter() {
            let (player_id, connection) = entry.pair();

            // Apply interest management if enabled
            let relevant_positions = if server.config.interest_radius > 0.0 {
                server.filter_by_distance(&positions, player_id, server.config.interest_radius)
            } else {
                positions.clone()
            };

            // Skip if no other players nearby
            if relevant_positions.is_empty() {
                continue;
            }

            // Create batch message
            let msg = UnreliableMessage::Batch {
                positions: relevant_positions,
                timestamp,
            };

            // Serialize and send
            if let Ok(data) = bincode::serialize(&msg) {
                match connection.send_datagram(&data) {
                    Ok(_) => {
                        server
                            .metrics
                            .messages_sent
                            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                        server
                            .metrics
                            .bytes_sent
                            .fetch_add(data.len() as u64, std::sync::atomic::Ordering::Relaxed);
                    }
                    Err(e) => {
                        tracing::warn!("Failed to send datagram to {}: {}", player_id, e);
                    }
                }
            }
        }

        Ok(())
    }

    /// Filter positions by distance (interest management)
    pub fn filter_by_distance(
        &self,
        positions: &[CompactPosition],
        player_id: &str,
        radius: f32,
    ) -> Vec<CompactPosition> {
        let player = match self.players.get(player_id) {
            Some(p) => p,
            None => return vec![],
        };

        let player_pos = player.state.position;
        let radius_sq = radius * radius;

        positions
            .iter()
            .filter(|p| {
                // Don't send player their own position
                if p.id == player_id {
                    return false;
                }

                let dx = p.pos[0] - player_pos.x;
                let dy = p.pos[1] - player_pos.y;
                let dz = p.pos[2] - player_pos.z;
                let distance_sq = dx * dx + dy * dy + dz * dz;

                distance_sq <= radius_sq
            })
            .take(self.config.max_visible_players)
            .cloned()
            .collect()
    }
}
