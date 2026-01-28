# Crossworld Project Structure

A compact reference guide to the Crossworld repository organization.

## Root Directory

```
crossworld/
├── crates/           # Rust workspace (15 crates)
├── packages/         # TypeScript workspace (bun workspaces)
├── assets/           # Static assets (.vox, .glb, manifests)
├── doc/              # Technical documentation
├── docs/             # Additional docs (this file)
├── openspec/         # Change proposal system
├── scripts/          # Build and utility scripts
├── specs/            # Feature specifications
│
├── Cargo.toml        # Rust workspace manifest
├── package.json      # Bun workspace root
├── justfile          # Task runner (primary build interface)
├── flake.nix         # Nix development environment
└── CLAUDE.md         # AI assistant guidance
```

## Rust Crates (`crates/`)

### WASM Modules (Browser)

| Crate | Output | Purpose |
|-------|--------|---------|
| **cube** | `wasm-cube` | Core voxel octree engine, CSM parser, mesh generation, raycasting, .vox loader |
| **world** | `wasm-world` | World terrain generation, multi-depth octree (macro/micro), procedural terrain |
| **physics** | `wasm-physics` | Rapier3D physics wrapper, character controllers, collision detection |

### Native Applications

| Crate | Binary | Purpose |
|-------|--------|---------|
| **server** | `server` | Game server - WebTransport multiplayer, position broadcasting, anti-cheat |
| **worldtool** | `worldtool` | CLI for Nostr live events (NIP-53), MoQ relay setup |
| **test-client** | `test-client` | Server testing client |
| **editor** | - | Native voxel editor (Bevy-based) |
| **app** | `hot-reload-app` | Hot-reload development app |
| **app-bevy** | - | Bevy-based application prototype |

### Libraries & Prototypes

| Crate | Purpose |
|-------|---------|
| **game** | Game logic library (hot-reloadable) |
| **assets** | Asset management tool, JSON generation for avatars |
| **renderer** | CPU raytracer (experimental, desktop-only) |
| **proto-gl** | OpenGL rendering prototype |
| **proto-bevy** | Bevy physics prototype |
| **testbed** | Testing and experimentation |

## TypeScript Packages (`packages/`)

### Application Packages

```
packages/
├── app/              # Main web application
│   ├── src/
│   │   ├── components/   # React UI (WorldCanvas, ChatPanel, SelectAvatar)
│   │   ├── renderer/     # Three.js (scene, camera, avatar rendering)
│   │   ├── physics/      # Physics world wrapper (wasm-physics bridge)
│   │   ├── services/     # Voice chat, avatar state, profile cache
│   │   └── utils/        # WASM wrappers, raycasting helpers
│   └── vite.config.ts
│
├── common/           # Shared utilities (Nostr integration, accounts, relays)
└── editor/           # Voxel editing UI components
```

### Generated WASM Packages (DO NOT EDIT)

```
packages/
├── wasm-cube/        # From crates/cube via wasm-pack
├── wasm-world/       # From crates/world via wasm-pack
└── wasm-physics/     # From crates/physics via wasm-pack
```

Each contains: `.wasm` binary, `.js` bindings, `.d.ts` type definitions

## Assets (`assets/`)

```
assets/
├── models/
│   ├── vox/          # MagicaVoxel avatars/objects (~100+ .vox files)
│   └── glb/          # GLTF models (man.glb, etc.)
├── avatars.json      # Avatar manifest (name, file, metadata)
├── models.json       # Model registry
├── models.csv        # Model metadata spreadsheet
├── materials.json    # Voxel material definitions
└── about.md          # Asset attribution
```

## Documentation

### Technical Docs (`doc/`)

```
doc/
├── README.md             # Documentation index
├── QUICKSTART.md         # Voice chat setup guide
├── nostr.md              # Nostr event specifications
│
├── architecture/         # System design
│   ├── overview.md       # High-level architecture
│   ├── voxel-system.md   # Octree and CSM format
│   ├── physics.md        # Rapier3D integration
│   ├── rendering.md      # Three.js pipeline
│   ├── raycast.md        # Ray-octree intersection
│   ├── bcf-format.md     # Binary chunk format
│   └── cubeworld-collision.md
│
├── features/             # Feature documentation
│   ├── avatar-system.md  # Avatar design
│   ├── voice-chat.md     # MoQ voice (Media over QUIC)
│   ├── nostr-integration.md
│   └── trellis-voxels.md
│
└── reference/            # Technical references
    ├── project-structure.md
    ├── build-system.md
    ├── materials.md
    └── server.md
```

