# Change: Create Bevy Physics Prototype

## Why

The project needs a standalone native physics prototype to validate and demonstrate advanced voxel physics capabilities:

1. **Physics Validation**: Test Rapier3D physics integration with complex voxel colliders before integrating into the web application
2. **Performance Benchmarking**: Measure performance of voxel collider generation and physics simulation with multiple objects
3. **Collider Optimization**: Develop and test optimized face-based collision detection that only processes overlapping volume regions
4. **Character Controller Testing**: Validate character physics (player cube) alongside dynamic voxel objects
5. **Configuration System**: Design TOML-based world configuration that can later be ported to the web app

This prototype builds on the existing `crates/editor` Bevy foundation but focuses on physics simulation rather than editing.

## What Changes

- **NEW**: Create `crates/proto` - Standalone Bevy application for physics prototyping
- **ENHANCED**: Extend `crates/physics` with optimized voxel collider generation:
  - Face-based collision traversal algorithm
  - Spatial filtering to only process overlapping volume regions between colliders
  - Native Bevy integration (non-WASM features)
- Reuse existing `cube` crate for voxel structure and .vox file loading
- Reuse existing `world` crate for world generation
- Implement Bevy systems for:
  - Loading random .vox models from `packages/app/dist/assets/models/vox/`
  - Spawning dynamic voxel cubes at random positions in the air
  - Physics simulation with falling objects
  - Character controller (player cube) for navigation
  - Camera controls (orbit, free-fly)
  - TOML configuration for world parameters (seed, depth, spawn count)

## Impact

**New capabilities:**
- `bevy-physics-prototype` - Full specification for native physics prototype application

**Affected code:**
- **NEW**: `crates/proto/` - New Bevy application crate with binary `proto`
- **NEW**: `crates/proto/config.toml` - World and physics configuration
- **ENHANCED**: `crates/physics/src/collider.rs` - Add optimized voxel collider generation
- **ENHANCED**: `crates/physics/src/lib.rs` - Add native-only features (non-WASM)
- **NO CHANGES**: `crates/cube/` - Used as-is (voxel octree and .vox loading)
- **NO CHANGES**: `crates/world/` - Used as-is (world generation)
- `Cargo.toml` - Add new workspace member `crates/proto`
- `justfile` - Add `just proto` task for running the prototype

**Dependencies:**
- Primary: `cube`, `world`, `crossworld-physics` (existing crates)
- New for proto: `bevy = "0.17.3"`, `bevy_rapier3d` (Bevy-specific physics integration), `toml` (config parsing)
- Enhanced physics features: `bevy` feature flag for non-WASM native integration

**Non-breaking:**
- Existing WASM physics (`crossworld-physics`) continues to work unchanged
- Existing editor (`crates/editor`) can coexist
- All changes to physics crate are behind feature flags or in separate modules
