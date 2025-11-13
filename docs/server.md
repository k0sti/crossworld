# Crossworld Server Architecture

## Overview

The Crossworld server provides multiplayer access to shared voxel worlds using a client-server architecture built on WebTransport. The server acts as a central coordination point and world data provider, enabling clients to:

- Connect from browsers (WASM) and native clients using WebTransport
- Authenticate using Nostr public keys (npub) with signature verification
- Request portions of the world at various detail levels (LOD)
- Subscribe to real-time world updates via bidirectional streams
- Edit world data with proper authorization

## Technology Stack

### Core Dependencies

- **web-transport-quinn** (native server): QUIC-based WebTransport server implementation
  - Built on Quinn (production-ready QUIC implementation)
  - TLS 1.3 encryption with certificate-based security
  - Native HTTP/3 support for WebTransport
  - Multiplexed bidirectional and unidirectional streams

- **web-transport-wasm** (browser client): WebTransport API for WASM
  - Browser-native WebTransport support
  - Platform-adaptive implementation via web-transport crate
  - Zero additional dependencies for browser clients

- **nostr** / **nostr-sdk**: Nostr protocol integration for npub authentication
  - Schnorr signature verification (secp256k1)
  - Public key cryptography for identity

- **serde** + **bincode**: Efficient binary serialization for protocol messages
  - Compact binary encoding for minimal bandwidth
  - Compatible with both native and WASM environments

- **tokio**: Async runtime for handling concurrent connections
  - Async/await for connection handling
  - Efficient multi-stream management

## Architecture

### Project Structure

```
crates/server/
├── Cargo.toml
├── src/
│   ├── lib.rs              # Core server library
│   ├── main.rs             # Server binary entry point
│   ├── auth/
│   │   ├── mod.rs          # Authentication system
│   │   └── npub.rs         # Nostr public key handling
│   ├── world/
│   │   ├── mod.rs          # World data management
│   │   ├── storage.rs      # Persistent world storage
│   │   ├── cache.rs        # In-memory world cache
│   │   └── lod.rs          # Level-of-detail processing
│   ├── protocol/
│   │   ├── mod.rs          # Protocol definitions
│   │   ├── messages.rs     # Message types
│   │   └── handshake.rs    # Connection handshake
│   ├── network/
│   │   ├── mod.rs          # Network layer
│   │   ├── webtransport.rs # WebTransport server implementation
│   │   ├── session.rs      # Per-session connection state
│   │   └── broadcast.rs    # Pub/sub broadcast system
│   └── config.rs           # Server configuration
└── README.md
```

## Protocol Design

### Connection Flow

```
Client (Browser/WASM or Native)           Server (WebTransport/Quinn)
  |                                         |
  |---- WebTransport connect (HTTPS) -----> |
  |      (TLS 1.3 handshake)                |
  | <---- TLS certificate verification ---- |
  |                                         |
  |---- Open bidirectional stream --------> |
  |                                         |
  |---- Handshake (npub, signature) ------> |
  |                                         | (verify Nostr signature)
  |                                         | (check authorization)
  | <---- HandshakeAck (session_id) ------- |
  |                                         |
  |---- WorldRequest ----------------------> |
  | <---- WorldData (via stream) ---------- |
  |                                         |
  |---- WorldEdit (authorized) ------------> |
  | <---- WorldEditAck/Error --------------- |
  |                                         |
  | <---- WorldUpdate (subscription) ------ | (real-time broadcast)
  |                                         |
```

### Serialization Strategy

The protocol uses **binary serialization via `bincode`** for all message types, providing:
- Compact encoding (much smaller than JSON)
- Fast serialization/deserialization
- Zero-copy deserialization where possible
- Native support for `Cube<u8>` via serde

**Key Data Types:**
- `Cube<u8>`: The octree structure from `crates/cube`
  - `u8` represents material ID (0-255)
  - Recursive enum: `Solid(u8)` | `Octa([Box<Cube<u8>>; 8])`
  - Implements `Serialize`/`Deserialize` automatically
  - Binary encoding is optimal for network transfer

**Why Binary over CSM Text?**
- **Size**: Binary `Cube<u8>` is ~60-80% smaller than CSM text format
- **Speed**: No parsing required, direct deserialization
- **Type safety**: Compile-time guarantees
- **WASM friendly**: Works identically in browser and native
- **CSM available**: Server can still export CSM via `serialize_csm()` for debugging/storage

### Message Types

All messages use binary encoding via `bincode` for efficiency.

