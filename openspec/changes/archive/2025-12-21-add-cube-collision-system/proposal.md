# Change: Add Cube Collision System

## Why

The current collision system generates collision geometry upfront using `VoxelColliderBuilder`, which creates compound colliders from all exposed voxel faces. This approach:
1. Creates expensive static colliders for dynamic objects
2. Doesn't leverage spatial locality - collisions only matter where objects intersect
3. Cannot utilize SDF-based collision detection for fabric/procedural surfaces
4. Has no support for Cube-to-Cube collisions with bounding volume optimization
5. Uses `#[cfg(not(target_arch = "wasm32"))]` for region filtering, limiting WASM support

A new collision system is needed that:
- Calculates AABB bounding volumes for CubeObjects (tighter than spheres for cubes)
- Determines intersection regions between objects efficiently
- Traverses only faces within intersection volumes
- Supports both face-based and SDF-based collision modes
- Is fully WASM-compatible (no conditional compilation)

## What Changes

- **ADDED**: WASM-compatible `Aabb` struct using only glam types
- **ADDED**: OBB→AABB transformation for rotated objects
- **ADDED**: Intersection region calculation (outputs CubeCoord + size Vec3I)
- **ADDED**: Face traversal within bounded regions
- **ADDED**: Static Cube to dynamic CubeObject collision
- **ADDED**: Dynamic CubeObject to CubeObject collision
- **ADDED**: SDF-based collision detection (high-level design for fabric cubes)
- **MODIFIED**: VoxelColliderBuilder to use region-based face iteration
- **ADDED**: `collision.md` documentation describing all collision calculations
- **DEPRECATED**: Full-cube face iteration for collision (replaced by region-based)

## Impact

- Affected specs: New `cube-collision` capability
- Affected code:
  - `crates/physics/src/collider.rs` - Refactor VoxelColliderBuilder
  - `crates/physics/src/collision.rs` - New collision module (WASM-compatible)
  - `crates/cube/src/traversal/` - Add region-bounded traversal
  - `doc/architecture/collision.md` - New documentation
- WASM compatibility: Full support, no `#[cfg]` exclusions
