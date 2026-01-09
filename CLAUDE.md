# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

**Crossworld** is a multiplayer voxel-based metaverse running in the browser that combines:
- **Decentralized identity** via Nostr protocol (NIP-53 live events for chat, NIP-01 for profiles)
- **Real-time voice communication** via MoQ (Media over QUIC)
- **3D voxel world rendering** using Three.js
- **High-performance voxel engine** written in Rust compiled to WebAssembly
- **Physics simulation** using Rapier3D physics engine
- **Custom voxel avatars** from MagicaVoxel (.vox) files

This is a hybrid architecture: Rust for performance-critical code (octree operations, mesh generation, physics), TypeScript/React for application logic, UI, and networking.

## Development Commands

All commands use **bun** (not npm/yarn) and are orchestrated via **justfile**:

```bash
# Primary development workflow
just dev              # Build WASM in dev mode + start Vite dev server (http://0.0.0.0:5173)
just build            # Production build (optimized WASM + bundled app)
just check            # Run all checks before deployment (Rust check/clippy/fmt + TypeScript build)

# WASM compilation
just build-wasm       # Build all WASM modules in release mode (cube, world, physics)
just build-wasm-dev   # Build all WASM modules in dev mode (faster compile, larger output)

# Utilities
just install          # Install bun dependencies
just preview          # Preview production build
just clean            # Clean build artifacts (wasm packages, vite cache)
just test             # Run Rust tests + TypeScript type check

# Nostr live event management
just start-live       # Create Nostr live event for Crossworld (uses worldtool CLI)

# MoQ relay (voice chat server)
just moq-relay        # Clone and run local MoQ relay on localhost:4443

# Game server
just gen-cert         # Generate self-signed certificate for development
just server           # Run game server in development mode
just server-prod      # Run game server in production mode with all features
```

## Repository Structure

```
dev-cw/
├─ crates/                    # Rust workspace (compiles to WASM)
│  ├─ cube/                   # Core voxel octree data structure
│  │                          # - CSM (CubeScript Model) parser
│  │                          # - Mesh generation, raycast operations
│  │                          # - .vox (MagicaVoxel) file loading
│  ├─ world/                  # World terrain generation
│  │                          # - Procedural terrain (noise-based)
│  │                          # - Multi-depth octree (macro + micro)
│  ├─ physics/                # Rapier3D physics wrapper
│  │                          # - Character controllers, collision detection
│  ├─ renderer/               # Native OpenGL renderer (desktop only, not used in web)
│  ├─ assets/                 # Asset management tool (JSON generation for avatars)
│  ├─ worldtool/              # CLI for Nostr event management (NIP-53 live events)
│  └─ server/                 # Game server (WebTransport, position broadcasting, anti-cheat)
│
├─ packages/                  # TypeScript workspace (bun workspaces)
│  ├─ wasm-cube/              # WASM output from crates/cube (generated)
│  ├─ wasm-world/             # WASM output from crates/world (generated)
│  ├─ wasm-physics/           # WASM output from crates/physics (generated)
│  ├─ common/                 # Shared utilities (Nostr integration, accounts, relay settings)
│  ├─ editor/                 # Voxel editing UI components
│  └─ app/                    # Main web application (Vite + React + Three.js)
│     ├─ src/renderer/        # Three.js scene management, avatar rendering, camera
│     ├─ src/physics/         # Physics world wrapper (wraps wasm-physics)
│     ├─ src/services/        # Application services (voice, avatar-state, profile-cache)
│     ├─ src/utils/           # Utilities (WASM wrappers, raycasting)
│     └─ src/components/      # React UI components (WorldCanvas, SelectAvatar, ChatPanel)
│
├─ assets/                    # Static assets (.vox files, JSON manifests, textures)
├─ justfile                   # Task runner (build, dev, test commands)
└─ Cargo.toml                 # Rust workspace manifest
```

## Architecture: Rust → WASM → TypeScript

### Data Flow

