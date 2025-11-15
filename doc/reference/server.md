# Game Server Technical Design

## Overview
The game server is a Rust application that manages the shared world state, broadcasts player positions, validates actions, and handles WebTransport connections. It acts as the authoritative source of truth for the game world.

## Architecture

```
┌─────────────────────────────────────────────────────────┐
│                    Game Server (Rust)                    │
├─────────────────────────────────────────────────────────┤
│  WebTransport Endpoint (QUIC/HTTP3)                      │
│  ├── Connection Manager                                  │
│  ├── Message Router                                      │
│  └── Rate Limiter                                        │
├─────────────────────────────────────────────────────────┤
│  Game World State                                        │
│  ├── Player Registry (npub -> Player)                    │
│  ├── Spatial Index (position queries)                    │
│  ├── Physics Simulation (optional)                       │
│  └── Game Rules Engine                                   │
├─────────────────────────────────────────────────────────┤
│  Broadcasting System                                     │
│  ├── Position Aggregator                                 │
│  ├── Interest Management                                 │
│  └── Delta Compression                                   │
├─────────────────────────────────────────────────────────┤
│  Discovery Announcer                                     │
│  └── Nostr Client (announces server status)              │
└─────────────────────────────────────────────────────────┘
```

## Server Dependencies

```toml
[package]
name = "game-server"
version = "0.1.0"
edition = "2021"

[dependencies]
# WebTransport
wtransport = "0.1"  # Or quinn with HTTP/3
h3 = "0.0.3"
h3-quinn = "0.0.4"

# Async runtime
tokio = { version = "1", features = ["full"] }

# Game state
dashmap = "5.5"  # Concurrent hashmap for players
rstar = "0.11"    # R-tree for spatial queries

# Serialization
bincode = "1.3"
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"

# Nostr for discovery
nostr-sdk = "0.39"

# Utilities
tracing = "0.1"
tracing-subscriber = "0.3"
anyhow = "1.0"
clap = { version = "4.0", features = ["derive"] }
config = "0.13"

# Metrics
prometheus = "0.13"
```

## Core Server Implementation

```rust
// src/main.rs
use std::sync::Arc;
use tokio::sync::RwLock;
use dashmap::DashMap;

#[derive(Clone)]
struct GameServer {
    // Player management
    players: Arc<DashMap<String, Player>>,  // hex_id -> Player
    
    // Spatial indexing for efficient queries
    spatial_index: Arc<RwLock<RTree<PlayerLocation>>>,
    
    // Connection management
    connections: Arc<DashMap<String, PlayerConnection>>,
    
    // Server configuration
    config: ServerConfig,
    
    // Metrics
    metrics: Arc<ServerMetrics>,
    
    // Nostr announcer
    discovery: Option<Arc<DiscoveryService>>,
}

struct Player {
    identity: PlayerIdentity,
    state: PlayerState,
    connection_id: String,
    last_update: Instant,
    last_position: Vec3,
    velocity: Vec3,
    
    // Anti-cheat
    position_history: CircularBuffer<(Instant, Vec3)>,
    violation_count: u32,
}

struct PlayerConnection {
    transport: Arc<WebTransportConnection>,
    player_id: String,  // hex
    
    // Rate limiting
    message_bucket: TokenBucket,
    position_bucket: TokenBucket,
    
    // Connection quality
    ping: f64,
    packet_loss: f32,
}

struct ServerConfig {
    // Network
    bind_address: String,  // "0.0.0.0:4433"
    cert_path: String,
    key_path: String,
    
    // Game
    max_players: usize,
    world_size: f32,
    tick_rate: u32,  // Hz
    position_broadcast_rate: u32,  // Hz
    
    // Performance
    interest_radius: f32,  // Only send updates for nearby players
    max_visible_players: usize,
    position_interpolation: bool,
    
    // Anti-cheat
    max_move_speed: f32,  // units per second
    position_validation: bool,
    teleport_threshold: f32,
    
    // Discovery
    enable_discovery: bool,
    nostr_relays: Vec<String>,
    announce_interval: Duration,
}
```