## Build Workflow

### Quick Commands

```bash
just dev              # WASM (dev) + Vite server (http://0.0.0.0:5173)
just build            # Production build (optimized WASM + bundled app)
just check            # Pre-deployment checks (Rust + TypeScript)
just server           # Run game server (development)
just editor           # Run native voxel editor (Bevy)
```

### WASM Compilation Flow

```
┌─────────────────┐     wasm-pack      ┌──────────────────┐
│ crates/cube     │ ──────────────────→│ packages/wasm-cube│
│ crates/world    │ ──────────────────→│ packages/wasm-world│
│ crates/physics  │ ──────────────────→│ packages/wasm-physics│
└─────────────────┘                    └──────────────────┘
                                               │
                                        ES module imports
                                               ↓
                                       ┌──────────────────┐
                                       │ packages/app     │
                                       │ (Vite + React)   │
                                       └──────────────────┘
```

### Full Build Pipeline

```bash
# Development
just build-wasm-dev   # Fast WASM compile (larger binaries)
cd packages/app && bun run dev  # Vite dev server with HMR

# Production
just build-wasm       # Optimized WASM (opt-level=3, lto=true)
cd packages/app && bun run build  # Bundle to dist/
```

## Architecture Overview

### Hybrid Rust + TypeScript

```
┌─────────────────────────────────────────────────────────────┐
│                    Browser (TypeScript)                      │
├─────────────────────────────────────────────────────────────┤
│  React UI  │  Three.js Renderer  │  MoQ Voice  │  Nostr    │
├─────────────────────────────────────────────────────────────┤
│                 WASM Modules (Rust → WebAssembly)           │
│  ┌──────────┐  ┌─────────────┐  ┌────────────────┐         │
│  │ wasm-cube│  │ wasm-world  │  │ wasm-physics   │         │
│  │ Octree   │  │ Terrain Gen │  │ Rapier3D       │         │
│  │ Mesh Gen │  │ Multi-depth │  │ Character Ctrl │         │
│  └──────────┘  └─────────────┘  └────────────────┘         │
└─────────────────────────────────────────────────────────────┘
                              │
                    WebTransport (QUIC)
                              ↓
┌─────────────────────────────────────────────────────────────┐
│                    Game Server (Rust Native)                 │
│  Position Broadcasting │ Anti-Cheat │ Nostr Discovery       │
└─────────────────────────────────────────────────────────────┘
```

### Key Data Flows

1. **Voxel World**: Rust octree → WASM mesh generation → Three.js BufferGeometry
2. **Physics**: WASM Rapier3D → Character controller → Position updates
3. **Multiplayer**: WebTransport datagrams → Server broadcast → Client interpolation
4. **Voice**: Microphone → MoQ publish → Relay → MoQ subscribe → Web Audio

## Path Mappings

### TypeScript Imports

```typescript
// WASM modules (via tsconfig paths)
import initCube from 'cube';
import { WorldCube } from 'crossworld-world';
import { WasmPhysicsWorld } from 'crossworld-physics';

// Workspace packages
import { ... } from '@crossworld/common';
import { ... } from '@crossworld/editor';
```

### Workspace Configuration

```json
// package.json (root)
{ "workspaces": ["packages/*"] }

// packages/app/package.json
{
  "dependencies": {
    "cube": "workspace:*",
    "crossworld-world": "workspace:*",
    "crossworld-physics": "workspace:*",
    "@crossworld/common": "workspace:*"
  }
}
```

## Related Documentation

- [CLAUDE.md](../CLAUDE.md) - Comprehensive project guide for AI assistants
- [doc/architecture/overview.md](../doc/architecture/overview.md) - System architecture
- [doc/reference/build-system.md](../doc/reference/build-system.md) - Build process details
- [doc/reference/server.md](../doc/reference/server.md) - Game server design