1. **Rust crates** (`cube`, `world`, `physics`) compiled via `wasm-pack build --target web`
2. **WASM packages** output to `packages/wasm-*` with JavaScript bindings and TypeScript definitions
3. **TypeScript app** imports WASM modules as ES modules:
   ```typescript
   import initCube, { loadCsm, WasmCube } from 'cube';
   import initWorld, { WorldCube } from 'crossworld-world';
   import initPhysics, { WasmPhysicsWorld } from 'crossworld-physics';
   ```

### Key Rust→WASM Exports

- **cube**: `loadCsm()`, `WasmCube`, `generateMesh()`, `raycast()`, `validateCsm()`
- **world**: `WorldCube`, `GeometryData`, `generateFrame()`, `setVoxelAtDepth()`
- **physics**: `WasmPhysicsWorld`, `createCharacter()`, `moveCharacter()`, `step()`

### WASM Initialization Pattern

Each WASM module must be initialized before use:
```typescript
await initCube();      // Loads cube.wasm
await initWorld();     // Loads crossworld-world.wasm
await initPhysics();   // Loads crossworld_physics.wasm
```

## Critical Architecture Concepts

### Multi-Depth Octree System

The voxel world uses a multi-depth octree with distinct purposes:

- **Macro depth** (0-3): Procedurally generated terrain (noise-based, generated once)
- **Micro depth** (4-7): User edits (placed/removed voxels, stored separately)
- **Border depth**: Transition/blending layers between macro and micro
- **Total depth**: Up to 7 levels (2^7 = 128 unit resolution per axis)

Operations:
- `WorldCube::new(macro_depth, micro_depth, border_depth, seed)` - Initialize world
- `setVoxelAtDepth(x, y, z, depth, colorIndex)` - Edit voxel at specific depth
- `generateFrame()` - Generate mesh from octree (returns `GeometryData` with vertices, normals, colors, indices)

### CSM (CubeScript Model) Format

Text-based voxel format for defining octree structures:
```
s[
  o[s1 s2 s0 s0 s0 s0 s0 s0]  // octree node (8 children)
  s5                           // solid voxel (material index 5)
]
```
- Used for avatars and prefab models
- Human-readable and version-control friendly
- Parsed in Rust (`nom` parser combinator)

### Voxel Avatar System

Avatars are loaded from MagicaVoxel `.vox` files:
- Parsed via `dot_vox` crate in WASM
- Per-user color customization (npub hash → HSL hue shift)
- Mesh generation with face culling
- Typical size: ~16x16x30 voxels
- Discovery: Nostr profile tags or local files

### Physics Integration

Uses Rapier3D physics engine wrapped in `wasm-physics`:
```typescript
const world = WasmPhysicsWorld.new({ x: 0, y: -9.81, z: 0 });
const characterHandle = world.createCharacter(position, 1.8, 0.3);

// Each frame:
world.moveCharacter(characterHandle, velocity);
const newPosition = world.getCharacterPosition(characterHandle);
```

## Technology Stack

### Rust Dependencies
- `wasm-bindgen` - Expose Rust functions to JavaScript
- `glam` - Math library (vectors, matrices, quaternions)
- `serde` - Serialization/deserialization
- `dot_vox` - MagicaVoxel .vox file parser
- `noise` - Procedural terrain generation (Perlin/Simplex)
- `rapier3d` - Physics engine
- `nom` - Parser combinator for CSM format

### TypeScript Dependencies
- **React** - UI framework with Chakra UI components
- **Three.js** - 3D rendering engine (BufferGeometry, materials, raycasting)
- **Vite** - Build tool and dev server
- **Bun** - Package manager and runtime (faster than npm)
- **Applesauce** - Nostr client library for React (accounts, relays, events)
- **nostr-tools** - Core Nostr protocol utilities
- **@kixelated/moq** - MoQ protocol for voice chat
- **@kixelated/hang** - High-level audio broadcasting

## Nostr Protocol Integration