## Message Processing

```rust
impl GameServer {
    /// Main message processing loop
    async fn handle_connection(&self, transport: WebTransportConnection) {
        let player_id = self.authenticate_connection(&transport).await?;
        
        // Set up connection
        let connection = PlayerConnection {
            transport: Arc::new(transport),
            player_id: player_id.clone(),
            message_bucket: TokenBucket::new(100, Duration::from_secs(1)),
            position_bucket: TokenBucket::new(60, Duration::from_secs(1)),
            ping: 0.0,
            packet_loss: 0.0,
        };
        
        self.connections.insert(player_id.clone(), connection);
        
        // Spawn handlers
        tokio::select! {
            _ = self.handle_reliable_messages(&transport, &player_id) => {},
            _ = self.handle_unreliable_messages(&transport, &player_id) => {},
            _ = self.handle_ping_pong(&transport, &player_id) => {},
        }
        
        // Cleanup on disconnect
        self.handle_disconnect(&player_id).await;
    }
    
    async fn handle_reliable_messages(&self, transport: &WebTransportConnection, player_id: &str) {
        while let Ok(stream) = transport.accept_uni().await {
            let data = read_stream(stream).await?;
            let msg: ReliableMessage = bincode::deserialize(&data)?;
            
            match msg {
                ReliableMessage::Join { player, position } => {
                    self.handle_player_join(player, position).await;
                }
                ReliableMessage::ChatMessage { content, .. } => {
                    // Validate and broadcast
                    if self.validate_chat(&content) {
                        self.broadcast_reliable(msg, Some(player_id)).await;
                    }
                }
                ReliableMessage::GameEvent { event_type, data } => {
                    self.handle_game_event(player_id, event_type, data).await;
                }
                _ => {}
            }
        }
    }
    
    async fn handle_unreliable_messages(&self, transport: &WebTransportConnection, player_id: &str) {
        let datagrams = transport.datagrams();
        let mut buf = vec![0u8; 1500];
        
        while let Ok(n) = datagrams.recv(&mut buf).await {
            // Rate limiting
            if !self.connections.get(player_id).unwrap().position_bucket.try_consume(1) {
                continue; // Drop packet if rate limited
            }
            
            let msg: UnreliableMessage = match bincode::deserialize(&buf[..n]) {
                Ok(m) => m,
                Err(_) => continue, // Drop malformed packets
            };
            
            match msg {
                UnreliableMessage::Position { x, y, z, rx, ry, rz, rw, seq } => {
                    // Validate position (anti-cheat)
                    if self.validate_position(player_id, x, y, z).await {
                        self.update_player_position(player_id, x, y, z, rx, ry, rz, rw, seq).await;
                    } else {
                        self.handle_position_violation(player_id).await;
                    }
                }
                _ => {}
            }
        }
    }
}
```

## Broadcasting System

