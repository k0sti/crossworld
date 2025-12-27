# Tasks: Add Terrain Composite Collider

## 1. Core Data Types

- [x] 1.1 Create `crates/physics/src/terrain/mod.rs` with module exports
- [x] 1.2 Implement `RegionId` in `region_id.rs`:
  - Corner-based coordinates (IVec3 + depth)
  - `to_region_bounds()` conversion to cube::RegionBounds
  - `local_aabb()` for region bounds in [0,1] space
  - `from_world_aabb()` iterator over intersecting regions
- [x] 1.3 Implement `TerrainPartId` in `region_id.rs`:
  - u64 encoding: [depth:4][x:16][y:16][z:16][tri_idx:12]
  - `new()`, `region()`, `triangle_idx()` methods
  - `face_idx()` and `triangle_in_face()` helpers

## 2. Triangle Generation

- [x] 2.1 Implement `face_to_triangles()` in `triangle_gen.rs`:
  - Input: `FaceInfo`, `world_size: f32`
  - Output: `[Triangle; 2]` (Rapier triangles)
  - Handle all 6 face directions with correct winding
- [x] 2.2 Implement `face_to_triangle()` single-triangle variant
- [x] 2.3 Add unit tests for triangle generation:
  - Test each face direction
  - Verify winding order produces outward normals
  - Test world-space transformation

## 3. Region Cache

- [x] 3.1 Implement `RegionCollisionData` in `region_cache.rs`:
  - `from_octree()` using `visit_faces_in_region()`
  - `triangle_count()` returns faces.len() * 2
  - `get_triangle()` by index
  - `get_triangle_aabb()` by index
- [x] 3.2 Add unit tests for region cache:
  - Test empty region returns empty faces
  - Test solid region returns expected faces
  - Test triangle retrieval by index

## 4. Main Collider Structure

- [x] 4.1 Implement `VoxelTerrainCollider` struct in `collider.rs`:
  - Fields: triangle_bvh, cube, world_size, border_materials, region_depth, region_cache, global_aabb
  - `new()` constructor
- [x] 4.2 Implement `on_terrain_modified()`:
  - Invalidate affected regions in cache
- [x] 4.3 Implement `update_triangle_bvh()`:
  - Find regions intersecting active AABB
  - Populate cache for missing regions
  - Build triangle_bvh from cached faces

## 5. Active Region Tracking

- [x] 5.1 Implement `ActiveRegionTracker` in `active_region.rs`:
  - `new(margin: f32)` constructor
  - `update()` takes dynamic AABBs, returns Option<Aabb> if changed
  - Margin-based hysteresis to avoid thrashing
- [x] 5.2 Add unit tests for active region tracking:
  - Test region expansion triggers rebuild
  - Test small movements don't trigger rebuild
  - Test empty dynamics returns None

## 6. Shape Trait Implementation

- [x] 6.1 Implement `CompositeShape` trait for `VoxelTerrainCollider` in `shape_impl.rs`:
  - `map_part_at()` looks up triangle by index
  - `bvh()` returns &triangle_bvh
- [x] 6.2 Implement `TypedCompositeShape` trait:
  - `type PartShape = Triangle`
  - `type PartNormalConstraints = ()`
  - `map_typed_part_at()` looks up in cache, returns Triangle
  - `map_untyped_part_at()` for dynamic dispatch
- [x] 6.3 Add integration test:
  - Create terrain collider with simple octree
  - Query triangles via trait methods
  - Verify triangles match expected positions

## 7. Cube Crate Enhancement (Optional)

- [ ] 7.1 Add `collect_faces_with_aabbs()` to `visit_faces.rs`:
  - Returns `Vec<(FaceInfo, Aabb)>`
  - Computes face AABB during traversal
  - Optimization for collision where both are needed

## 8. Integration

- [x] 8.1 Export terrain module from `crates/physics/src/lib.rs`
- [ ] 8.2 Add integration example showing:
  - Creating VoxelTerrainCollider from WorldCube
  - Using with Rapier ColliderSet
  - Active region tracking during simulation
- [ ] 8.3 Update `doc/architecture/cubeworld-collision.md` with implementation notes

## 9. Testing and Validation

- [x] 9.1 Run `cargo test --workspace` to verify all tests pass
- [x] 9.2 Run `cargo clippy --workspace -- -D warnings`
- [x] 9.3 Run `cargo check --workspace` to verify compilation
- [ ] 9.4 Profile triangle generation and QBVH rebuild times

## Dependencies

- Tasks 1.x must complete before 2.x, 3.x, 4.x
- Tasks 2.x, 3.x can run in parallel
- Tasks 4.x, 5.x depend on 2.x and 3.x
- Task 6.x depends on 4.x and 5.x
- Task 7.x is independent (optional optimization)
- Task 8.x depends on 6.x

## Implementation Notes

### API Changes from Design

The implementation uses Rapier/Parry 0.25 APIs (via rapier3d git dependency) which differ from the originally designed 0.17 APIs:

- **`Bvh`** instead of `Qbvh` - Parry 0.25 renamed the type
- **`TypedCompositeShape`** instead of `TypedSimdCompositeShape` - Simplified trait interface
- **`CompositeShape`** trait added for untyped access
- **`from_world_aabb()`** returns `Vec<RegionId>` instead of iterator due to closure lifetime constraints

### Triangle Winding

Triangle winding uses `Face::vertices()` from the cube crate to ensure consistent counter-clockwise winding when viewed from outside, producing correct outward-facing normals for collision detection.
