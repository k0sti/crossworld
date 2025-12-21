# Change: Optimize World Collision Performance

## Why

The current proto-gl implementation creates a single massive compound collider for the entire world terrain at startup (`VoxelColliderBuilder::from_cube_scaled()`). With `border_depth=10` producing an 8192-unit world, this generates thousands of thin shell cuboids that Rapier must test against every dynamic object.

**Current bottleneck**: Every physics step tests 100+ dynamic box colliders against potentially thousands of world face colliders via Rapier's broad-phase BVH.

**Existing infrastructure** (from `add-cube-collision-system`):
- `Aabb`, `IntersectionRegion`, `RegionBounds` - already implemented in `crates/physics/src/collision.rs`
- `visit_faces_in_region()` - already implemented in `crates/cube/src/traversal/visit_faces.rs`
- `VoxelColliderBuilder::from_cube_with_region_scaled()` - already implemented

**Problem**: This infrastructure is **not integrated** into proto-gl's physics loop.

## What Changes

This change implements and benchmarks **three world collision strategies** with a common `WorldCollider` trait interface, enabling performance comparison and strategy selection:

### Strategy 1: Monolithic Compound (Baseline)
- Current approach: single compound collider with all world faces
- Serves as performance baseline for comparison
- No changes to existing behavior

### Strategy 2: Chunked Colliders
- Divide world into spatial chunks (e.g., 64×64×64 units)
- Load/unload chunk colliders based on dynamic object positions
- Uses existing `visit_faces_in_region()` for chunk mesh generation
- Rapier handles chunk↔object collision via standard broad-phase

### Strategy 3: Custom Octree Query (Hybrid)
- Use Rapier only for dynamic↔dynamic collision
- Bypass Rapier for world collision with direct octree queries
- Resolve penetration manually using face normals
- Leverages octree spatial hierarchy without flattening to compound shapes

### Common Interface

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

### Benchmarking Framework

- Configurable strategy selection via `config.toml`
- Frame time measurement for physics step
- Collider count tracking
- Face generation count per frame

## Impact

- **Affected specs**: New `world-collision-strategies` capability
- **Affected code**:
  - `crates/physics/src/world_collider.rs` - New module with trait + implementations
  - `crates/proto-gl/src/app.rs` - Integrate configurable strategy
  - `crates/proto-gl/config.toml` - Add strategy selection
- **Performance target**: 5-10x improvement over monolithic approach
- **Backward compatibility**: Monolithic strategy preserves current behavior
