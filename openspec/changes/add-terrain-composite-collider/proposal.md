# Change: Add Terrain Composite Collider with TypedSimdCompositeShape

## Why

The current world collision implementation uses either:
1. **Monolithic compound collider**: Pre-generates all terrain faces as a single compound shape (slow initialization, large memory)
2. **Direct octree queries**: Bypasses Rapier entirely for terrain collision (`WorldCollider::resolve_collision`)

Neither approach leverages Rapier's efficient `TypedSimdCompositeShape` trait, which allows lazy on-demand triangle generation during narrowphase collision detection. This trait enables the terrain to appear as a single collider while generating geometry only for regions actively queried.

The design document at `doc/architecture/cubeworld-collision.md` outlines the integration pattern using the cube crate's existing `visit_faces_in_region()` and `RegionBounds` for efficient spatial queries.

## What Changes

### New Terrain Collider System

1. **RegionId and TerrainPartId**: Compact encodings for identifying collision regions and individual triangles within the octree coordinate system.

2. **RegionCollisionData Cache**: Per-region cache that stores faces queried via `visit_faces_in_region()`, avoiding repeated octree traversal.

3. **Triangle Generation**: Convert cube crate `FaceInfo` to Rapier `Triangle` shapes on demand (2 triangles per face).

4. **Two-Level QBVH**: Coarse region-level QBVH + fine triangle-level QBVH for active regions near dynamic objects.

5. **TypedSimdCompositeShape Implementation**: Integrate with Rapier's narrowphase by implementing the composite shape trait.

6. **Active Region Tracking**: Only build triangle QBVH for regions near dynamic bodies.

### Integration Points

- Uses existing `cube::visit_faces_in_region()` - no changes to cube crate required
- Uses existing `cube::RegionBounds` for octree coordinate mapping
- Adds optional `collect_faces_with_aabbs()` function to cube crate for optimization
- Extends physics crate with new `terrain/` module

## Impact

- **New spec**: `terrain-collision` capability (separate from existing `cube-collision`)
- **Affected code**:
  - `crates/physics/src/terrain/` - New module with all implementations
  - `crates/cube/src/traversal/visit_faces.rs` - Optional optimization function
- **Performance**: Lazy generation reduces memory and enables infinite terrain
- **Compatibility**: Does not modify existing `WorldCollider` - can coexist as alternative strategy
