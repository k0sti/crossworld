# Project Context

## Purpose
Crossworld is a prototyping platform for a Nostr-based metaverse. It provides a web-based 3D voxel world with decentralized identity, voice chat, and real-time physics. The project combines Rust (compiled to WebAssembly) for high-performance core systems with TypeScript/React for the user interface.

## Tech Stack

### Backend (Rust → WebAssembly)
- **Rust Edition**: 2021 (cube/physics), 2024 (world)
- **WASM Compilation**: wasm-pack with web target
- **Physics Engine**: Rapier3D (git version for latest features)
- **Voxel System**: Custom octree-based cube engine with CSM format support
- **Math Library**: glam 0.29 with serde support
- **Serialization**: serde with wasm-bindgen integration
- **Logging**: tracing + tracing-wasm

### Frontend (TypeScript/React)
- **Runtime**: Bun (for all build tasks per global conventions)
- **Framework**: React 18 with TypeScript 5.8
- **Build Tool**: Vite 6
- **UI Framework**: Chakra UI 2.10.9 with Emotion
- **3D Rendering**: Three.js 0.170
- **Routing**: React Router DOM 7.1.3
- **Nostr SDK**: Applesauce 4.0 (accounts, core, react, relay, signers)
- **Voice/Streaming**: MoQ (Media over QUIC) - @kixelated packages
- **Utilities**: nostr-tools 2.16.2, react-icons, react-markdown

### Infrastructure
- **Task Runner**: just (justfile for common tasks)
- **Package Manager**: Bun (workspace-based monorepo)
- **Version Control**: Git (currently on 'planet' branch, main branch for PRs)

## Project Conventions

### Code Style

#### File Naming
- **React Components**: PascalCase.tsx (e.g., SelectAvatar.tsx)
- **Utilities/Services**: kebab-case.ts (e.g., avatar-state.ts)
- **Types**: Co-located with implementation or in dedicated types/ directory

#### Import Organization
Groups in order:
1. External dependencies (React, Three.js, etc.)
2. Internal absolute imports (from src/)
3. Relative imports (from ./ or ../)

#### Numeric Values with Units
Always include units in variable names for clarity:
```typescript
const timeout_ms = 5000;
const distance_m = 10.5;
const angle_rad = Math.PI / 4;
```

#### Logging
- Use console logging with prefixes for easy filtering (e.g., `[AvatarState]`)
- Include relevant context in error messages

### Architecture Patterns

#### Data Validation Philosophy: Reset over Migration
- **Early Development Stage**: Data formats change frequently
- **No Legacy Mappings**: Don't maintain mappings of old identifiers to new ones
- **Fail Fast**: If data doesn't match expected format, reset to initial state
- **Simple Validation**: Check current valid formats only
- **User Impact**: Acceptable to reset user selections during active development

#### State Management
- **Local Component State**: useState for UI-only state
- **Props**: For parent-child communication
- **Services**: For shared business logic and state
- **Avoid Global State**: Unless absolutely necessary

#### Coordinate System
- **Unified Centered System**: All components use the same world space coordinate system
- **Phased Initialization**: Parallel loading with dependency management

#### Component Architecture
- Pure functions where possible (easier to test)
- Dependency injection for testability
- Atomic and predictable state updates
- Avoid deep object mutations; prefer creating new objects

### Testing Strategy

#### Rust Crates
- **Comprehensive Test Suite**: 33 raycast tests in cube crate (all passing)
- **Run Tests**: `cargo test --workspace` or `just test`

#### TypeScript/React
- **Type Checking**: Strict TypeScript compilation
- **Build Validation**: `bun run build` (includes type checking)
- **Manual Testing**: Test features through UI during development

#### Pre-Deployment Checks
Use `just check` which runs:
- `cargo check --workspace`
- `cargo clippy --workspace -- -D warnings`
- `cargo fmt --check`
- WASM build
- TypeScript build

### Git Workflow

#### Branch Strategy
- **Main Branch**: `main` (use for PRs)
- **Current Branch**: `planet`
- **Repository Status**: Clean working directory

#### Recent Work
- Avatar persistence and initialization improvements
- Removal of excessive logging
- WebGL 3D texture error fixes
- World panel settings centralization
- Avatar selector UI improvements

## Domain Context

### Nostr Integration
- **Identity System**: Nostr public keys (npub/hex) as player IDs
- **Discovery**: Nostr events for server/player discovery
- **Chat**: Decentralized chat using Nostr events
- **Live Events**: NIP-33 replaceable events for world configuration
- **Tool**: worldtool CLI for managing Nostr live events

### Voxel System (Cube Crate)
- **Octree Structure**: Hierarchical voxel storage with configurable depths
- **Raycasting**: High-performance ray-cube intersection
- **Mesh Generation**: Optimized geometry for rendering
- **CSM Format**: Custom Cube Script Model format for voxel data
- **Palette Support**: Color palette system for models

### Physics System
- **Engine**: Rapier3D physics simulation
- **Voxel Collision**: Custom collision detection for voxel geometry
- **Character Controller**: Avatar movement and collision
- **Real-time Simulation**: Integrated with rendering loop