```rust
impl GameServer {
    /// Efficient broadcasting with interest management
    async fn broadcast_positions(&self) {
        let mut interval = tokio::time::interval(
            Duration::from_millis(1000 / self.config.position_broadcast_rate as u64)
        );
        
        loop {
            interval.tick().await;
            
            // Collect all player positions
            let positions: Vec<CompactPosition> = self.players
                .iter()
                .map(|entry| {
                    let player = entry.value();
                    CompactPosition {
                        id: entry.key().clone(),
                        pos: [player.state.position.x, player.state.position.y, player.state.position.z],
                        rot: [
                            player.state.rotation.x, 
                            player.state.rotation.y, 
                            player.state.rotation.z, 
                            player.state.rotation.w
                        ],
                        vel: [player.velocity.x, player.velocity.y, player.velocity.z],
                        anim: player.state.animation_state as u8,
                    }
                })
                .collect();
            
            // Broadcast to each player based on interest
            for entry in self.connections.iter() {
                let (player_id, connection) = entry.pair();
                
                // Interest management: only send nearby players
                let relevant_positions = if self.config.interest_radius > 0.0 {
                    self.filter_by_distance(&positions, player_id, self.config.interest_radius)
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
                    timestamp: instant::now(),
                };
                
                // Send datagram
                if let Ok(data) = bincode::serialize(&msg) {
                    let _ = connection.transport.datagrams().send(&data).await;
                    
                    // Update metrics
                    self.metrics.messages_sent.fetch_add(1, Ordering::Relaxed);
                    self.metrics.bytes_sent.fetch_add(data.len() as u64, Ordering::Relaxed);
                }
            }
        }
    }
    
    /// Spatial query for interest management
    fn filter_by_distance(&self, positions: &[CompactPosition], player_id: &str, radius: f32) -> Vec<CompactPosition> {
        let player = match self.players.get(player_id) {
            Some(p) => p,
            None => return vec![],
        };
        
        let player_pos = player.state.position;
        
        positions.iter()
            .filter(|p| {
                if p.id == player_id {
                    return false; // Don't send player their own position
                }
                
                let dx = p.pos[0] - player_pos.x;
                let dy = p.pos[1] - player_pos.y;
                let dz = p.pos[2] - player_pos.z;
                let distance_sq = dx * dx + dy * dy + dz * dz;
                
                distance_sq <= radius * radius
            })
            .take(self.config.max_visible_players)
            .cloned()
            .collect()
    }
}
```

## Anti-Cheat Validation

```rust
impl GameServer {
    /// Validate position update for cheating
    async fn validate_position(&self, player_id: &str, x: f32, y: f32, z: f32) -> bool {
        if !self.config.position_validation {
            return true;
        }
        
        let mut player = match self.players.get_mut(player_id) {
            Some(p) => p,
            None => return false,
        };
        
        let new_pos = Vec3 { x, y, z };
        let old_pos = player.last_position;
        let dt = player.last_update.elapsed().as_secs_f32();
        
        // Check for teleportation
        let distance = ((new_pos.x - old_pos.x).powi(2) + 
                        (new_pos.y - old_pos.y).powi(2) + 
                        (new_pos.z - old_pos.z).powi(2)).sqrt();
        
        if distance > self.config.teleport_threshold {
            return false; // Obvious teleport
        }
        
        // Check movement speed
        let speed = distance / dt.max(0.001);
        if speed > self.config.max_move_speed {
            player.violation_count += 1;
            return false;
        }
        
        // Check world boundaries
        if x.abs() > self.config.world_size || 
           y < 0.0 || y > 1000.0 ||  // Assuming Y is up
           z.abs() > self.config.world_size {
            return false;
        }
        
        // Update history for pattern detection
        player.position_history.push((Instant::now(), new_pos));
        
        true
    }
    
    async fn handle_position_violation(&self, player_id: &str) {
        if let Some(mut player) = self.players.get_mut(player_id) {
            player.violation_count += 1;
            
            if player.violation_count > 10 {
                // Kick player for consistent violations
                self.kick_player(player_id, "Position validation failed").await;
            } else if player.violation_count > 5 {
                // Rubber-band player back to last valid position
                self.force_position_sync(player_id, player.last_position).await;
            }
        }
    }
}
```

## Discovery Announcer

```rust
impl DiscoveryService {
    /// Announce server to Nostr relays
    async fn announce_loop(&self, server: Arc<GameServer>) {
        let mut interval = tokio::time::interval(self.announce_interval);
        
        loop {
            interval.tick().await;
            
            let announcement = ServerAnnouncement {
                kind: Kind::Custom(30311),
                content: json!({
                    "game": "your-game-id",
                    "endpoint": self.endpoint,
                    "name": self.name,
                    "region": self.region,
                    "players": server.players.len(),
                    "max_players": server.config.max_players,
                    "version": env!("CARGO_PKG_VERSION"),
                    "features": ["webtransport", "nostr-auth"],
                    "ping": self.measure_median_ping(&server).await,
                }).to_string(),
                tags: vec![
                    Tag::custom(TagKind::D, vec![self.server_id]),
                    Tag::custom(TagKind::Custom("game"), vec!["your-game-id"]),
                    Tag::custom(TagKind::Custom("status"), vec!["online"]),
                ],
            };
            
            let _ = self.nostr_client.send_event(announcement).await;
        }
    }
}
```