```rust
use serde::{Deserialize, Serialize};
use glam::IVec3;
use cube::{Cube, CubeCoord};

/// Client -> Server: Initial handshake
#[derive(Serialize, Deserialize, Debug)]
pub struct Handshake {
    /// Nostr public key (npub in hex format)
    pub npub: String,

    /// Timestamp for replay protection
    pub timestamp: u64,

    /// Signature of (server_url + timestamp) using Nostr private key (Schnorr)
    /// This proves the client owns the npub
    pub signature: Vec<u8>,

    /// Optional: Client display name
    pub display_name: Option<String>,
}

/// Server -> Client: Handshake acknowledgment
#[derive(Serialize, Deserialize, Debug)]
pub struct HandshakeAck {
    /// Unique session identifier
    pub session_id: u64,

    /// Server capabilities and world info
    pub world_info: WorldInfo,

    /// Client's authorization level
    pub auth_level: AuthLevel,
}

/// Authorization levels for different operations
#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuthLevel {
    /// Can only read world data
    ReadOnly,

    /// Can read and make limited edits (quotas apply)
    User,

    /// Can make unlimited edits and access admin features
    Admin,
}

/// World metadata
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct WorldInfo {
    /// World identifier
    pub world_id: String,

    /// Maximum depth level available
    pub max_depth: u32,

    /// Macro depth (terrain generation depth)
    pub macro_depth: u32,

    /// Whether the world has border layers
    pub border_depth: u32,
}

/// Client -> Server: Request world data
#[derive(Serialize, Deserialize, Debug)]
pub struct WorldRequest {
    /// Session ID from handshake
    pub session_id: u64,

    /// Position and depth in world space
    /// CubeCoord specifies both the coordinate and the octree depth level
    /// - coord.pos: Position in world space (unit cubes)
    /// - coord.depth: Requested octree depth level
    ///   - Higher values = more detail
    ///   - depth=0: entire world as single cube
    ///   - depth=macro_depth: terrain detail
    ///   - depth=macro_depth+micro_depth: finest detail
    pub coord: CubeCoord,

    /// Optional: subscribe to updates for this region
    pub subscribe: bool,
}

/// Server -> Client: World data response
#[derive(Serialize, Deserialize, Debug)]
pub struct WorldData {
    /// Request coordinate and depth (echo back)
    pub coord: CubeCoord,

    /// Cube data - the root octree node for the requested region
    /// Type: Cube<u8> where u8 represents material ID (0-255)
    /// Serialized directly via serde (Cube implements Serialize/Deserialize)
    pub root: Cube<u8>,

    /// Subscription ID if subscription was requested
    pub subscription_id: Option<u64>,
}

/// Client -> Server: Edit world data
#[derive(Serialize, Deserialize, Debug)]
pub struct WorldEdit {
    /// Session ID
    pub session_id: u64,

    /// Edit operation
    pub operation: EditOperation,

    /// Client-side transaction ID for acknowledgment tracking
    pub transaction_id: u64,
}

/// Types of world editing operations
#[derive(Serialize, Deserialize, Debug)]
pub enum EditOperation {
    /// Replace an entire cube region with binary Cube data
    SetCube {
        coord: CubeCoord,
        cube: Cube<u8>,  // Direct serialization of Cube<u8>
    },
}

/// Server -> Client: Edit acknowledgment
#[derive(Serialize, Deserialize, Debug)]
pub struct WorldEditAck {
    /// Original transaction ID
    pub transaction_id: u64,

    /// Result of the operation
    pub result: EditResult,
}

/// Result of an edit operation
#[derive(Serialize, Deserialize, Debug)]
pub enum EditResult {
    /// Edit applied successfully
    Success,

    /// Edit rejected with reason
    Error(EditError),
}

/// Reasons an edit might be rejected
#[derive(Serialize, Deserialize, Debug)]
pub enum EditError {
    /// Client not authorized for this operation
    Unauthorized,

    /// Invalid coordinates or depth
    InvalidCoordinates,

    /// Quota exceeded (rate limiting)
    QuotaExceeded,

    /// Server-side error
    ServerError(String),
}

/// Server -> Client: Real-time update notification
#[derive(Serialize, Deserialize, Debug)]
pub struct WorldUpdate {
    /// Subscription ID this update belongs to
    pub subscription_id: u64,

    /// The edit that occurred
    pub operation: EditOperation,

    /// npub of the user who made the edit
    pub author: String,

    /// Server timestamp
    pub timestamp: u64,
}
```

### Authentication System

#### Nostr-based Authentication

The server uses Nostr public keys (npub) for identity:

1. **Client generates signature**: Sign `server_url + timestamp` with Nostr private key (Schnorr/secp256k1)
2. **Server verifies signature**: Using the provided npub and Nostr signature verification
3. **Server checks authorization**: Compare npub against admin/user lists

**Why Nostr Authentication?**
- Decentralized identity (no central authority required)
- Users control their own keys
- Same identity across multiple servers/worlds
- Built-in cryptographic verification
- Compatible with existing Nostr ecosystem

```rust
pub struct AuthConfig {
    /// List of npubs with admin access
    pub admin_npubs: Vec<String>,

    /// List of npubs with user access (if None, all verified npubs get user access)
    pub user_npubs: Option<Vec<String>>,

    /// Whether to allow read-only access without authentication
    pub allow_anonymous_read: bool,

    /// Maximum timestamp age for replay protection (seconds)
    pub max_timestamp_age: u64,
}

impl AuthConfig {
    pub fn determine_auth_level(&self, npub: &str) -> AuthLevel {
        if self.admin_npubs.contains(&npub.to_string()) {
            return AuthLevel::Admin;
        }

        if let Some(ref users) = self.user_npubs {
            if users.contains(&npub.to_string()) {
                return AuthLevel::User;
            }
            return AuthLevel::ReadOnly;
        }

        // Default: all verified npubs get user access
        AuthLevel::User
    }
}
```

### World Data Management

#### Octree Storage

The server maintains the world state using the same `Octree` structure from the `cube` crate:

```rust
use cube::{Octree, Cube};

pub struct WorldStorage {
    /// Root octree
    octree: Octree,

    /// Macro depth (terrain generation)
    macro_depth: u32,

    /// Micro depth (user edits)
    micro_depth: u32,

    /// Border depth
    border_depth: u32,

    /// Persistent storage backend
    backend: StorageBackend,
}

impl WorldStorage {
    /// Get a cube at the specified coordinate and depth
    /// Returns the cube subtree for the requested region
    pub fn get_cube(&self, coord: CubeCoord) -> Result<Cube<u8>, WorldError> {
        // Navigate octree to requested location
        // Extract the cube subtree at that position and depth
        // The Cube<u8> is directly serializable via serde
        todo!()
    }

    /// Set an entire cube region
    pub fn set_cube(&mut self, coord: CubeCoord, cube: Cube<u8>) -> Result<(), WorldError> {
        // Replace the octree subtree at coord/depth with the provided cube
        // More efficient than individual voxel operations
        // Useful for bulk imports or area replacements
        todo!()
    }

    /// Export a region to CSM format (for debugging/backups)
    pub fn export_csm(&self, coord: CubeCoord) -> Result<String, WorldError> {
        let cube = self.get_cube(coord, depth)?;
        Ok(cube::serialize_csm(&cube))
    }

    /// Import from CSM format (for debugging/restoring backups)
    pub fn import_csm(&mut self, coord: CubeCoord, csm: &str) -> Result<(), WorldError> {
        let cube = cube::parse_csm(csm)?;
        self.set_cube(coord, depth, cube)
    }
}
```

#### Depth

Server serves Cube that is at least of requested depth on given position.
Server provides automatic level-of-detail by providing cubes farther from position with lower resolution.

#### Persistence

World state is persisted using a pluggable backend:

```rust
pub trait StorageBackend: Send + Sync {
    /// Save the entire world state
    /// The root Cube<u8> is serialized to disk via bincode
    fn save_world(&self, root: &Cube<u8>) -> Result<(), StorageError>;

    /// Load the world state
    /// Deserializes the root Cube<u8> from disk
    fn load_world(&self) -> Result<Cube<u8>, StorageError>;

    /// Save incremental edits (for efficiency and rollback capability)
    fn save_edit(&self, edit: &EditOperation, timestamp: u64, author: &str) -> Result<(), StorageError>;

    /// Compact edit log into the main world file
    fn compact(&self) -> Result<(), StorageError>;
}

/// File-based storage implementation
pub struct FileStorage {
    /// Path to main world file (bincode-serialized Cube<u8>)
    world_path: PathBuf,

    /// Path to edit log (append-only bincode stream)
    edit_log_path: PathBuf,
}

impl StorageBackend for FileStorage {
    fn save_world(&self, root: &Cube<u8>) -> Result<(), StorageError> {
        // Serialize Cube<u8> to bincode
        let data = bincode::serialize(root)?;

        // Write to file atomically (temp file + rename)
        let temp_path = self.world_path.with_extension("tmp");
        std::fs::write(&temp_path, data)?;
        std::fs::rename(temp_path, &self.world_path)?;

        Ok(())
    }

    fn load_world(&self) -> Result<Cube<u8>, StorageError> {
        // Read bincode file
        let data = std::fs::read(&self.world_path)?;

        // Deserialize Cube<u8>
        let root: Cube<u8> = bincode::deserialize(&data)?;

        Ok(root)
    }

    fn save_edit(&self, edit: &EditOperation, timestamp: u64, author: &str) -> Result<(), StorageError> {
        // Append edit to log file
        let log_entry = EditLogEntry {
            timestamp,
            author: author.to_string(),
            operation: edit.clone(),
        };

        let data = bincode::serialize(&log_entry)?;

        // Append to log file
        use std::io::Write;
        let mut file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.edit_log_path)?;

        // Write length prefix + data
        file.write_all(&(data.len() as u32).to_be_bytes())?;
        file.write_all(&data)?;

        Ok(())
    }

    fn compact(&self) -> Result<(), StorageError> {
        // Clear edit log after saving world state
        std::fs::write(&self.edit_log_path, &[])?;
        Ok(())
    }
}

#[derive(Serialize, Deserialize)]
struct EditLogEntry {
    timestamp: u64,
    author: String,
    operation: EditOperation,
}

/// Future: Database-based storage for larger worlds
pub struct DatabaseStorage {
    // Connection to PostgreSQL/SQLite/etc
    // Store Cube<u8> as BLOB
    // Edit log as structured table with spatial indices
}
```

#### Serialization Performance Analysis

### Network Layer (WebTransport Integration)

#### Server Initialization

