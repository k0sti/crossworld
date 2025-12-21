# Tasks: Add Cube Collision System

## 1. Core Data Structures (WASM-Compatible)

- [ ] 1.1 Add `Aabb` struct to `crates/physics/src/collision.rs`
  - Fields: `min: Vec3`, `max: Vec3` (glam types only, no parry/nalgebra)
  - Method: `unit() -> Self` (create [0,1]³ AABB)
  - Method: `to_world(position: Vec3, rotation: Quat, scale: f32) -> Self`
  - Method: `intersects(&self, other: &Aabb) -> bool`
  - Method: `intersection(&self, other: &Aabb) -> Option<Aabb>`
  - Unit tests for AABB operations

- [ ] 1.2 Add `IntersectionRegion` struct to `crates/physics/src/collision.rs`
  - Fields: `coord: CubeCoord`, `size: IVec3`
  - Method: `from_aabb(world_aabb, cube_pos, cube_scale, depth) -> Option<Self>`
  - Method: `octant_count() -> usize` (returns 1-8)
  - Method: `iter_coords() -> impl Iterator<Item = CubeCoord>`
  - Unit tests for region calculation

- [ ] 1.3 Add WASM compatibility tests
  - Verify `Aabb` compiles without `#[cfg]` guards
  - Test with `cargo test --target wasm32-unknown-unknown` (if wasm target available)
  - Ensure no parry/nalgebra types in public API

## 2. Region-Bounded Traversal

- [ ] 2.1 Add `traverse_faces_in_region()` to `crates/cube/src/traversal/`
  - Input: `NeighborGrid`, `IntersectionRegion`, visitor callback
  - Only visits voxels within region bounds
  - Reuses existing `NeighborView` and `FaceInfo` infrastructure
  - Unit tests comparing face counts (region vs full)

- [ ] 2.2 Add region bounds checking to traversal
  - Helper: `coord_in_region(coord: CubeCoord, region: IntersectionRegion) -> bool`
  - Early termination when traversing outside region
  - Benchmark: measure reduction in visited nodes

## 3. Collision Generation

- [ ] 3.1 Refactor `VoxelColliderBuilder::from_cube_region()`
  - Accept `IntersectionRegion` instead of raw AABB
  - Use `traverse_faces_in_region()` internally
  - Maintain backward compatibility (None = full traversal)
  - Unit tests for partial region collider generation

- [ ] 3.2 Add `CubeCollider` struct for Cube-CubeObject collision
  - Method: `generate(ground: &Cube, object: &CubeObject, world: &PhysicsWorld) -> Option<Collider>`
  - Calculates bounding sphere, intersection region, generates faces
  - Returns None if no intersection

- [ ] 3.3 Add `ObjectCollider` struct for CubeObject-CubeObject collision
  - Method: `generate(obj_a: &CubeObject, obj_b: &CubeObject, world: &PhysicsWorld) -> Option<(Collider, Collider)>`
  - Sphere-sphere test first
  - Generates colliders for both objects' intersection regions

## 4. SDF Interface (Design Only)

- [ ] 4.1 Define `SdfCollider` trait in `crates/physics/src/sdf.rs`
  - Method: `sdf(&self, point: Vec3) -> f32`
  - Method: `normal(&self, point: Vec3) -> Vec3`
  - Documentation explaining SDF convention (negative = inside)

- [ ] 4.2 Add stub implementation for fabric cubes
  - Implement `SdfCollider` for a `FabricSdf` wrapper
  - Uses quaternion magnitude from `fabric/surface.rs`
  - Mark as `#[cfg(feature = "fabric")]` for future activation

- [ ] 4.3 Document SDF collision algorithm in design notes
  - Sphere marching approach
  - Contact point generation
  - Performance considerations

## 5. CubeObject Integration

- [ ] 5.1 Add `local_aabb()` and `world_aabb()` methods to `CubeObject`
  - `local_aabb()` returns unit AABB [0,1]³
  - `world_aabb()` transforms local AABB using position, rotation, scale
  - Uses OBB→AABB transformation (8 corners)

- [ ] 5.2 Add collision helpers to `CubeObject`
  - Method: `intersects_aabb(other: &Aabb) -> bool`
  - Method: `intersection_region(cube: &Cube, depth: u32) -> Option<IntersectionRegion>`

## 6. Documentation

- [ ] 6.1 Create `doc/architecture/collision.md`
  - Section: Overview of collision system
  - Section: Bounding sphere calculation (with formula)
  - Section: Intersection region algorithm (with diagrams)
  - Section: Face-based collision generation
  - Section: SDF collision (design overview)
  - Section: Performance characteristics

- [ ] 6.2 Update `crates/physics/README.md`
  - Add collision system usage examples
  - Document `CubeCollider` and `ObjectCollider` APIs
  - Add performance tips section

## 7. Testing

- [ ] 7.1 Unit tests for AABB
  - Test unit AABB creation
  - Test world space transformation with rotation
  - Test AABB-AABB intersection
  - Test intersection volume calculation

- [ ] 7.2 Unit tests for intersection region
  - Test AABB inside single octant
  - Test AABB spanning multiple octants
  - Test AABB outside bounds
  - Test edge cases (AABB at octant boundaries)
  - Test rotated AABB (45° rotation expands bounds)

- [ ] 7.3 Integration tests for collision generation
  - Test Cube-CubeObject collision
  - Test CubeObject-CubeObject collision
  - Test rotated objects collision
  - Test with proto-gl falling cubes scenario

- [ ] 7.4 WASM compatibility tests
  - Build physics crate with `--target wasm32-unknown-unknown`
  - Verify no conditional compilation excludes collision code
  - Test collision module exports in wasm bindings

- [ ] 7.5 Performance benchmarks
  - Benchmark: region traversal vs full traversal
  - Benchmark: collision generation time vs cube complexity
  - Target: 70%+ reduction in face visits for typical collisions

## 8. Cleanup

- [ ] 8.1 Remove deprecated full-traversal paths
  - Mark old `from_cube()` as calling `from_cube_region(None)`
  - Add deprecation warning to direct full traversal usage

- [ ] 8.2 Update proto-gl to use new collision API
  - Modify `spawn_cube_objects()` to use `CubeCollider`
  - Verify physics behavior unchanged

## Dependencies

- Tasks 1.x must complete before 2.x (data structures needed for traversal)
- Tasks 2.x must complete before 3.x (traversal needed for collision)
- Tasks 4.x can proceed in parallel (design only)
- Tasks 5.x depend on 1.x and 3.x
- Tasks 6.x can proceed in parallel after 3.x starts
- Tasks 7.x depend on corresponding implementation tasks
- Tasks 8.x must be last
