# Crossworld Project Contents

## Overview

**Crossworld** is a multiplayer voxel-based metaverse running in the browser, combining:

- **Decentralized identity** via Nostr protocol (NIP-53 live events, NIP-01 profiles)
- **Real-time voice communication** via MoQ (Media over QUIC)
- **3D voxel world rendering** using Three.js
- **High-performance voxel engine** written in Rust compiled to WebAssembly
- **Physics simulation** using Rapier3D physics engine
- **Custom voxel avatars** from MagicaVoxel (.vox) files
- **AI-powered 3D generation** via XCube, Trellis, and Robocube integrations

This is a hybrid architecture: **Rust** for performance-critical code (octree operations, mesh generation, physics), **TypeScript/React** for application logic, UI, and networking.

---

## Repository Structure

```
crossworld/
├── crates/                    # Rust workspace (22 crates)
├── packages/                  # TypeScript workspace (3 packages)
├── assets/                    # Static assets (voxel models, textures, palettes)
├── doc/                       # Documentation
├── openspec/                  # OpenSpec specifications
├── specs/                     # Project specifications
├── scripts/                   # Utility scripts
├── Cargo.toml                 # Rust workspace manifest
├── package.json               # Bun workspace manifest
├── justfile                   # Task runner commands
├── flake.nix                  # Nix development environment
└── CLAUDE.md                  # AI assistant instructions
```

---

## Rust Crates (`crates/`)

### Core Engine Crates

| Crate | Description |
|-------|-------------|
| **core** | Core utilities: glam math, glow rendering, winit windowing, gamepad support (gilrs) |
| **cube** | Voxel octree data structure, CSM parser, mesh generation, raycast operations, .vox loading |
| **world** | World terrain generation with multi-depth octree (macro + micro), procedural noise terrain |
| **physics** | Rapier3D physics wrapper with character controllers, collision detection, WASM support |

### Application Crates

| Crate | Description |
|-------|-------------|
| **app** | Application framework with runtime features (glutin, egui, hot-reload support) |
| **app-bevy** | Bevy-based application scaffold |
| **game** | Hot-reload game library with world generation and rendering |
| **renderer** | Native OpenGL renderer with egui integration, raytracing, color systems |

### Editor Crates

| Crate | Description |
|-------|-------------|
| **editor** | Native voxel editor using glow/egui/winit with Nostr login and Lua scripting |
| **editor-bevy** | Bevy-based voxel editor |

### Networking & Identity

| Crate | Description |
|-------|-------------|
| **server** | Game server with WebTransport (QUIC), position broadcasting, anti-cheat, Nostr discovery |
| **test-client** | WebTransport test client for server testing |
| **nostr** | Nostr integration library with NIP-46 signer support |
| **worldtool** | CLI for Nostr event management (NIP-53 live events) |

### AI/3D Generation

| Crate | Description |
|-------|-------------|
| **xcube** | XCube text-to-3D inference client and server wrapper |
| **trellis** | Trellis.2 image-to-3D inference client |
| **robocube** | Roblox Cube3D text-to-3D to voxel format converter |

### Utilities & Prototypes

| Crate | Description |
|-------|-------------|
| **assets** | Asset management tool for JSON generation from voxel models |
| **scripting** | Lua scripting (mlua) and KDL configuration parsing |
| **proto-gl** | Lightweight OpenGL physics prototype viewer |
| **proto-bevy** | Bevy physics prototype with Rapier3D debug rendering |
| **testbed** | Development testbed for physics and rendering experiments |

---

## TypeScript Packages (`packages/`)

### Main Application

| Package | Description |
|---------|-------------|
| **@crossworld/app** | Main web application (Vite + React + Three.js + Chakra UI) |

**Key dependencies:**
- **Three.js** (v0.170) - 3D rendering
- **React** (v18) - UI framework
- **Chakra UI** (v2.10) - Component library
- **Applesauce** (v4) - Nostr client library
- **MoQ** (@kixelated/moq, hang, signals) - Voice chat

**Structure:**
```
packages/app/src/
├── components/         # React UI components
├── renderer/           # Three.js scene management
├── physics/            # Physics world wrapper
├── services/           # Application services (voice, avatar-state)
├── hooks/              # React hooks
├── utils/              # Utilities (WASM wrappers, raycasting)
├── workers/            # Web workers
└── App.tsx             # Main application component
```

### Shared Libraries

| Package | Description |
|---------|-------------|
| **@crossworld/common** | Shared utilities: Nostr integration, accounts, relay settings, types |
| **@crossworld/editor** | Voxel editing UI components using Three.js |

### WASM Packages (Generated)

| Package | Source Crate |
|---------|--------------|
| **wasm-core** | `crates/core` |
| **wasm-cube** | `crates/cube` |
| **wasm-world** | `crates/world` |
| **wasm-physics** | `crates/physics` |