```rust
use wtransport::{Endpoint, ServerConfig, Certificate};
use tokio::sync::{RwLock, broadcast};
use std::sync::Arc;
use std::collections::HashMap;

pub struct WebTransportServer {
    /// WebTransport endpoint (Quinn-based)
    endpoint: Endpoint,

    /// World storage
    world: Arc<RwLock<WorldStorage>>,

    /// Active client sessions
    sessions: Arc<RwLock<HashMap<u64, ClientSession>>>,

    /// Authentication config
    auth_config: AuthConfig,

    /// Broadcast channels for pub/sub
    broadcast_tx: broadcast::Sender<WorldUpdate>,
}

impl WebTransportServer {
    pub async fn new(config: ServerConfig) -> Result<Self, ServerError> {
        // Load or generate TLS certificate
        let cert = Certificate::load(&config.cert_path, &config.key_path)
            .await
            .or_else(|_| Certificate::self_signed(&config.server_domain))?;

        // Create WebTransport endpoint
        let server_config = wtransport::ServerConfig::builder()
            .with_bind_address(config.bind_address)
            .with_certificate(cert)
            .build();

        let endpoint = Endpoint::server(server_config)?;

        // Load world state
        let world = WorldStorage::load_or_create(config.world_config)?;

        // Create broadcast channel for world updates
        let (broadcast_tx, _) = broadcast::channel(1000);

        Ok(Self {
            endpoint,
            world: Arc::new(RwLock::new(world)),
            sessions: Arc::new(RwLock::new(HashMap::new())),
            auth_config: config.auth_config,
            broadcast_tx,
        })
    }

    pub async fn run(&self) -> Result<(), ServerError> {
        tracing::info!("WebTransport server listening on {}", self.endpoint.local_addr()?);

        loop {
            // Accept incoming WebTransport connections
            let incoming = self.endpoint.accept().await;

            // Spawn handler for this connection
            let server = self.clone();
            tokio::spawn(async move {
                match incoming.await {
                    Ok(conn) => {
                        tracing::info!("New connection from {:?}", conn.remote_address());
                        if let Err(e) = server.handle_connection(conn).await {
                            tracing::error!("Connection error: {}", e);
                        }
                    }
                    Err(e) => {
                        tracing::error!("Failed to accept connection: {}", e);
                    }
                }
            });
        }
    }

    async fn handle_connection(&self, conn: wtransport::Connection) -> Result<(), ServerError> {
        // Accept the first bidirectional stream for handshake
        let (mut send, mut recv) = conn.accept_bi().await?;

        // Read handshake message
        let handshake: Handshake = read_message(&mut recv).await?;

        // Verify Nostr signature
        let auth_level = self.verify_handshake(&handshake)?;

        // Generate session ID
        let session_id = generate_session_id();

        // Send acknowledgment
        let ack = HandshakeAck {
            session_id,
            world_info: self.get_world_info(),
            auth_level,
        };
        write_message(&mut send, &ack).await?;

        // Create client session
        let session = ClientSession::new(
            session_id,
            conn.clone(),
            auth_level,
            self.broadcast_tx.subscribe(),
        );

        self.sessions.write().await.insert(session_id, session.clone());

        // Handle messages from this session
        self.handle_session_messages(session, send, recv).await?;

        // Clean up session on disconnect
        self.sessions.write().await.remove(&session_id);

        Ok(())
    }

    async fn handle_session_messages(
        &self,
        session: ClientSession,
        mut send: wtransport::SendStream,
        mut recv: wtransport::RecvStream,
    ) -> Result<(), ServerError> {
        loop {
            tokio::select! {
                // Handle incoming messages from client
                msg = read_message(&mut recv) => {
                    match msg? {
                        ClientMessage::WorldRequest(req) => {
                            self.handle_world_request(&session, &mut send, req).await?;
                        }
                        ClientMessage::WorldEdit(edit) => {
                            self.handle_world_edit(&session, &mut send, edit).await?;
                        }
                        ClientMessage::Disconnect => {
                            break;
                        }
                    }
                }

                // Handle broadcast updates for subscriptions
                update = session.broadcast_rx.recv() => {
                    if let Ok(update) = update {
                        // Check if this update matches any of the session's subscriptions
                        if session.is_subscribed_to(&update).await {
                            write_message(&mut send, &update).await?;
                        }
                    }
                }
            }
        }

        Ok(())
    }

    fn verify_handshake(&self, handshake: &Handshake) -> Result<AuthLevel, ServerError> {
        // Check timestamp for replay protection
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)?
            .as_secs();

        if now.saturating_sub(handshake.timestamp) > self.auth_config.max_timestamp_age {
            return Err(ServerError::HandshakeExpired);
        }

        // Verify Nostr signature
        let message = format!("{}{}", self.endpoint.local_addr()?, handshake.timestamp);
        let pubkey = nostr_sdk::PublicKey::from_str(&handshake.npub)?;
        let signature = nostr_sdk::Signature::from_slice(&handshake.signature)?;

        if !pubkey.verify(&message.as_bytes(), &signature) {
            return Err(ServerError::InvalidSignature);
        }

        // Determine authorization level
        Ok(self.auth_config.determine_auth_level(&handshake.npub))
    }

    async fn handle_world_request(
        &self,
        session: &ClientSession,
        send: &mut wtransport::SendStream,
        req: WorldRequest,
    ) -> Result<(), ServerError> {
        // Verify session ID
        if req.session_id != session.session_id {
            return Err(ServerError::InvalidSession);
        }

        // Get world data
        let world = self.world.read().await;
        let cube = world.get_cube(req.coord)?;

        // Handle subscription if requested
        let subscription_id = if req.subscribe {
            let sub_id = session.add_subscription(req.coord).await;
            Some(sub_id)
        } else {
            None
        };

        // Send response
        let response = WorldData {
            coord: req.coord,
            root: cube,  // Cube<u8> serialized directly via bincode
            subscription_id,
        };

        write_message(send, &response).await?;

        Ok(())
    }

    async fn handle_world_edit(
        &self,
        session: &ClientSession,
        send: &mut wtransport::SendStream,
        edit: WorldEdit,
    ) -> Result<(), ServerError> {
        // Verify session ID
        if edit.session_id != session.session_id {
            return Err(ServerError::InvalidSession);
        }

        // Check authorization
        if !session.can_edit() {
            let ack = WorldEditAck {
                transaction_id: edit.transaction_id,
                result: EditResult::Error(EditError::Unauthorized),
            };
            write_message(send, &ack).await?;
            return Ok(());
        }

        // Check rate limit
        if !session.check_rate_limit().await {
            let ack = WorldEditAck {
                transaction_id: edit.transaction_id,
                result: EditResult::Error(EditError::QuotaExceeded),
            };
            write_message(send, &ack).await?;
            return Ok(());
        }

        // Apply edit to world
        let mut world = self.world.write().await;
        let result = match &edit.operation {
            EditOperation::SetCube { coord, cube } => {
                world.set_cube(*coord, cube.clone())
            }
        };

        // Send acknowledgment
        let ack = WorldEditAck {
            transaction_id: edit.transaction_id,
            result: if result.is_ok() {
                EditResult::Success
            } else {
                EditResult::Error(EditError::ServerError(result.unwrap_err().to_string()))
            },
        };
        write_message(send, &ack).await?;

        // Broadcast update to subscribers
        if result.is_ok() {
            let update = WorldUpdate {
                subscription_id: 0, // Will be filtered per-session
                operation: edit.operation,
                author: session.npub.clone(),
                timestamp: SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs(),
            };

            // Ignore broadcast errors (no subscribers is ok)
            let _ = self.broadcast_tx.send(update);
        }

        Ok(())
    }
}

/// Helper functions for reading/writing bincode messages over WebTransport streams
async fn read_message<T: serde::de::DeserializeOwned>(
    stream: &mut wtransport::RecvStream,
) -> Result<T, ServerError> {
    // Read message length (4 bytes)
    let mut len_buf = [0u8; 4];
    stream.read_exact(&mut len_buf).await?;
    let len = u32::from_be_bytes(len_buf) as usize;

    // Read message data
    let mut data = vec![0u8; len];
    stream.read_exact(&mut data).await?;

    // Deserialize
    Ok(bincode::deserialize(&data)?)
}

async fn write_message<T: serde::Serialize>(
    stream: &mut wtransport::SendStream,
    msg: &T,
) -> Result<(), ServerError> {
    // Serialize message
    let data = bincode::serialize(msg)?;

    // Write length prefix
    let len = data.len() as u32;
    stream.write_all(&len.to_be_bytes()).await?;

    // Write message data
    stream.write_all(&data).await?;

    Ok(())
}
```

