# Proposal: Create Proto-GL Physics Viewer

## Problem Statement

The current Bevy-based physics prototype (`crates/proto`) has slow build times that hinder rapid iteration during development. Bevy's large dependency tree and compilation overhead make the develop-test cycle frustratingly long, especially when experimenting with physics parameters or voxel configurations.

## Proposed Solution

Create a new lightweight physics viewer (`crates/proto-gl`) that replaces Bevy's rendering stack with the existing egui+GL renderer from `crates/renderer`. This maintains the physics validation goals while dramatically improving build times.

### Core Changes

1. **New Crate**: `crates/proto-gl` - standalone native binary
2. **Rendering**: Use `crates/renderer`'s egui+GL stack (reuse patterns from renderer binary)
3. **Physics**: Keep Rapier3D integration from `crates/physics`
4. **Voxel Data**: Direct `Cube<u8>` usage (no WorldCube wrapper)
5. **Scope Reduction**: Remove player/character features, focus on physics validation

## Rationale

**Why egui+GL over Bevy:**
- Faster compile times (~5-10s vs 45-60s for Bevy)
- Simpler dependency tree
- Proven rendering pipeline already in crates/renderer
- Still provides 3D viewport + UI controls

**Why keep proto separate:**
- Proto validates Bevy patterns for future web integration
- Proto-GL validates physics + collision in lightweight environment
- Different tool for different purposes

## Comparison with Existing Proto

| Feature | crates/proto (Bevy) | crates/proto-gl (egui+GL) |
|---------|---------------------|---------------------------|
| Rendering | Bevy PBR + ECS | egui + OpenGL + custom tracers |
| Physics | Rapier3D (Bevy plugin) | Rapier3D (direct) |
| UI | Bevy UI | egui |
| Build time | ~45-60s | ~5-10s |
| Voxel data | CSM → Cube | CSM → Cube |
| Debug overlay | Bevy diagnostics | egui panels |
| Player character | Yes (planned) | No |
| Camera | Orbit/free-fly | Orbit (egui-controlled) |
| Dynamic cubes | Spawn system (planned) | Spawn system |
| World collider | VoxelColliderBuilder | VoxelColliderBuilder |

## Out of Scope

Per user requirements:
- **No debug overlay** requirement (but egui panels are natural anyway)
- **No window title** requirement (but will be set for clarity)
- **No player/character system** - focus on falling cubes only
- **No WorldCube wrapper** - direct Cube usage

## Dependencies

**Core crates to use:**
- `cube` - CSM parsing, Cube types, mesh generation, face mesh utilities
- `crossworld-physics` - VoxelColliderBuilder, Rapier3D integration, physics world management
- `renderer` - GL rendering pipeline (GlCubeTracer), egui integration, window/GL context setup

**Reference crate for patterns:**
- `proto` - Reference for physics setup, cube spawning, and collision handling patterns (code examples only, not a dependency)

**New dependencies:**
- Same as `crates/renderer`: glow, glutin, winit, egui stack
- toml, serde for configuration

## Success Criteria

1. Application launches and displays 3D viewport
2. Parse CSM → generate voxel mesh → render in GL viewport
3. Spawn dynamic cubes with physics
4. Cubes fall with gravity and collide with world
5. Build time < 15 seconds (clean build)
6. Configurable via TOML (world depth, spawn count, physics params)

## Related Work

- Uses patterns from `create-bevy-physics-prototype` but different rendering
- Leverages existing GL renderer infrastructure
- Complements (not replaces) the Bevy prototype
