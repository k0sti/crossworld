# Design: Terrain Composite Collider

## Context

Rapier physics engine supports custom composite shapes via the `TypedSimdCompositeShape` trait. This allows a shape to expose a QBVH (Quantized Bounding Volume Hierarchy) of sub-parts while generating actual geometry lazily during collision queries.

The cube crate provides efficient octree traversal with `visit_faces_in_region()` and `RegionBounds`, which can query faces in bounded regions of the octree.

This design bridges the two systems: Rapier queries our custom collider, which looks up triangles from the octree on demand.

## Goals / Non-Goals

**Goals:**
- Implement `TypedSimdCompositeShape` for voxel terrain
- Generate triangles lazily from octree faces
- Cache region data to avoid repeated traversal
- Use existing cube crate traversal functions

**Non-Goals:**
- Replace existing `WorldCollider` implementation
- Modify core Rapier physics pipeline
- Support dynamic terrain editing in real-time (cache invalidation is basic)

## Decisions

### Decision 1: Region-Based Indexing

**What**: Use spatial regions (fixed-depth octants) as the primary index, not individual triangles.

**Why**:
- Aligns with cube crate's `RegionBounds` coordinate system
- Enables region-level cache invalidation
- Reduces QBVH complexity at coarse level
- Each region maps to a `visit_faces_in_region()` call

**Alternatives considered**:
- Per-triangle indexing: Would require custom octree traversal, not compatible with existing functions
- Chunk-based (world-space grid): Would not align with octree structure, causing boundary issues

### Decision 2: Two-Level QBVH

**What**: Maintain two QBVHs - region-level (coarse) and triangle-level (fine).

**Why**:
- Region QBVH is cheap to build/update for entire terrain
- Triangle QBVH only built for active regions near dynamic objects
- Matches Rapier's expectation for `typed_qbvh()` returning triangle-level index

**Implementation**:
```rust
pub struct VoxelTerrainCollider {
    region_qbvh: Qbvh<RegionId>,      // Coarse: all regions
    triangle_qbvh: Qbvh<TerrainPartId>, // Fine: active regions only
}
```

### Decision 3: Face-to-Triangle Conversion

**What**: Each `FaceInfo` from octree produces 2 triangles.

**Why**:
- Rapier's `TypedSimdCompositeShape` expects `Triangle` parts
- Voxel faces are quads, naturally split into 2 triangles
- Consistent winding order ensures correct normals

**Encoding**:
```
TerrainPartId = [depth:4][x:16][y:16][z:16][tri_idx:12]
tri_idx = face_index * 2 + (0 or 1)
```

### Decision 4: Region Cache

**What**: Cache `Vec<FaceInfo>` per region, populated lazily.

**Why**:
- Avoids repeated octree traversal for same region
- Faces are immutable reference data (octree doesn't change during frame)
- Can be invalidated per-region on terrain modification

**Trade-off**: Memory usage vs. CPU. With depth 3 (8 regions per axis = 512 regions), cache is bounded.

### Decision 5: Active Region Tracking

**What**: Only build triangle QBVH for regions overlapping dynamic body AABBs.

**Why**:
- Unbounded terrain would require unbounded triangle QBVH
- Most frames only need triangles near moving objects
- Region tracking detects when QBVH needs rebuild

## Data Flow

```
Physics Step Start
       │
       ▼
┌──────────────────────────┐
│ Collect dynamic AABBs    │
└──────────────────────────┘
       │
       ▼
┌──────────────────────────┐
│ ActiveRegionTracker      │
│ - Compute union of AABBs │
│ - Check if rebuild needed│
└──────────────────────────┘
       │ (if changed)
       ▼
┌──────────────────────────┐
│ For each region in AABB: │
│ - Query region_cache     │
│ - If miss: visit_faces() │
│ - Add triangles to QBVH  │
└──────────────────────────┘
       │
       ▼
┌──────────────────────────┐
│ triangle_qbvh.rebuild()  │
└──────────────────────────┘
       │
       ▼
┌──────────────────────────┐
│ Rapier physics step      │
│ - Broadphase: terrain vs │
│   dynamic bodies         │
│ - Narrowphase: queries   │
│   TypedSimdCompositeShape│
│   → map_typed_part_at()  │
│   → returns Triangle     │
└──────────────────────────┘
```

## Risks / Trade-offs

| Risk | Mitigation |
|------|------------|
| QBVH rebuild every frame | ActiveRegionTracker only triggers on significant movement |
| Cache memory growth | Bound region depth; LRU eviction if needed |
| Triangle generation overhead | Triangles are simple Vec3 arithmetic; profile if bottleneck |
| Rapier version compatibility | Use Qbvh and TypedSimdCompositeShape from rapier3d::parry |

## File Structure

```
crates/physics/src/
├── terrain/
│   ├── mod.rs              # Module exports
│   ├── region_id.rs        # RegionId, TerrainPartId
│   ├── triangle_gen.rs     # face_to_triangles()
│   ├── region_cache.rs     # RegionCollisionData
│   ├── collider.rs         # VoxelTerrainCollider
│   ├── active_region.rs    # ActiveRegionTracker
│   └── shape_impl.rs       # Shape + TypedSimdCompositeShape impls
├── lib.rs                  # Add `mod terrain;`
```

## Open Questions

1. **Region depth selection**: Should be configurable (3-4 recommended). Higher depth = finer regions = more QBVH entries.

2. **Cache eviction**: Current design clears cache only on terrain modification. For very large worlds, may need LRU eviction.

3. **Parallel region loading**: Could populate region cache on background thread for async terrain streaming.
