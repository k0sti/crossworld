# Specification: Optimize World Collision Performance

## Overview

Implement and benchmark three world collision strategies with a common `WorldCollider` trait interface, enabling performance comparison and strategy selection. The current proto-gl implementation creates a single massive compound collider for the entire world terrain at startup, which generates thousands of thin shell cuboids that Rapier must test against every dynamic object.

## Source

Migrated from: `openspec/changes/optimize-world-collision/`

## Current Status

**Completion: 66% (23/35 tasks complete)**

### Completed Work
- Phase 1: WorldCollider Trait and Infrastructure (3/3)
- Phase 2: Monolithic Strategy / Baseline (3/3)
- Phase 3: Chunked Strategy (4/6) - implementation done, tests pending
- Phase 4: Hybrid Octree Strategy (5/6) - implementation done, efficiency test pending
- Phase 5: Configuration Integration (3/3)
- Phase 6: Proto-GL Integration (3/5) - partial integration

### Pending Work
- Unit tests for chunked strategy (load/unload behavior)
- Octree query efficiency test
- Main loop update() call before physics step
- Debug overlay metrics display
- Full benchmarking framework
- Documentation
- Cleanup and polish

## Problem Statement

**Current bottleneck**: Every physics step tests 100+ dynamic box colliders against potentially thousands of world face colliders via Rapier's broad-phase BVH.

**Root cause**: `VoxelColliderBuilder::from_cube_scaled()` creates a single compound collider with all world faces at startup. With `border_depth=10` producing an 8192-unit world, this generates thousands of thin shell cuboids.

**Existing infrastructure** (from `add-cube-collision-system`):
- `Aabb`, `IntersectionRegion`, `RegionBounds` - implemented in `crates/physics/src/collision.rs`
- `visit_faces_in_region()` - implemented in `crates/cube/src/traversal/visit_faces.rs`
- `VoxelColliderBuilder::from_cube_with_region_scaled()` - implemented

**Problem**: This infrastructure is **not integrated** into proto-gl's physics loop.

## Solution: Three Collision Strategies

### Strategy 1: Monolithic Compound (Baseline)
- Current approach: single compound collider with all world faces
- Serves as performance baseline for comparison
- No changes to existing behavior

### Strategy 2: Chunked Colliders
- Divide world into spatial chunks (e.g., 64x64x64 units)
- Load/unload chunk colliders based on dynamic object positions
- Uses existing `visit_faces_in_region()` for chunk mesh generation
- Rapier handles chunk-object collision via standard broad-phase

### Strategy 3: Custom Octree Query (Hybrid)
- Use Rapier only for dynamic-dynamic collision
- Bypass Rapier for world collision with direct octree queries
- Resolve penetration manually using face normals
- Leverages octree spatial hierarchy without flattening to compound shapes

## Common Interface

```rust
pub trait WorldCollider {
    /// Initialize the collider with world cube and physics world
    fn init(&mut self, cube: &Rc<Cube<u8>>, world_size: f32, physics: &mut PhysicsWorld);

    /// Update colliders based on dynamic object positions (called each frame)
    fn update(&mut self, dynamic_aabbs: &[(RigidBodyHandle, Aabb)], physics: &mut PhysicsWorld);

    /// Resolve world collisions for a body (for hybrid approach)
    fn resolve_collision(&self, body_handle: RigidBodyHandle, physics: &mut PhysicsWorld) -> Vec3;

    /// Get performance metrics
    fn metrics(&self) -> ColliderMetrics;
}
```

## Configuration

Strategy selection via `config.toml`:

```toml
[physics]
world_collision_strategy = "hybrid"  # Options: monolithic, chunked, hybrid

[physics.chunked]
chunk_size = 64.0
load_radius = 2  # chunks beyond object AABB
```

## Affected Files

### Primary
- `crates/physics/src/world_collider.rs` - Trait and implementations
- `crates/proto-gl/src/app.rs` - Strategy integration
- `crates/proto-gl/config.toml` - Strategy selection

### Reference
- `crates/physics/src/collision.rs` - Existing collision primitives
- `crates/cube/src/traversal/visit_faces.rs` - Face traversal

## Bug Fixes Applied

During implementation, several bugs were discovered and fixed:

1. **Face center position**: Was computed as voxel center instead of actual face surface position
2. **Coplanar face corrections**: Were summed instead of taking max per axis
3. **Penetration direction**: Was inverted; now correctly pushes in direction of face normal
4. **Velocity dampening**: Added to prevent continuous falling in hybrid mode

## Performance Target

- **Goal**: 5-10x improvement over monolithic approach
- **Metric**: Frame time for physics step with 100+ dynamic objects
- **Benchmark**: 300 frames with depth-10 terrain world

## Success Criteria

1. All three collision strategies pass unit tests
2. Chunked strategy correctly loads/unloads chunks based on object positions
3. Hybrid strategy correctly resolves penetration without Rapier world colliders
4. Benchmarks demonstrate measurable improvement over monolithic baseline
5. Configuration allows strategy selection at startup
6. Debug overlay shows real-time collision metrics

## Development Environment

```bash
# Run physics tests
cargo test -p crossworld-physics

# Check proto-gl builds
cargo check -p proto-gl

# Run proto-gl with specific strategy
cargo run -p proto-gl  # Uses config.toml setting
```

## Parallelism

After Phase 1 completes, three developers could work in parallel:
- Developer A: Monolithic strategy (Phase 2) + benchmarking (Phase 7)
- Developer B: Chunked strategy (Phase 3)
- Developer C: Hybrid strategy (Phase 4)

Then converge on:
- Configuration (Phase 5)
- Proto-GL integration (Phase 6)
- Documentation and cleanup (Phases 8-9)