#### Per-Session State

```rust
pub struct ClientSession {
    session_id: u64,
    connection: wtransport::Connection,
    auth_level: AuthLevel,
    npub: String,
    subscriptions: Arc<RwLock<Vec<Subscription>>>,
    rate_limiter: Arc<RwLock<RateLimiter>>,
    broadcast_rx: broadcast::Receiver<WorldUpdate>,
}

impl ClientSession {
    pub fn new(
        session_id: u64,
        connection: wtransport::Connection,
        auth_level: AuthLevel,
        broadcast_rx: broadcast::Receiver<WorldUpdate>,
    ) -> Self {
        Self {
            session_id,
            connection,
            auth_level,
            npub: String::new(),
            subscriptions: Arc::new(RwLock::new(Vec::new())),
            rate_limiter: Arc::new(RwLock::new(RateLimiter::new())),
            broadcast_rx,
        }
    }

    pub fn can_edit(&self) -> bool {
        matches!(self.auth_level, AuthLevel::User | AuthLevel::Admin)
    }

    pub async fn add_subscription(&self, coord: CubeCoord) -> u64 {
        let mut subs = self.subscriptions.write().await;
        let id = generate_subscription_id();
        subs.push(Subscription { id, coord });
        id
    }

    pub async fn is_subscribed_to(&self, update: &WorldUpdate) -> bool {
        let subs = self.subscriptions.read().await;
        // Check if update's coordinates overlap with any subscription
        // This is a simplified check - full implementation would check spatial overlap
        !subs.is_empty()
    }

    pub async fn check_rate_limit(&self) -> bool {
        let mut limiter = self.rate_limiter.write().await;
        limiter.check_and_consume()
    }
}

pub struct Subscription {
    id: u64,
    coord: CubeCoord,
}

/// Token bucket rate limiter
pub struct RateLimiter {
    tokens: f64,
    max_tokens: f64,
    refill_rate: f64, // tokens per second
    last_refill: std::time::Instant,
}

impl RateLimiter {
    pub fn new() -> Self {
        Self {
            tokens: 10.0,
            max_tokens: 10.0,
            refill_rate: 10.0,
            last_refill: std::time::Instant::now(),
        }
    }

    pub fn check_and_consume(&mut self) -> bool {
        // Refill tokens based on elapsed time
        let now = std::time::Instant::now();
        let elapsed = now.duration_since(self.last_refill).as_secs_f64();
        self.tokens = (self.tokens + elapsed * self.refill_rate).min(self.max_tokens);
        self.last_refill = now;

        // Check if we have tokens available
        if self.tokens >= 1.0 {
            self.tokens -= 1.0;
            true
        } else {
            false
        }
    }
}
```