### Purpose
- **Decentralized identity**: No central login server (Nostr keypairs)
- **Profile discovery**: Display names, avatars from Nostr profiles (kind 0)
- **Chat system**: NIP-53 live events (kind 30311) for group chat, kind 1311 for messages
- **Presence**: Track who's online in the world

### Worldtool CLI

Creates NIP-53 live events for Crossworld sessions:
```bash
cd crates/worldtool
cargo run -- init-live --streaming https://moq.justinmoon.com/anon

# Or via just:
just start-live
```

This publishes a persistent live event with d-tag `crossworld-dev` to configured Nostr relays.

## MoQ (Media over QUIC) Integration

### Purpose
Real-time, low-latency voice communication without SFU servers.

### Architecture
- **Connection**: WebTransport over QUIC (secure, multiplexed)
- **Broadcasts**: Each user publishes to unique MoQ path: `crossworld/voice/{d-tag}/{npub}`
- **Discovery**: Dual system (Nostr ClientStatusService + MoQ native announcements)
- **Audio**: MediaStream → MoQ broadcast → Remote playback via Web Audio API

### Relay Options
- **Public**: `https://relay.moq.dev/anon`
- **Local dev**: `just moq-relay` (localhost:4443, auto-generated certificates)
- **Custom**: Any MoQ-compatible relay

## Common Development Workflows

### Adding Rust Functionality

1. Edit Rust code in `crates/{cube,world,physics}/`
2. Add `#[wasm_bindgen]` to functions/structs you want to expose
3. Rebuild WASM: `just build-wasm-dev` (fast) or `just build-wasm` (optimized)
4. TypeScript types auto-update in `packages/wasm-*/`
5. Import in TypeScript: `import { MyFunction } from 'cube'`

### Editing Voxel World Logic

1. Modify `WorldCube` in `crates/world/src/lib.rs`
2. Rebuild: `just build-wasm-dev`
3. Update TypeScript usage in `packages/app/src/renderer/scene.ts`
4. Test in dev server: `just dev`

### Adding UI Components

1. Create component in `packages/app/src/components/` or `packages/editor/src/components/`
2. Use Chakra UI for styling (theme customization in `packages/app/src/theme.ts`)
3. Import and use in main app or editor views
4. Hot reload via Vite (no restart needed)

### Debugging WASM

1. Build WASM in dev mode for better stack traces: `just build-wasm-dev`
2. Enable browser developer tools (Console shows WASM panics)
3. Use `web_sys::console::log!()` in Rust for debug logging
4. Check generated JavaScript bindings in `packages/wasm-*/`

## Build and Deployment

### Development Build
```bash
just dev
```
- WASM compiled in dev mode (faster, larger binaries)
- Vite dev server with HMR (Hot Module Replacement)
- Serves on `http://0.0.0.0:5173`

### Production Build
```bash
just build
```
- WASM compiled in release mode (`opt-level = 3`, `lto = true`)
- Vite bundles and optimizes TypeScript
- Output: `packages/app/dist/`

### Pre-Deployment Checks
```bash
just check
```
Runs:
1. `cargo check --workspace`
2. `cargo clippy --workspace -- -D warnings`
3. `cargo fmt --check`
4. `just build-wasm` (release mode)
5. `cd packages/app && bun run build`

## Path Mappings and Imports

TypeScript path mappings in `packages/app/tsconfig.json`:
```json
{
  "paths": {
    "crossworld-world": ["../wasm-world"],
    "cube": ["../wasm-cube"],
    "crossworld-physics": ["../wasm-physics"]
  }
}
```

Workspace dependencies in `package.json`:
```json
{
  "dependencies": {
    "cube": "workspace:*",
    "crossworld-world": "workspace:*",
    "crossworld-physics": "workspace:*",
    "@crossworld/common": "workspace:*",
    "@crossworld/editor": "workspace:*"
  }
}
```

## Asset Management

