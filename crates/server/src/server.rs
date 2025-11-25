use crate::messages::{PlayerIdentity, PlayerState};
use dashmap::DashMap;
use glam::Vec3;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

/// Maximum size of position history buffer
const MAX_POSITION_HISTORY: usize = 60;

/// Game server state
#[derive(Clone)]
pub struct GameServer {
    /// Player management (hex_id -> Player)
    pub players: Arc<DashMap<String, Player>>,

    /// Server configuration
    pub config: Arc<ServerConfig>,

    /// Metrics
    pub metrics: Arc<ServerMetrics>,
}

impl GameServer {
    pub fn new(config: ServerConfig) -> Self {
        Self {
            players: Arc::new(DashMap::new()),
            config: Arc::new(config),
            metrics: Arc::new(ServerMetrics::default()),
        }
    }

    /// Get player count
    pub fn player_count(&self) -> usize {
        self.players.len()
    }

    /// Add a new player
    pub fn add_player(&self, hex_id: String, identity: PlayerIdentity, position: Vec3) -> bool {
        if self.players.len() >= self.config.max_players {
            return false;
        }

        let player = Player {
            identity,
            state: PlayerState {
                position,
                ..Default::default()
            },
            last_update: Instant::now(),
            last_position: position,
            velocity: Vec3::ZERO,
            position_history: CircularBuffer::new(MAX_POSITION_HISTORY),
            violation_count: 0,
        };

        self.players.insert(hex_id, player);
        self.metrics
            .connected_players
            .store(self.players.len() as u64, Ordering::Relaxed);
        true
    }

    /// Remove a player
    pub fn remove_player(&self, hex_id: &str) {
        self.players.remove(hex_id);
        self.metrics
            .connected_players
            .store(self.players.len() as u64, Ordering::Relaxed);
    }

    /// Update player position
    #[allow(dead_code)]
    pub fn update_player_position(
        &self,
        hex_id: &str,
        position: Vec3,
        rotation: glam::Quat,
        velocity: Vec3,
    ) -> bool {
        if let Some(mut player) = self.players.get_mut(hex_id) {
            player.last_position = player.state.position;
            player.state.position = position;
            player.state.rotation = rotation;
            player.state.velocity = velocity;
            player.last_update = Instant::now();
            player.position_history.push((Instant::now(), position));
            true
        } else {
            false
        }
    }

    /// Validate position update for anti-cheat
    pub fn validate_position(&self, hex_id: &str, new_pos: Vec3) -> bool {
        if !self.config.position_validation {
            return true;
        }

        let Some(player) = self.players.get(hex_id) else {
            return false;
        };

        let old_pos = player.last_position;
        let dt = player.last_update.elapsed().as_secs_f32().max(0.001);

        // Check for teleportation
        let distance = (new_pos - old_pos).length();
        if distance > self.config.teleport_threshold {
            tracing::warn!("Player {} teleported {} units", hex_id, distance);
            return false;
        }

        // Check movement speed
        let speed = distance / dt;
        if speed > self.config.max_move_speed {
            tracing::warn!(
                "Player {} moving too fast: {} units/sec (max: {})",
                hex_id,
                speed,
                self.config.max_move_speed
            );
            return false;
        }

        // Check world boundaries
        if new_pos.x.abs() > self.config.world_size
            || new_pos.y < 0.0
            || new_pos.y > 1000.0
            || new_pos.z.abs() > self.config.world_size
        {
            tracing::warn!("Player {} out of bounds: {:?}", hex_id, new_pos);
            return false;
        }

        true
    }

    /// Handle position violation
    pub fn handle_position_violation(&self, hex_id: &str) {
        if let Some(mut player) = self.players.get_mut(hex_id) {
            player.violation_count += 1;
            self.metrics
                .position_violations
                .fetch_add(1, Ordering::Relaxed);

            tracing::warn!(
                "Player {} violation count: {}",
                hex_id,
                player.violation_count
            );
        }
    }

    /// Get players within radius of a position
    pub fn get_players_in_radius(&self, center: Vec3, radius: f32) -> Vec<String> {
        let radius_sq = radius * radius;
        self.players
            .iter()
            .filter_map(|entry| {
                let distance_sq = (entry.value().state.position - center).length_squared();
                if distance_sq <= radius_sq {
                    Some(entry.key().clone())
                } else {
                    None
                }
            })
            .collect()
    }
}

/// Player data
#[allow(dead_code)]
pub struct Player {
    pub identity: PlayerIdentity,
    pub state: PlayerState,
    pub last_update: Instant,
    pub last_position: Vec3,
    pub velocity: Vec3,
    pub position_history: CircularBuffer<(Instant, Vec3)>,
    pub violation_count: u32,
}

/// Server configuration
#[derive(Debug, Clone)]
pub struct ServerConfig {
    // Network
    pub bind_address: String,
    pub cert_path: String,
    pub key_path: String,

    // Game
    pub max_players: usize,
    pub world_size: f32,
    pub tick_rate: u32,
    pub position_broadcast_rate: u32,

    // Performance
    pub interest_radius: f32,
    pub max_visible_players: usize,

    // Anti-cheat
    pub max_move_speed: f32,
    pub position_validation: bool,
    pub teleport_threshold: f32,

    // Discovery
    pub enable_discovery: bool,
    pub nostr_relays: Vec<String>,
    pub announce_interval: Duration,

    // Server identity
    pub server_name: String,
    pub server_region: String,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            bind_address: "127.0.0.1:4433".to_string(),
            cert_path: "localhost.pem".to_string(),
            key_path: "localhost-key.pem".to_string(),
            max_players: 10,
            world_size: 1000.0,
            tick_rate: 60,
            position_broadcast_rate: 30,
            interest_radius: 0.0, // Disabled for dev
            max_visible_players: 100,
            max_move_speed: 20.0,
            position_validation: false, // Disabled for dev
            teleport_threshold: 50.0,
            enable_discovery: false,
            nostr_relays: vec![],
            announce_interval: Duration::from_secs(60),
            server_name: "Crossworld Dev Server".to_string(),
            server_region: "local".to_string(),
        }
    }
}

/// Server metrics
#[derive(Default)]
pub struct ServerMetrics {
    pub connected_players: AtomicU64,
    pub messages_sent: AtomicU64,
    pub messages_received: AtomicU64,
    pub bytes_sent: AtomicU64,
    pub bytes_received: AtomicU64,
    pub position_violations: AtomicU64,
}

/// Simple circular buffer
#[allow(dead_code)]
pub struct CircularBuffer<T> {
    buffer: Vec<T>,
    capacity: usize,
    index: usize,
}

#[allow(dead_code)]
impl<T> CircularBuffer<T> {
    pub fn new(capacity: usize) -> Self {
        Self {
            buffer: Vec::with_capacity(capacity),
            capacity,
            index: 0,
        }
    }

    pub fn push(&mut self, item: T) {
        if self.buffer.len() < self.capacity {
            self.buffer.push(item);
        } else {
            self.buffer[self.index] = item;
            self.index = (self.index + 1) % self.capacity;
        }
    }

    pub fn iter(&self) -> impl Iterator<Item = &T> {
        self.buffer.iter()
    }
}
