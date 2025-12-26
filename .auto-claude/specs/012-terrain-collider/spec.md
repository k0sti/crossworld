# Specification: Add Terrain Composite Collider

## Overview

Implement a terrain composite collider using Rapier's `TypedCompositeShape` trait for lazy on-demand triangle generation during narrowphase collision detection. This enables efficient collision with infinite terrain by generating geometry only for regions actively queried, rather than pre-generating all terrain faces upfront.

## Source

Migrated from: `openspec/changes/add-terrain-composite-collider/`

## Current Status

**Completion: 78% (18/23 tasks complete)**

### Completed Work
- Phase 1: Core Data Types (3/3) - RegionId, TerrainPartId
- Phase 2: Triangle Generation (3/3) - face_to_triangles(), unit tests
- Phase 3: Region Cache (2/2) - RegionCollisionData
- Phase 4: Main Collider Structure (3/3) - VoxelTerrainCollider
- Phase 5: Active Region Tracking (2/2) - ActiveRegionTracker
- Phase 6: Shape Trait Implementation (3/3) - CompositeShape, TypedCompositeShape
- Phase 8.1: Export terrain module from lib.rs
- Phase 9.1-9.3: Tests pass, clippy clean, workspace compiles

### Pending Work
- Phase 7.1: Optional `collect_faces_with_aabbs()` optimization
- Phase 8.2: Integration example
- Phase 8.3: Update architecture documentation
- Phase 9.4: Profile triangle generation and QBVH rebuild times

## Problem Statement

The current world collision implementation uses either:
1. **Monolithic compound collider**: Pre-generates all terrain faces as a single compound shape (slow initialization, large memory)
2. **Direct octree queries**: Bypasses Rapier entirely for terrain collision (`WorldCollider::resolve_collision`)

Neither approach leverages Rapier's efficient `TypedCompositeShape` trait, which allows lazy on-demand triangle generation during narrowphase collision detection.

## Solution: TypedCompositeShape Terrain Collider

### Architecture

1. **RegionId and TerrainPartId**: Compact encodings for identifying collision regions and individual triangles within the octree coordinate system.
   - TerrainPartId u64 encoding: `[depth:4][x:16][y:16][z:16][tri_idx:12]`

2. **RegionCollisionData Cache**: Per-region cache storing faces queried via `visit_faces_in_region()`, avoiding repeated octree traversal.

3. **Triangle Generation**: Convert cube crate `FaceInfo` to Rapier `Triangle` shapes on demand (2 triangles per face).

4. **Two-Level BVH**: Coarse region-level BVH + fine triangle-level BVH for active regions near dynamic objects.

5. **TypedCompositeShape Implementation**: Integrate with Rapier's narrowphase by implementing the composite shape trait.

6. **Active Region Tracking**: Only build triangle BVH for regions near dynamic bodies.

### Key Components

```rust
/// Compact region identifier
pub struct RegionId {
    corner: IVec3,
    depth: u8,
}

/// Triangle identifier within terrain
pub struct TerrainPartId(u64);

/// Cached collision data for a region
pub struct RegionCollisionData {
    faces: Vec<FaceInfo>,
    world_size: f32,
}

/// Main terrain collider implementing TypedCompositeShape
pub struct VoxelTerrainCollider<T> {
    triangle_bvh: Bvh,
    cube: Rc<Cube<T>>,
    world_size: f32,
    border_materials: HashSet<T>,
    region_depth: u32,
    region_cache: HashMap<RegionId, RegionCollisionData>,
    global_aabb: Aabb,
}

/// Tracks active regions near dynamic bodies
pub struct ActiveRegionTracker {
    current_aabb: Option<Aabb>,
    margin: f32,
}
```

## Integration Points

- Uses existing `cube::visit_faces_in_region()` - no changes to cube crate required
- Uses existing `cube::RegionBounds` for octree coordinate mapping
- Adds optional `collect_faces_with_aabbs()` function to cube crate for optimization
- Extends physics crate with new `terrain/` module

## API Notes (Rapier/Parry 0.25)

The implementation uses Rapier/Parry 0.25 APIs which differ from older versions:

- **`Bvh`** instead of `Qbvh` - Parry 0.25 renamed the type
- **`TypedCompositeShape`** instead of `TypedSimdCompositeShape` - Simplified trait interface
- **`CompositeShape`** trait added for untyped access
- **`from_world_aabb()`** returns `Vec<RegionId>` instead of iterator due to closure lifetime constraints

### Triangle Winding

Triangle winding uses `Face::vertices()` from the cube crate to ensure consistent counter-clockwise winding when viewed from outside, producing correct outward-facing normals for collision detection.

## Affected Files

### New Files (terrain module)
- `crates/physics/src/terrain/mod.rs` - Module exports
- `crates/physics/src/terrain/region_id.rs` - RegionId, TerrainPartId
- `crates/physics/src/terrain/triangle_gen.rs` - face_to_triangles()
- `crates/physics/src/terrain/region_cache.rs` - RegionCollisionData
- `crates/physics/src/terrain/collider.rs` - VoxelTerrainCollider
- `crates/physics/src/terrain/active_region.rs` - ActiveRegionTracker
- `crates/physics/src/terrain/shape_impl.rs` - CompositeShape, TypedCompositeShape

### Modified Files
- `crates/physics/src/lib.rs` - Export terrain module

### Reference Files
- `crates/cube/src/traversal/visit_faces.rs` - Face traversal
- `crates/physics/src/collision.rs` - Collision primitives
- `doc/architecture/cubeworld-collision.md` - Design document

## Performance Benefits

- **Lazy generation**: Reduces memory by only generating triangles for actively queried regions
- **Infinite terrain**: Enables collision with unbounded terrain
- **Two-level BVH**: Fast broad-phase with coarse region BVH, detailed narrow-phase with triangle BVH
- **Region caching**: Avoids repeated octree traversal for static terrain

## Compatibility

- Does not modify existing `WorldCollider` implementations
- Can coexist as alternative collision strategy
- Adds new `terrain-collision` capability separate from existing `cube-collision`

## Success Criteria

1. TypedCompositeShape implementation passes all trait method tests
2. Triangle generation produces correct outward-facing normals
3. Region caching reduces octree traversal
4. Active region tracking prevents thrashing
5. Integration with Rapier ColliderSet works correctly

## Development Environment

```bash
# Run physics tests
cargo test -p crossworld-physics

# Check compilation
cargo check --workspace

# Run clippy
cargo clippy --workspace -- -D warnings
```

## Parallelism

Tasks 2.x and 3.x can run in parallel after Phase 1 completes:
- Triangle generation (Phase 2)
- Region cache (Phase 3)

Phase 7 (Cube Crate Enhancement) is independent and optional.
