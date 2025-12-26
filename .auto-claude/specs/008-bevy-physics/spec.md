# Specification: Bevy Physics Prototype

## Overview

Create a standalone native physics prototype (`crates/proto`) using Bevy to validate and demonstrate advanced voxel physics capabilities. This prototype tests Rapier3D physics integration with complex voxel colliders before integrating into the web application.

## Source

Migrated from: `openspec/changes/create-bevy-physics-prototype/`

## Current Status

**Completion: ~58% (Phases 1-3 complete, Phase 4 partial, Phases 5-10 pending, Phase 11 partial)**

### Completed Work
- Phase 1: Project Setup (5/5) - crate created with dependencies, config, workspace, justfile
- Phase 2: Enhanced Physics Collider System (7/7) - bevy feature flag, native module, spatial filtering API
- Phase 3: Application Scaffold (7/7) - Bevy app, plugins, camera, debug overlay, flake.nix
- Phase 4: World Generation (3/5) - WorldCube mesh generation with i32 API
- Phase 11: Documentation (4/6) - README, config docs, algorithm docs

### Pending Work
- Phase 4: Complete world collision and dev verification
- Phase 5: Voxel Object System - .vox file loading and mesh conversion
- Phase 6: Dynamic Cube Spawning - random positions, physics bodies
- Phase 7: Optimized Collision Integration - AABB overlap, spatial filtering
- Phase 8: Player Character System - KinematicCharacterController, movement
- Phase 9: Physics Simulation - gravity, collisions, tuning
- Phase 10: Testing and Validation - performance benchmarks
- Phase 11: Complete example configs and CLAUDE.md update

## Problem Statement

**Goal**: Validate Rapier3D physics integration with voxel colliders in a standalone native environment before web deployment.

**Key challenges**:
1. Physics Validation: Test Rapier3D with complex voxel colliders
2. Performance Benchmarking: Measure voxel collider generation and physics simulation
3. Collider Optimization: Develop face-based collision that only processes overlapping regions
4. Character Controller: Validate player physics alongside dynamic objects

## Architecture

### Core Components

```
crates/proto/
  src/
    main.rs       - Bevy app initialization and systems
  config.toml     - World and physics configuration
  flake.nix       - NixOS development environment
  README.md       - Usage documentation

crates/physics/
  src/
    lib.rs        - Physics module exports
    native.rs     - Bevy-specific physics utilities (feature-gated)
    collision.rs  - VoxelColliderBuilder with spatial filtering
```

### Dependencies
- `bevy = "0.17.3"` - Game engine and ECS
- `bevy_rapier3d` - Bevy-integrated physics plugin
- `cube` - Voxel octree and .vox loading
- `world` - WorldCube terrain generation
- `crossworld-physics` - VoxelColliderBuilder
- `toml`, `serde` - Configuration

### Key Features

| Feature | Description |
|---------|-------------|
| World Generation | WorldCube from config (macro_depth, micro_depth, seed) |
| Voxel Objects | Load random .vox models from assets directory |
| Dynamic Spawning | Spawn cubes at random positions in air |
| Optimized Collision | Face-based traversal with AABB overlap filtering |
| Player Character | KinematicCharacterController with WASD + jump |
| Camera | Orbit/free-fly controls (reuse from editor) |
| Debug Overlay | FPS, entity count, physics stats |

## Configuration

Via `crates/proto/config.toml`:

```toml
# World parameters
seed = 12345
macro_depth = 5
micro_depth = 3

# Physics settings
spawn_count = 20
models_path = "packages/app/dist/assets/models/vox/"

# Gravity
gravity_y = -9.81
```

## Affected Files

### Primary (to modify/create)
- `crates/proto/src/main.rs` - Bevy app and systems
- `crates/proto/config.toml` - Configuration
- `crates/physics/src/native.rs` - Bevy-specific utilities
- `crates/physics/src/collision.rs` - Spatial filtering API

### Dependencies (reference only)
- `crates/cube/` - Voxel structures, used as-is
- `crates/world/` - World generation, used as-is
- `crates/editor/` - Camera patterns reference

### Root level
- `Cargo.toml` - Workspace member
- `justfile` - `just proto` task
- `CLAUDE.md` - Project documentation

## Success Criteria

1. Application launches with Bevy window
2. WorldCube renders as static mesh with collision
3. .vox models load and spawn as dynamic physics bodies
4. Cubes fall with gravity and collide realistically
5. Player character navigates world with WASD + jump
6. Optimized collision reduces face count in overlap regions
7. Debug overlay shows FPS and physics stats
8. Configuration via TOML works for all parameters

## Development Environment

```bash
# Run prototype
just proto

# Or directly
cargo run -p proto

# Build only
cargo build -p proto

# Run physics tests
cargo test -p crossworld-physics
```

## Out of Scope

- Web/WASM integration (handled separately)
- Networking/multiplayer
- Save/load functionality
- Complex UI beyond debug overlay