#### Broadcast System

The server uses tokio's broadcast channels for efficient pub/sub:

```rust
// When a client makes an edit:
let update = WorldUpdate { /* ... */ };
broadcast_tx.send(update)?;

// Each session receives updates via its own receiver:
// (in handle_session_messages)
tokio::select! {
    update = session.broadcast_rx.recv() => {
        if session.is_subscribed_to(&update) {
            write_message(&mut send, &update).await?;
        }
    }
}
```

This provides efficient broadcasting without needing to track all subscribers manually.

## Configuration

### Server Configuration File

```toml
# server.toml

[network]
bind_address = "0.0.0.0:4433"
server_domain = "crossworld.atlantislabs.live"

# TLS certificate paths (will auto-generate self-signed if not found)
cert_path = "./certs/cert.pem"
key_path = "./certs/key.pem"

[world]
world_id = "main"
world_path = "./worlds/main.world"
edit_log_path = "./worlds/main.edits"

# World parameters
macro_depth = 3
micro_depth = 3
border_depth = 1
seed = 42

[auth]
# Admin npubs (full access)
admin_npubs = [
    "npub1zc6ts76lel22d38l9uk7zazsen8yd7dtuzcz5uv8d3vkast9hlks4725sl",
]

# User npubs (if empty, all verified npubs get user access)
user_npubs = []

# Allow anonymous read-only access
allow_anonymous_read = true

# Replay protection window (seconds)
max_timestamp_age = 300

[limits]
# Rate limiting per user
max_edits_per_second = 10
max_subscriptions_per_client = 100
```

### Running the Server

```bash
# Start server
cargo run --release -p server

# With custom config
cargo run --release -p server -- --config /path/to/server.toml

# Generate TLS certificate
cargo run --release -p server -- --gen-cert
```

## Security Considerations

### Authentication Security

- **Signature verification**: Prevents impersonation of npubs
- **Timestamp replay protection**: Prevents replay attacks
- **Per-session authorization**: Auth level checked for each operation

### Rate Limiting

```rust
pub struct RateLimiter {
    edits_per_second: u32,
    subscriptions_per_client: u32,

    // Per-session tracking
    edit_tokens: HashMap<u64, TokenBucket>,
}
```

### Data Validation

- Coordinate bounds checking
- CSM format validation before applying edits
- Depth level validation

## Data Serialization Summary

### Protocol Data Flow

```
Client Request:
  WorldRequest { session_id, coord: CubeCoord { pos, depth }, subscribe }
    ↓ (bincode serialization)
  [4-byte length][binary message] → WebTransport stream
    ↓
  Server receives and deserializes

Server Response:
  WorldData { coord: CubeCoord, root: Cube<u8>, subscription_id }
    ↓ (bincode serialization of entire struct including Cube<u8>)
  [4-byte length][binary message] → WebTransport stream
    ↓
  Client receives and deserializes
    ↓
  Cube<u8> ready for immediate use (mesh gen, raycast, etc.)
  CubeCoord provides both position and depth information
```

### Key Design Decisions

1. **Binary over Text**: `bincode` instead of CSM/JSON
   - 3x faster serialization
   - 60-80% smaller payload size
   - Zero parsing overhead
   - Type-safe

2. **Direct `Cube<u8>` Transfer**: No intermediate formats
   - Server extracts `Cube<u8>` from octree
   - Serializes directly via bincode
   - Client deserializes to `Cube<u8>`
   - No conversion needed

3. **Material IDs as `u8`**: 0-255 range
   - `0` = empty or no operation for source cube data (keep source)
   - `1-255` = materials
   - Matches existing material system
   - Compact representation

4. **CSM**: Text export
   - Available for debugging
   - Useful for persistence/backups
   - Can convert between formats when needed
   - Not used for network protocol

### Example Message Sizes

```rust
// WorldRequest message
struct WorldRequest {
    session_id: u64,      // 8 bytes
    coord: CubeCoord,     // 16 bytes (IVec3 pos + u32 depth)
    subscribe: bool,      // 1 byte
}
// Total: 25 bytes + 4-byte length prefix = 29 bytes

// WorldData message (depth 3, typical terrain)
struct WorldData {
    coord: CubeCoord,     // 16 bytes (IVec3 pos + u32 depth)
    root: Cube<u8>,       // ~800 bytes (typical)
    subscription_id: Option<u64>,  // 9 bytes (1 tag + 8 value)
}
// Total: ~825 bytes + 4-byte length prefix = ~829 bytes

// EditOperation::SetCube message
struct SetCube {
    coord: CubeCoord,     // 16 bytes
    cube: Cube<u8>,       // Variable (could be 2 bytes for Solid, or KB for complex)
}

// For comparison, CSM equivalent would be ~2.5KB + overhead
```

### Integration with Existing Code

The `Cube<u8>` type from your `crates/cube` already has everything needed:

```rust
// Server side: Extract from world using CubeCoord
let coord = CubeCoord { pos: IVec3::new(0, 0, 0), depth: 3 };
let cube: Cube<u8> = world_storage.get_cube(coord)?;

// Send via network (automatic serialization)
let msg = WorldData { coord, root: cube, subscription_id: None };
write_message(&stream, &msg).await?;

// Client side: Receive (automatic deserialization)
let msg: WorldData = read_message(&stream).await?;

// CubeCoord tells you position AND depth
println!("Received cube at {:?} with depth {}", msg.coord.pos, msg.coord.depth);

// Use immediately with existing cube functions
let mesh = generate_face_mesh(&msg.root, &DefaultMeshBuilder);
let hit = raycast(&msg.root, ray_origin, ray_direction);
// ... etc
```

## Why WebTransport?

WebTransport provides several key advantages for Crossworld:

### Browser Compatibility
- **Native WASM support**: Runs directly in modern browsers (Chrome, Edge, Firefox)
- **No complex dependencies**: Browser's built-in WebTransport API
- **Same protocol everywhere**: Consistent behavior between native and WASM clients

### Performance Benefits
- **QUIC-based**: 0-RTT connection establishment reduces latency
- **Multiplexed streams**: Multiple independent data streams without head-of-line blocking
- **Better congestion control**: Modern algorithms optimized for real-time data
- **No SFU required**: Unlike WebRTC, direct client-server connections are simple

### Developer Experience
- **Simpler architecture**: No need for STUN/TURN servers or complex NAT traversal
- **Native async/await**: Clean Rust async code with tokio
- **Bidirectional streams**: Easy request/response and pub/sub patterns
- **Binary protocol**: Efficient bincode serialization without base64 overhead

### Comparison to Alternatives

| Feature | WebTransport | WebRTC | WebSocket |
|---------|-------------|--------|-----------|
| Browser support | Modern browsers | All browsers | All browsers |
| Connection setup | Fast (0-RTT) | Slow (ICE) | Medium (TCP) |
| Multiple streams | Yes (native) | Yes (data channels) | No |
| Head-of-line blocking | No | No | Yes |
| Congestion control | Advanced (QUIC) | Basic | TCP-based |
| Setup complexity | Low | High (STUN/TURN) | Low |
| Real-time performance | Excellent | Good | Fair |
| WASM overhead | None | Moderate | Low |

## Testing Strategy

```rust
#[cfg(test)]
mod tests {
    // Unit tests for protocol serialization
    #[test]
    fn test_handshake_serialization() { }

    // Integration tests for auth
    #[test]
    fn test_signature_verification() { }

    // Integration tests for world operations
    #[test]
    fn test_world_request_response() { }

    // End-to-end tests with WebTransport
    #[tokio::test]
    async fn test_client_server_flow() { }
}
```

## Example Client Usage

### Native Client (Rust)

```rust
use wtransport::ClientConfig;
use glam::IVec3;

// Connect to server
let config = ClientConfig::builder()
    .with_server_url("https://crossworld.atlantislabs.live:4433")
    .build();

let connection = wtransport::Endpoint::client(config)?
    .connect()
    .await?;

// Open bidirectional stream for handshake
let (mut send, mut recv) = connection.open_bi().await?;

// Perform handshake
let handshake = Handshake {
    npub: my_npub.to_string(),
    timestamp: current_timestamp(),
    signature: sign_with_nostr_key(&my_secret_key, &server_url, timestamp),
    display_name: Some("Player1".to_string()),
};

write_message(&mut send, &handshake).await?;
let ack: HandshakeAck = read_message(&mut recv).await?;

println!("Connected with session ID: {}", ack.session_id);

// Request world region using CubeCoord
let request = WorldRequest {
    session_id: ack.session_id,
    coord: CubeCoord {
        pos: IVec3::ZERO,
        depth: 3
    },
    subscribe: true,
};

write_message(&mut send, &request).await?;
let world_data: WorldData = read_message(&mut recv).await?;

// The Cube<u8> is now available for rendering or manipulation
println!("Received cube at {:?}, depth {}",
    world_data.coord.pos, world_data.coord.depth);

// You can use the cube directly with your existing cube crate functions
// For example, generate mesh, raycast, traverse, etc.
use cube::DefaultMeshBuilder;
let mesh = cube::mesh::generate_face_mesh(&world_data.root, &DefaultMeshBuilder);

// Make an edit (requires authorization)
// To edit a voxel, create a modified Cube<u8> and send it
let mut modified_cube = world_data.root.clone();
// ... modify the cube using cube manipulation functions ...

let edit = WorldEdit {
    session_id: ack.session_id,
    operation: EditOperation::SetCube {
        coord: CubeCoord {
            pos: IVec3::new(10, 5, 10),
            depth: 6,
        },
        cube: modified_cube,
    },
    transaction_id: generate_tx_id(),
};

write_message(&mut send, &edit).await?;
let edit_ack: WorldEditAck = read_message(&mut recv).await?;

// Listen for real-time updates
loop {
    let update: WorldUpdate = read_message(&mut recv).await?;
    println!("World updated by {}: {:?}", update.author, update.operation);
}
```

### WASM Client (Browser)