## Server Launch Configuration

```rust
// src/config.rs
#[derive(Parser)]
struct Args {
    /// Bind address
    #[arg(long, default_value = "0.0.0.0:4433")]
    bind: String,
    
    /// Certificate path
    #[arg(long)]
    cert: String,
    
    /// Private key path
    #[arg(long)]
    key: String,
    
    /// Maximum players
    #[arg(long, default_value = "100")]
    max_players: usize,
    
    /// Enable Nostr discovery
    #[arg(long)]
    enable_discovery: bool,
    
    /// Nostr relays for discovery
    #[arg(long, value_delimiter = ',')]
    relays: Vec<String>,
    
    /// Interest management radius (0 = disabled)
    #[arg(long, default_value = "100.0")]
    interest_radius: f32,
    
    /// Enable anti-cheat validation
    #[arg(long, default_value = "true")]
    validate_positions: bool,
}

// Development mode
impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            bind_address: "127.0.0.1:4433".to_string(),
            cert_path: "localhost.pem".to_string(),
            key_path: "localhost-key.pem".to_string(),
            max_players: 10,
            tick_rate: 60,
            position_broadcast_rate: 30,
            interest_radius: 0.0,  // Disabled for dev
            max_visible_players: 100,
            position_interpolation: false,
            max_move_speed: 20.0,
            position_validation: false,  // Disabled for dev
            teleport_threshold: 50.0,
            enable_discovery: false,
            nostr_relays: vec![],
            announce_interval: Duration::from_secs(60),
        }
    }
}
```

## Running the Server

```bash
# Development
cargo run --bin server

# Production with discovery
cargo run --bin server -- \
  --bind 0.0.0.0:4433 \
  --cert /etc/certs/game.pem \
  --key /etc/certs/game-key.pem \
  --max-players 500 \
  --enable-discovery \
  --relays wss://relay.damus.io,wss://nos.lol \
  --interest-radius 200 \
  --validate-positions

# Docker deployment
FROM rust:1.75 as builder
WORKDIR /app
COPY . .
RUN cargo build --release --bin server

FROM debian:bookworm-slim
COPY --from=builder /app/target/release/server /usr/local/bin/server
EXPOSE 4433/udp
CMD ["server"]
```

## Performance Optimizations

1. **Interest Management**: Only send position updates for nearby players
2. **Delta Compression**: Send only changed positions
3. **Batch Updates**: Combine multiple positions in one datagram
4. **Spatial Indexing**: R-tree for efficient proximity queries
5. **Rate Limiting**: Per-player token buckets
6. **Connection Pooling**: Reuse QUIC connections

## Monitoring

```rust
struct ServerMetrics {
    connected_players: AtomicU64,
    messages_sent: AtomicU64,
    messages_received: AtomicU64,
    bytes_sent: AtomicU64,
    bytes_received: AtomicU64,
    position_violations: AtomicU64,
    average_ping: AtomicU64,
}

// Prometheus endpoint
async fn metrics_handler() -> String {
    format!(
        "# HELP connected_players Number of connected players\n\
         # TYPE connected_players gauge\n\
         connected_players {}\n",
        metrics.connected_players.load(Ordering::Relaxed)
    )
}
```

## Server Requirements

- **Single Server**: Authoritative for the entire world
- **Stateful**: Maintains player positions and game state
- **Real-time**: 30-60 Hz position updates
- **Scalable**: Handle 100-500 players per instance
- **Observable**: Metrics and logging for monitoring
