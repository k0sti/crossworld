# 3D Game Implementation Specification

## Project Configuration

### Technology Stack
- **Networking**: Rust + WebTransport (`web-transport` crate v0.7)
- **Identity/Social**: TypeScript + Applesauce (Nostr)
- **Serialization**: bincode (binary), serde_json (discovery only)
- **WASM Target**: `wasm-pack` with `--target web`

### Project Structure
See [project-structure.md](./project-structure.md) for current project organization.

## Core Design Decisions

### Architecture
- **Single World**: No rooms/lobbies, all players share one world space
- **Hybrid Transport**: Unreliable datagrams for position/rotation, reliable streams for events
- **Identity**: Nostr public keys (npub/hex) as player IDs
- **Discovery**: Nostr for server/player discovery, WebTransport for game traffic

### Development Mode
- **Direct Connection**: Skip discovery, connect to `https://localhost:4433`
- **TLS Required**: Use mkcert or self-signed certificates (WebTransport requirement)
- **Environment Variable**: `GAME_SERVER=https://localhost:4433` overrides discovery

## Data Types

```rust
// Player identification
pub struct PlayerIdentity {
    pub npub: String,         // Bech32 format (for display/Nostr)
    pub hex: String,          // Hex format (REQUIRED in binary protocol)
    pub display_name: String,
    pub avatar_url: Option<String>,
}
// Note: All binary protocol messages MUST use hex format for pubkey, not npub

// 3D transforms
pub struct Vec3 { pub x: f32, pub y: f32, pub z: f32 }
pub struct Quaternion { pub x: f32, pub y: f32, pub z: f32, pub w: f32 }

// Animation states (u8 for efficiency)
pub enum AnimationState {
    Idle = 0,
    Walking = 1,
    Running = 2,
    Jumping = 3,
    Falling = 4,
    Custom(u8), // 5-255 for game-specific
}

// Button bitfield (u16)
Jump = 1<<0, Action1 = 1<<1, Action2 = 1<<2, Sprint = 1<<3, // etc.
```

## Network Protocol

### Reliable Messages (Streams)
```rust
pub enum ReliableMessage {
    Join { player: PlayerIdentity, position: Vec3 },
    Leave { player_id: String },
    PlayerSpawned { player: PlayerIdentity, position: Vec3 },
    ChatMessage { from: String, content: String, timestamp: f64 },
    GameEvent { event_type: String, data: serde_json::Value },
}
```

### Unreliable Messages (Datagrams)
```rust
// Use short field names for bandwidth efficiency
pub enum UnreliableMessage {
    #[serde(rename = "p")]
    Position {
        #[serde(rename = "i")] id: String,      // player pubkey in HEX format
        #[serde(rename = "x")] x: f32,
        #[serde(rename = "y")] y: f32,
        #[serde(rename = "z")] z: f32,
        #[serde(rename = "rx")] rx: f32,        // quaternion
        #[serde(rename = "ry")] ry: f32,
        #[serde(rename = "rz")] rz: f32,
        #[serde(rename = "rw")] rw: f32,
        #[serde(rename = "s")] seq: u32,
    },
    #[serde(rename = "b")]
    Batch { positions: Vec<CompactPosition>, timestamp: f64 },
}
```

## WASM Exports

```rust
#[wasm_bindgen]
pub struct NetworkClient {
    // Internal state with Arc<RwLock<T>> for thread safety
}

#[wasm_bindgen]
impl NetworkClient {
    #[wasm_bindgen(constructor)]
    pub fn new() -> NetworkClient;
    
    pub async fn connect(
        &mut self,
        server_url: String,
        npub: String,
        display_name: String,
        avatar_url: Option<String>,
        initial_x: f32, initial_y: f32, initial_z: f32,
    ) -> Result<(), JsValue>;
    
    pub fn send_position(
        &self,
        x: f32, y: f32, z: f32,
        rx: f32, ry: f32, rz: f32, rw: f32,
    );
    
    pub async fn send_chat(&self, message: String) -> Result<(), JsValue>;
    
    // Callbacks (all optional)
    pub fn on_player_joined(&mut self, callback: js_sys::Function);
    pub fn on_position_update(&mut self, callback: js_sys::Function);
    pub fn on_chat_message(&mut self, callback: js_sys::Function);
}
```

## TypeScript Interface

```typescript
// Network wrapper
class NetworkManager {
    private client: NetworkClient;
    
    async connect(serverUrl: string, identity: NostrIdentity): Promise<void> {
        // In dev: serverUrl from env var
        // In prod: serverUrl from discovery
        await this.client.connect(
            serverUrl,
            identity.npub,
            identity.displayName,
            identity.avatar,
            0, 0, 0  // spawn position
        );
    }
    
    sendPosition(transform: Transform): void {
        this.client.send_position(
            transform.position.x, transform.position.y, transform.position.z,
            transform.rotation.x, transform.rotation.y, 
            transform.rotation.z, transform.rotation.w
        );
    }
}
```

## Required External Knowledge

The implementation assumes familiarity with:
- WebTransport API basics
- Nostr event structure (NIP-01, NIP-33)
- WASM compilation with wasm-pack
- TypeScript/JavaScript async patterns

## Not Included

- Game engine integration (Three.js, Babylon, etc.)
- Avatar rendering and interpolation
- Voice chat implementation
- Persistence/save system
- Matchmaking/skill rating
- Anti-cheat measures