```rust
use wasm_bindgen::prelude::*;
use web_sys::WebTransport;

#[wasm_bindgen]
pub async fn connect_to_server(server_url: String, npub: String) -> Result<(), JsValue> {
    // Create WebTransport connection (browser API)
    let transport = WebTransport::new(&server_url)?;

    // Wait for connection to be ready
    let ready = JsFuture::from(transport.ready()).await?;

    // Open bidirectional stream
    let streams = JsFuture::from(transport.create_bidirectional_stream()).await?;
    let (send, recv) = get_streams_from_js(streams)?;

    // Perform handshake (same as native)
    let handshake = Handshake {
        npub,
        timestamp: current_timestamp(),
        signature: sign_with_nostr_key_wasm(&server_url, timestamp),
        display_name: Some("WebPlayer".to_string()),
    };

    write_message_wasm(&send, &handshake).await?;
    let ack: HandshakeAck = read_message_wasm(&recv).await?;

    // Request world data using CubeCoord
    let request = WorldRequest {
        session_id: ack.session_id,
        coord: CubeCoord {
            pos: IVec3::ZERO,
            depth: 3,
        },
        subscribe: true,
    };

    write_message_wasm(&send, &request).await?;
    let world_data: WorldData = read_message_wasm(&recv).await?;

    // The Cube<u8> is directly usable in WASM
    // You can integrate it with your WorldCube or use it directly for rendering

    // Option 1: Use cube directly for mesh generation
    use cube::DefaultMeshBuilder;
    let mesh = cube::mesh::generate_face_mesh(&world_data.root, &DefaultMeshBuilder);

    // Option 2: If you need to integrate with existing WorldCube
    // Convert to CSM for compatibility (only if needed)
    let csm = cube::serialize_csm(&world_data.root);
    let world_cube = WorldCube::new(3, 0, 0, 0);
    world_cube.set_root(&csm)?;

    Ok(())
}
```

### JavaScript Integration

```javascript
// Import WASM module
import init, { connect_to_server } from './pkg/crossworld_client.js';

async function main() {
  await init();

  const serverUrl = 'https://crossworld.example.com:4433';
  const npub = 'npub1...'; // User's Nostr public key

  try {
    await connect_to_server(serverUrl, npub);
    console.log('Connected to Crossworld server!');
  } catch (e) {
    console.error('Connection failed:', e);
  }
}

main();
```

## Dependencies (Cargo.toml)

### Server Crate

```toml
[package]
name = "crossworld-server"
version = "0.0.1"
edition = "2024"

[dependencies]
# WebTransport (Quinn-based)
wtransport = "0.1"  # WebTransport server using Quinn

# Async runtime
tokio = { version = "1.0", features = ["full"] }

# World management (workspace crate)
cube = { path = "../cube" }

# Serialization
serde = { version = "1.0", features = ["derive"] }
bincode = "1.3"

# Nostr authentication
nostr-sdk = "0.36"

# Logging
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }

# Error handling
thiserror = "2.0"
anyhow = "1.0"

# Configuration
toml = "0.8"

# Utilities
glam = { version = "0.29", features = ["serde"] }

[dev-dependencies]
tokio-test = "0.4"
```

### Client Crate (supports both native and WASM)

```toml
[package]
name = "crossworld-client"
version = "0.0.1"
edition = "2024"

[lib]
crate-type = ["cdylib", "rlib"]

[dependencies]
# WebTransport - platform adaptive
wtransport = { version = "0.1", optional = true }  # For native
web-sys = { version = "0.3", features = ["WebTransport"], optional = true }  # For WASM
wasm-bindgen = { version = "0.2", optional = true }
wasm-bindgen-futures = { version = "0.4", optional = true }

# Async runtime (conditional)
tokio = { version = "1.0", features = ["full"], optional = true }

# World management
cube = { path = "../cube" }

# Serialization
serde = { version = "1.0", features = ["derive"] }
bincode = "1.3"
serde-wasm-bindgen = { version = "0.6", optional = true }

# Nostr authentication
nostr-sdk = { version = "0.36", optional = true }  # Native
# For WASM, use nostr-wasm or implement using web-crypto

# Utilities
glam = { version = "0.29", features = ["serde"] }

# Logging
tracing = "0.1"
tracing-wasm = { version = "0.2", optional = true }

# Error handling
thiserror = "2.0"

[features]
default = ["native"]
native = ["wtransport", "tokio", "nostr-sdk"]
wasm = ["wasm-bindgen", "wasm-bindgen-futures", "web-sys", "serde-wasm-bindgen", "tracing-wasm"]

[target.'cfg(target_arch = "wasm32")'.dependencies]
getrandom = { version = "0.3", features = ["wasm_js"] }
```

## Implementation Checklist

- [ ] Set up server crate with dependencies
- [ ] Implement protocol message types
- [ ] Implement Nostr signature verification
- [ ] Implement WebTransport server initialization
- [ ] Implement handshake protocol
- [ ] Implement world data request/response
- [ ] Implement world editing with authorization
- [ ] Implement broadcast/subscription system
- [ ] Implement rate limiting
- [ ] Implement persistence layer
- [ ] Set up client crate (native)
- [ ] Set up client crate (WASM)
- [ ] Implement client connection logic
- [ ] Add integration tests
- [ ] Add end-to-end tests
- [ ] Performance testing and optimization
- [ ] Documentation and examples