---

## Assets (`assets/`)

| Item | Description |
|------|-------------|
| **avatars.json** | Avatar manifest (name, file path, metadata) |
| **models.json** | Model manifest for voxel objects |
| **materials.json** | Material definitions |
| **models.csv** | Model data in CSV format |
| **models/** | MagicaVoxel .vox files (LFS tracked) |
| **palettes/** | Color palettes |
| **textures/** | Texture atlases |

---

## Development Environment

### Nix Flake (`flake.nix`)

Two development shells:

1. **default** - Standard development (Rust nightly + wasm-pack + bun + just)
2. **cuda** - CUDA-enabled shell for AI inference (XCube, Trellis)

### Key Commands (`justfile`)

```bash
# Development
just dev              # Build WASM + start Vite dev server
just build            # Production build
just check            # Run all checks (Rust + TypeScript)
just test             # Run tests

# Native Applications
just editor           # Run voxel editor (glow/egui)
just editor-release   # Optimized voxel editor
just proto            # Run physics prototype (Bevy)
just proto-gl         # Run physics viewer (OpenGL)

# Server
just server           # Run game server (dev mode)
just server-prod      # Run game server (production)
just test-client      # Run WebTransport test client

# Hot Reload
just game-run         # Run hot-reload app (Terminal 1)
just game-watch       # Watch and rebuild (Terminal 2)

# AI Inference
just xcube-setup      # Set up XCube environment
just xcube-server     # Start XCube inference server
just trellis-setup    # Set up Trellis.2 environment
just trellis-server   # Start Trellis.2 server
just robocube-server  # Start Robocube server

# Nostr
just start-live       # Create Nostr live event
just moq-relay        # Run local MoQ relay
```

---

## Key Technologies

### Rust Stack
- **glam** - Math library (vectors, matrices, quaternions)
- **Rapier3D** - Physics engine
- **dot_vox** - MagicaVoxel .vox parser
- **noise** - Procedural terrain generation
- **nom** - Parser combinator for CSM format
- **wasm-bindgen** - WebAssembly bindings
- **mlua** - Lua scripting
- **kdl** - Configuration format
- **nostr-sdk** - Nostr protocol
- **wtransport** - WebTransport (QUIC)

### TypeScript Stack
- **React** (v18) - UI framework
- **Three.js** (v0.170) - 3D rendering
- **Vite** (v6) - Build tool
- **Bun** - Package manager
- **Chakra UI** - Component library
- **Applesauce** - Nostr client
- **MoQ** - Voice chat protocol

### Native Rendering
- **glow** - OpenGL bindings
- **glutin** - OpenGL context
- **egui** - Immediate mode UI
- **winit** - Windowing
- **Bevy** (v0.17) - Game engine (for prototypes/editor)

---

## Documentation (`doc/`)

| Document | Description |
|----------|-------------|
| **README.md** | Documentation index |
| **QUICKSTART.md** | Getting started guide |
| **PROJECT-STRUCTURE.md** | Detailed repository layout |
| **app.md** | Application documentation |
| **nostr.md** | Nostr event specifications |
| **architecture/** | System architecture docs |
| **features/** | Feature documentation (avatars, voice, etc.) |
| **reference/** | Technical references (build, materials, server) |

---

## Architecture Overview

### Data Flow

```
Rust Crates (cube, world, physics)
       ↓ wasm-pack build
WASM Packages (wasm-cube, wasm-world, wasm-physics)
       ↓ ES module imports
TypeScript App (Three.js rendering, React UI)
       ↓ WebTransport
Game Server (position sync, anti-cheat)
       ↓ Nostr
Discovery (live events, profiles)
```

### Multi-Depth Octree System

- **Macro depth** (0-3): Procedurally generated terrain
- **Micro depth** (4-7): User edits (placed/removed voxels)
- **Total depth**: Up to 7 levels (2^7 = 128 unit resolution)

### CSM (CubeScript Model) Format

Text-based voxel format for octree structures:
```
s[
  o[s1 s2 s0 s0 s0 s0 s0 s0]  // octree node
  s5                           // solid voxel
]
```

---

## Server Architecture

The game server (`crates/server`) provides:

- **WebTransport** (QUIC/HTTP3) for low-latency multiplayer
- **Position broadcasting** at 30 Hz with interest management
- **Anti-cheat validation** (teleport detection, speed limits)
- **Nostr discovery** announcements (NIP-53)

Metrics: connected players, messages sent/received, bytes transferred, position violations.

---

## Build Profiles

| Profile | Description |
|---------|-------------|
| `dev` | Development with debug info |
| `dev-cranelift` | Fast compile with Cranelift backend |
| `fast-dev` | Cranelift + opt-level=1 |
| `release` | Optimized (opt-level=3, LTO) |

---

## License

MIT OR Apache-2.0 (per crate)