Assets are stored in `assets/` directory at repository root:
- `assets/avatars/*.vox` - MagicaVoxel avatar models
- `assets/models/*.vox` - Prefab voxel models
- `assets/textures/*.webp` - Texture atlases
- `assets/avatars.json` - Avatar manifest (name, file path, metadata)
- `assets/models.json` - Model manifest
- `assets/materials.json` - Material definitions

During build, Vite plugin copies assets to `dist/assets/` and serves them during development.

## Performance Considerations

### Why Rust/WASM?
1. **Octree operations**: Recursive tree traversal is CPU-intensive
2. **Mesh generation**: Thousands of faces per frame, needs optimization
3. **Raycast**: Fast spatial queries for voxel editing
4. **Physics**: Rapier3D provides 60+ FPS on complex scenes
5. **Memory efficiency**: Octree compression, shared memory between Rust and JavaScript

### Optimization Techniques
- **Face culling**: Only render visible faces (internal faces omitted)
- **Octree merging**: Combine uniform regions into single nodes
- **Lazy generation**: Generate mesh only when octree modified
- **BufferGeometry**: Efficient Three.js mesh storage
- **Release mode**: Always use `just build-wasm` for production

## Vibe-Kanban Review Workflow

When completing a vibe-kanban task, use the review workflow to get user approval before marking work as done.

### Review Process

1. **Prepare for Review**
   - Ensure all changes are committed
   - Run checks: `just check`
   - Update task status: `mcp__vibe_kanban__update_task(task_id, status: "inreview")`

2. **Generate Review Document**
   Create `doc/review/current.md` using the template at `doc/templates/review.md`

3. **Launch Review**
   ```bash
   cargo run --bin testbed -- --review doc/review/current.md
   ```

4. **Parse and Execute Response**
   The response contains one or more commands (one per line). You MUST execute ALL commands.

### Response Commands

| Command | Example | Action |
|---------|---------|--------|
| `APPROVE` | `APPROVE` | Mark task done |
| `CONTINUE` | `CONTINUE: add error handling` | Update status to inprogress, implement feedback |
| `SPAWN` | `SPAWN: Fix related bug in module X` | Create new task |
| `DISCARD` | `DISCARD` | Cancel task, discard changes |
| `REBASE` | `REBASE` | Rebase onto main |
| `MERGE` | `MERGE` | Merge branch to main |
| `COMMENT` | `COMMENT: Good progress so far` | Record comment |

### Command Validation Rules

1. **At least one command required** - Empty response is invalid
2. **Mutually exclusive statuses** - Cannot combine `APPROVE` + `DISCARD` or `APPROVE` + `CONTINUE`
3. **Order matters** - Commands executed in order (e.g., `REBASE` before `MERGE`)
4. **Multiple `SPAWN` allowed** - Can create multiple follow-up tasks
5. **`COMMENT` is additive** - Can combine with any other command

### Executing Commands

For each command in the response, execute the corresponding action:

| Command | Required Agent Actions |
|---------|------------------------|
| `APPROVE` | `mcp__vibe_kanban__update_task(task_id, status: "done")` |
| `CONTINUE` | `mcp__vibe_kanban__update_task(task_id, status: "inprogress")`, incorporate feedback |
| `SPAWN` | `mcp__vibe_kanban__create_task(project_id, title: "<spawn argument>")` for each SPAWN |
| `DISCARD` | `mcp__vibe_kanban__update_task(task_id, status: "cancelled")`, `git checkout main`, delete branch |
| `REBASE` | `git fetch origin main && git rebase origin/main` |
| `MERGE` | `git checkout main && git merge <branch> --no-ff && git push origin main` |
| `COMMENT` | Log comment, optionally add to task description |

### Example Response Handling

Response from reviewer:
```
APPROVE
SPAWN: Add integration tests
SPAWN: Update README with new feature
MERGE
```

Agent must execute:
1. `mcp__vibe_kanban__update_task(task_id, status: "done")`
2. `mcp__vibe_kanban__create_task(project_id, title: "Add integration tests")`
3. `mcp__vibe_kanban__create_task(project_id, title: "Update README with new feature")`
4. `git checkout main && git merge <branch> --no-ff && git push origin main`

