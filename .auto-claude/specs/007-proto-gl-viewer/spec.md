# Specification: Proto-GL Physics Viewer

## Overview

Create a lightweight physics viewer (`crates/proto-gl`) that replaces Bevy's rendering stack with the existing egui+GL renderer from `crates/renderer`. This maintains the physics validation goals while dramatically improving build times compared to the Bevy-based prototype.

## Source

Migrated from: `openspec/changes/create-proto-gl-physics-viewer/`

## Current Status

**Completion: ~75% (Phases 1-6 complete, Phases 7-12 partially complete or pending)**

### Completed Work
- Phase 1: Project Setup (6/6) - crate created with dependencies
- Phase 2: Application Scaffold (7/7) - winit, glutin, egui setup
- Phase 3: Rendering Integration (7/7) - GlCubeTracer, orbit camera
- Phase 4: World Generation and Rendering (9/9) - CSM parsing, mesh generation, rendering
- Phase 5: Physics World Setup (6/6) - Rapier3D, VoxelColliderBuilder
- Phase 6: CubeObject System (7/7) - .vox loading, model management
- Phase 7: Dynamic Cube Spawning (5/7) - partial, random rotations pending
- Phase 8: Physics Simulation (2/8) - partial, transform extraction pending
- Phase 9: UI Panel (7/8) - egui panels, reset pending

### Pending Work
- Complete cube spawn with random rotations
- Extract transforms from physics state
- Render dynamic cubes at physics positions
- Verify collisions work correctly
- Scene reset functionality
- Testing and validation (Phase 10)
- Documentation (Phase 11)
- Integration testing (Phase 12)

## Problem Statement

**Current bottleneck**: The Bevy-based physics prototype (`crates/proto`) has slow build times (~45-60s) that hinder rapid iteration during physics development and experimentation.

**Root cause**: Bevy's large dependency tree and compilation overhead create significant latency in the develop-test cycle.

**Solution**: Use the existing egui+GL rendering stack from `crates/renderer` which provides:
- Faster compile times (~5-10s)
- Simpler dependency tree
- Proven rendering pipeline
- 3D viewport + UI controls

## Architecture

### Core Components

```
crates/proto-gl/
  src/
    main.rs       - Entry point, winit event loop
    app.rs        - Application state and logic
    config.rs     - Configuration loading
    camera.rs     - Orbit camera implementation
    physics.rs    - Physics world wrapper
    models.rs     - .vox model loading
  config.toml     - Default configuration
```

### Dependencies
- `cube` - CSM parsing, Cube types, mesh generation
- `crossworld-physics` - VoxelColliderBuilder, Rapier3D integration
- `renderer` - GlCubeTracer, GL rendering pipeline
- `glow`, `glutin`, `winit` - OpenGL context and windowing
- `egui`, `egui-glow` - UI framework

### Comparison with crates/proto (Bevy)

| Feature | crates/proto (Bevy) | crates/proto-gl (egui+GL) |
|---------|---------------------|---------------------------|
| Rendering | Bevy PBR + ECS | egui + OpenGL + GlCubeTracer |
| Physics | Rapier3D (Bevy plugin) | Rapier3D (direct) |
| UI | Bevy UI | egui |
| Build time | ~45-60s | ~5-10s |
| Debug overlay | Bevy diagnostics | egui panels |
| Player character | Yes (planned) | No |
| Camera | Orbit/free-fly | Orbit (egui-controlled) |

## Configuration

Strategy selection via `config.toml`:

```toml
# Root cube CSM file
root_cube = "crossworld.csm"

# World settings
[world]
border_depth = 4

# Physics settings
[physics]
gravity = -9.81
spawn_count = 10
spawn_radius = 50.0

# Models directory
models_path = "models/"
```

## Affected Files

### Primary
- `crates/proto-gl/src/main.rs` - Application entry point
- `crates/proto-gl/src/app.rs` - Main application logic
- `crates/proto-gl/src/config.rs` - Configuration handling
- `crates/proto-gl/config.toml` - Default configuration

### Dependencies Used
- `crates/cube/` - Voxel data structures and mesh generation
- `crates/physics/` - Physics world and collision
- `crates/renderer/` - GL rendering

## Success Criteria

1. Application launches and displays 3D viewport
2. Parse CSM -> generate voxel mesh -> render in GL viewport
3. Spawn dynamic cubes with physics
4. Cubes fall with gravity and collide with world
5. Build time < 15 seconds (clean build)
6. Configurable via TOML (world depth, spawn count, physics params)

## Development Environment

```bash
# Run proto-gl
just proto-gl

# Or directly
cargo run -p proto-gl

# Build only
cargo build -p proto-gl
```

## Out of Scope

Per user requirements:
- **No player/character system** - focus on falling cubes only
- **No WorldCube wrapper** - direct Cube usage
- Web integration (handled by crates/proto with Bevy)