### Voice Chat (MoQ)
- **Protocol**: Media over QUIC (MoQ) for spatial audio
- **Relay Server**: Local or public MoQ relay
- **WebTransport**: Browser-native WebTransport API
- **Location Tracking**: Spatial audio based on avatar positions
- **Setup Tool**: worldtool CLI can manage MoQ relay server

### Rendering Pipeline
- **Three.js Integration**: 3D scene management
- **Dual Rendering**: GPU (Three.js) + CPU (custom raytracer)
- **Shaders**: Custom shader system for voxel rendering
- **Post-Processing**: Effects pipeline

## Important Constraints

### Technical
- **WASM Target**: Must compile to wasm32-unknown-unknown
- **WebTransport Requirement**: TLS certificates required (mkcert for dev)
- **Browser Compatibility**: Modern browsers with WebTransport support
- **Build Time**: WASM compilation can take 2-3 minutes in release mode
- **Getrandom**: Required for WASM builds (transitive dependency)

### Development
- **Bun Required**: All npm operations must use bun (per global conventions)
- **Just Tool**: Task automation uses justfile
- **OpenSpec System**: Use for change proposals and architectural decisions
- **No Documentation Proactivity**: Don't create markdown docs unless requested

### Performance
- **Release Optimization**: opt-level = 3, lto = true
- **Development Mode**: Separate WASM dev builds for faster iteration
- **Parallel Builds**: Independent WASM crates build in parallel

## External Dependencies

### Required Services
- **MoQ Relay**: For voice chat functionality
  - Local: `just moq-relay` (auto-clones kixelated/moq)
  - Public: https://moq.justinmoon.com/anon (fallback)
- **Nostr Relays**: For identity and discovery
  - Configured via Applesauce SDK

### Development Tools
- **wasm-pack**: WASM compilation and packaging
- **cargo**: Rust build system
- **bun**: JavaScript/TypeScript package manager and runtime
- **just**: Task runner
- **vite**: Frontend build tool and dev server

### Optional Tools
- **mkcert**: For generating local TLS certificates
- **worldtool**: CLI for Nostr and MoQ relay management

## Build Commands

### Development
- `just dev` - Build WASM (dev mode) + start dev server
- `just build-wasm-dev` - Build all WASM modules in dev mode
- `cd packages/app && bun run dev` - Frontend only (WASM must exist)

### Production
- `just build` - Build everything for production (WASM + frontend)
- `just build-wasm` - Build all WASM modules in release mode

### Testing & Validation
- `just test` - Run all tests (Rust + TypeScript)
- `just check` - Pre-deployment validation (check, clippy, fmt, build)
- `cargo test --workspace` - Rust tests only

### Utilities
- `just clean` - Clean build artifacts
- `just install` - Install dependencies
- `just preview` - Preview production build
- `just start-live` - Initialize Nostr live event
- `just moq-relay` - Run local MoQ relay server

## Project Structure

### Workspace Organization
```
crossworld/
├── crates/           # Rust crates (compiled to WASM)
│   ├── world/       # Main world simulation + WASM bindings
│   ├── cube/        # Voxel octree engine
│   ├── physics/     # Rapier3D integration
│   ├── renderer/    # CPU raytracer
│   ├── assets/      # Asset management
│   └── worldtool/   # CLI tool (not compiled to WASM)
├── packages/
│   ├── app/         # Main React application
│   ├── common/      # Shared UI components
│   ├── editor/      # Voxel model editor
│   ├── wasm-world/  # Generated from crates/world
│   ├── wasm-cube/   # Generated from crates/cube
│   └── wasm-physics/# Generated from crates/physics
├── doc/             # Project documentation
├── openspec/        # OpenSpec system (change proposals)
├── assets/          # Static assets (models, textures)
└── justfile         # Task automation
```

### Generated Artifacts (Excluded from Git)
- `packages/wasm-*/` - Auto-generated WASM bindings
- `target/` - Rust build outputs
- `dist/` - Frontend build outputs
- `node_modules/` - JavaScript dependencies

## Documentation Files

### Overview
- `doc/README.md` - Documentation index and navigation guide

### Getting Started
- `doc/QUICKSTART.md` - Voice chat setup and first-run guide
- `packages/app/README.md` - App-specific information and setup

### Architecture
- `doc/architecture/overview.md` - High-level system architecture and components
- `doc/architecture/voxel-system.md` - Voxel octree engine and CSM format
- `doc/architecture/physics.md` - Physics integration with Rapier3D
- `doc/architecture/raycast.md` - Ray-octree intersection system
- `doc/architecture/rendering.md` - Rendering pipeline and Three.js integration

### Features
- `doc/features/avatar-system.md` - Avatar design, physics, and animation
- `doc/features/voice-chat.md` - MoQ-based spatial voice chat setup and debugging
- `doc/features/nostr-integration.md` - Nostr identity, discovery, and worldtool CLI

### Reference
- `doc/reference/project-structure.md` - Repository organization and crate layout
- `doc/reference/build-system.md` - Build process, justfile commands, and WASM compilation
- `doc/reference/materials.md` - Material system and shader specifications

### Development
- `CLAUDE.md` - OpenSpec instructions for AI assistants
- `openspec/AGENTS.md` - Detailed OpenSpec workflow
- `doc/design-master.md` - Historical design decisions