### Example Complete Workflow

```
# 1. Agent completes work and runs checks
$ just check
✓ All checks passed

# 2. Update task status to inreview
mcp__vibe_kanban__update_task(task_id="abc123", status="inreview")

# 3. Create review document at doc/review/current.md

# 4. Launch review
$ cargo run --bin testbed -- --review doc/review/current.md

# 5. User reviews and responds with commands

# 6. Agent parses response and executes all commands
```

## Common Issues and Solutions

### WASM not found or import errors
- Ensure WASM is built: `just build-wasm-dev`
- Check `packages/wasm-*/` directories exist and contain `.wasm`, `.js`, `.d.ts` files
- Restart dev server after rebuilding WASM

### TypeScript errors after Rust changes
- Rebuild WASM to regenerate TypeScript definitions: `just build-wasm-dev`
- Check `packages/wasm-*/*.d.ts` for updated types

### Physics not working
- Verify `crossworld-physics` is initialized: `await initPhysics()`
- Check character controller is created before `moveCharacter()`
- Ensure `step()` is called each frame (typically in animation loop)

### Voice chat connection issues
- Check MoQ relay is running: `just moq-relay` for local testing
- Verify WebTransport is supported (Chrome/Edge 97+, not Safari/Firefox yet)
- Check browser console for WebTransport/QUIC errors
- Confirm microphone permissions granted

### Asset loading failures
- Check asset path format: `/crossworld/assets/avatars/filename.vox`
- Verify file exists in `assets/` directory
- Check Vite plugin in `packages/app/vite.config.ts` is copying assets
- For Nostr-hosted assets, check CORS headers and URL validity

## Game Server

### Overview

The game server (`crates/server`) is a standalone Rust application that provides authoritative multiplayer functionality using WebTransport (QUIC/HTTP3). It manages the shared world state, broadcasts player positions, validates actions, and handles Nostr discovery announcements.

### Server Architecture

```
┌─────────────────────────────────────────────────────────┐
│                 Game Server (Rust Native)                │
├─────────────────────────────────────────────────────────┤
│  WebTransport Endpoint (QUIC/HTTP3)                      │
│  ├── Connection Manager (bidirectional streams)         │
│  ├── Message Router (reliable + unreliable)             │
│  └── Rate Limiter (per-player token buckets)            │
├─────────────────────────────────────────────────────────┤
│  Game World State                                        │
│  ├── Player Registry (DashMap: hex_id -> Player)        │
│  ├── Position Validation (anti-cheat)                   │
│  └── Spatial Queries (interest management)              │
├─────────────────────────────────────────────────────────┤
│  Broadcasting System                                     │
│  ├── Position Aggregator (30 Hz broadcast)              │
│  ├── Interest Management (radius-based filtering)       │
│  └── Batch Compression (datagram optimization)          │
├─────────────────────────────────────────────────────────┤
│  Discovery Announcer (optional)                          │
│  └── Nostr Client (NIP-53 live events)                  │
└─────────────────────────────────────────────────────────┘
```

### Running the Server

```bash
# Development mode (with auto-generated certificate)
just server

# Production mode (requires proper certificates and configuration)
just server-prod

# Generate self-signed certificate manually
just gen-cert

# Custom configuration
cargo run --bin server -- \
  --bind 0.0.0.0:4433 \
  --cert /path/to/cert.pem \
  --key /path/to/key.pem \
  --max-players 100 \
  --interest-radius 200 \
  --validate-positions \
  --enable-discovery \
  --relays wss://relay.damus.io,wss://nos.lol \
  --log-level info
```

### Message Protocol

The server uses two message channels:

**Reliable Messages (Bidirectional Streams)**
- `Join`: Player joins with npub, display name, avatar, initial position
- `Leave`: Player disconnects gracefully
- `ChatMessage`: Chat messages (validated, max 1000 chars)
- `GameEvent`: Game-specific events (voxel edits, etc.)
- `ServerCommand`: Server responses and commands
- `Kick`: Force disconnect with reason

**Unreliable Messages (Datagrams)**
- `Position`: Single player position update (x, y, z, rotation quaternion, sequence number)
- `Batch`: Bulk position updates for multiple players (interest-managed)
- `Ping`/`Pong`: Latency measurement

### Position Broadcasting

The server runs a 30 Hz position broadcast loop (configurable via `--broadcast-rate`):

1. Collect all player positions from `GameServer.players`
2. For each connected client:
   - Apply interest management: filter players within `--interest-radius` (if enabled)
   - Limit to `--max-visible-players` (default: 100)
   - Create `UnreliableMessage::Batch` with relevant positions
   - Send via datagram (UDP-like, no delivery guarantee)

This design minimizes bandwidth while ensuring players only receive updates for nearby players.

### Anti-Cheat Validation

When `--validate-positions` is enabled, the server validates every position update:

**Checks performed:**
- **Teleport detection**: Reject moves exceeding `--teleport-threshold` (default: 50 units)
- **Speed limit**: Reject moves faster than `--max-move-speed` (default: 20 units/sec)
- **World boundaries**: Reject positions outside `--world-size` (default: 1000 units)
- **Violation tracking**: Count violations per player (kick after threshold)

**Violation handling:**
- Warnings logged for first 5 violations
- Rubber-banding (force position sync) for 5-10 violations
- Auto-kick after 10+ violations

### Nostr Discovery

When `--enable-discovery` is set, the server announces itself to Nostr relays every 60 seconds:

**NIP-53 Live Event (kind 30311)**
```json
{
  "content": {
    "game": "crossworld",
    "endpoint": "127.0.0.1:4433",
    "name": "Crossworld Dev Server",
    "region": "local",
    "players": 5,
    "max_players": 100,
    "version": "0.1.0",
    "features": ["webtransport", "nostr-auth"]
  },
  "tags": [
    ["d", "crossworld-server"],
    ["g", "crossworld"],
    ["status", "live"],
    ["participants", "5"]
  ]
}
```

Clients can discover servers by subscribing to these events on Nostr relays.

### Server Configuration

**Network:**
- `--bind`: Bind address (default: `127.0.0.1:4433`)
- `--cert`: TLS certificate path (PEM format)
- `--key`: Private key path (PEM format)

**Game:**
- `--max-players`: Maximum concurrent players (default: 10)
- `--world-size`: World boundary size in units (default: 1000.0)
- `--broadcast-rate`: Position broadcast frequency in Hz (default: 30)

**Performance:**
- `--interest-radius`: Distance for interest management in units (0 = disabled, default: 0)
- `--max-visible-players`: Maximum players to broadcast per client (default: 100)

**Anti-Cheat:**
- `--validate-positions`: Enable position validation (default: false in dev)
- `--max-move-speed`: Maximum movement speed in units/sec (default: 20.0)
- `--teleport-threshold`: Distance threshold for teleport detection (default: 50.0)

**Discovery:**
- `--enable-discovery`: Enable Nostr announcements (default: false)
- `--relays`: Comma-separated Nostr relay URLs
- `--server-name`: Server display name
- `--server-region`: Server region identifier

**Logging:**
- `--log-level`: trace, debug, info, warn, error (default: info)

### Metrics

The server tracks metrics accessible via logs (every 30 seconds):

- `connected_players`: Current player count
- `messages_sent`: Total messages sent
- `messages_received`: Total messages received
- `bytes_sent`: Total bytes sent
- `bytes_received`: Total bytes received
- `position_violations`: Total anti-cheat violations

Metrics can be exposed in Prometheus format via `ServerMetrics::to_prometheus()` (for future HTTP endpoint).

### TLS Certificates

WebTransport requires HTTPS/TLS certificates. For development:

```bash
just gen-cert  # Auto-generates localhost.pem and localhost-key.pem
```

For production, use proper certificates from:
- Let's Encrypt (via certbot)
- Cloud provider certificate manager
- Self-signed certificates (requires client trust)

**Certificate requirements:**
- Must be in PEM format
- Private key must be unencrypted
- Certificate must match the server's domain/IP

### Client Integration

To connect a client to the server:

```typescript
// TypeScript client example
const url = `https://127.0.0.1:4433`;
const transport = new WebTransport(url);

await transport.ready;

// Send reliable message (join)
const writer = await transport.createBidirectionalStream();
const joinMsg = {
  Join: {
    npub: "npub1...",
    display_name: "Player1",
    avatar_url: "https://...",
    position: [0, 10, 0]
  }
};
await writer.write(bincode.serialize(joinMsg));

// Send unreliable position updates
const datagramWriter = transport.datagrams.writable.getWriter();
const posMsg = {
  Position: {
    x: 1.0, y: 10.0, z: 2.0,
    rx: 0, ry: 0, rz: 0, rw: 1,
    seq: 123
  }
};
await datagramWriter.write(bincode.serialize(posMsg));

// Receive position batches
const datagramReader = transport.datagrams.readable.getReader();
while (true) {
  const { value, done } = await datagramReader.read();
  if (done) break;
  const batch = bincode.deserialize(value);
  // Handle batch.positions
}
```

**Note:** Clients need to implement their own WebTransport connection logic. The browser client code is in `packages/app/src/services/` (future implementation).

### Performance Characteristics

**Scalability:**
- Single server instance handles 100-500 players (depending on hardware)
- Interest management reduces bandwidth linearly with radius
- Position broadcast rate trades off latency vs bandwidth (30 Hz recommended)

**Bandwidth per player (30 Hz broadcast, 100 visible players):**
- Position update: ~40 bytes
- Batch of 100 players: ~4 KB
- Total bandwidth: ~120 KB/sec per client (960 Kbps)

**Optimizations:**
- DashMap for concurrent player access (lock-free reads)
- Batch datagrams to minimize QUIC overhead
- Interest management filters distant players
- No world persistence (stateless restarts)

### Deployment

**Docker deployment:**
```dockerfile
FROM rust:1.75 as builder
WORKDIR /app
COPY . .
RUN cargo build --release --bin server

FROM debian:bookworm-slim
COPY --from=builder /app/target/release/server /usr/local/bin/server
COPY cert.pem key.pem /etc/certs/
EXPOSE 4433/udp
CMD ["server", "--bind", "0.0.0.0:4433", "--cert", "/etc/certs/cert.pem", "--key", "/etc/certs/key.pem"]
```

**Systemd service:**
```ini
[Unit]
Description=Crossworld Game Server
After=network.target

[Service]
Type=simple
User=crossworld
ExecStart=/usr/local/bin/server --bind 0.0.0.0:4433 --cert /etc/certs/cert.pem --key /etc/certs/key.pem --max-players 100 --validate-positions
Restart=on-failure

[Install]
WantedBy=multi-user.target
```

### Development Workflow

When developing server features:

1. Make changes in `crates/server/src/`
2. Build and run: `just server` or `cargo run --bin server`
3. Server automatically generates certificate if missing
4. Connect client to `https://127.0.0.1:4433`
5. Check logs for connection and message events
6. Use `--log-level debug` for detailed tracing

### Future Enhancements

Based on `doc/reference/server.md`, planned features include:

- **World persistence**: Save/load world state from disk or database
- **Client authentication**: Verify Nostr signatures on join
- **Voxel synchronization**: Broadcast world edits to clients
- **Chunk streaming**: Send world regions on demand
- **Physics simulation**: Server-side physics for authoritative movement
- **HTTP metrics endpoint**: Prometheus metrics on `/metrics`
- **Multi-region support**: Automatic region selection based on latency
- **Voice integration**: Coordinate MoQ voice chat via server
