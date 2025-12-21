# Tasks: Optimize World Collision Performance

## 1. WorldCollider Trait and Infrastructure

- [x] 1.1 Create `crates/physics/src/world_collider.rs` module
  - Define `WorldCollider` trait with `init()`, `update()`, `resolve_collision()`, `metrics()`
  - Define `ColliderMetrics` struct
  - Export from `crates/physics/src/lib.rs`

- [x] 1.2 Add `remove_collider()` method to `PhysicsWorld`
  - Wrap `collider_set.remove()` with proper cleanup
  - Required for dynamic chunk loading/unloading

- [x] 1.3 Add helper to count compound shape children
  - `count_compound_shapes(collider: &Collider) -> usize`
  - Used by metrics collection

## 2. Monolithic Strategy (Baseline)

- [x] 2.1 Implement `MonolithicCollider` struct
  - Wrap existing `VoxelColliderBuilder::from_cube_scaled()` logic
  - Store world_body and world_collider handles
  - Implement `WorldCollider` trait

- [x] 2.2 Add init timing measurement
  - Use `std::time::Instant` for init duration
  - Store in metrics

- [x] 2.3 Unit test: monolithic creates expected collider count
  - Test with known cube, verify face count matches
  - **Added test_monolithic_metrics, test_count_compound_shapes**

## 3. Chunked Strategy

- [x] 3.1 Implement `ChunkedCollider` struct
  - Fields: cube, world_size, chunk_size, load_radius
  - HashMap<IVec3, ChunkData> for active chunks
  - world_body handle for all chunks

- [x] 3.2 Implement chunk position calculation
  - `world_to_chunk(pos: Vec3) -> IVec3`
  - `chunks_in_aabb(aabb: &Aabb) -> impl Iterator<Item = IVec3>`
  - Unit tests for edge cases
  - **Added test_chunked_world_to_chunk, test_chunked_chunks_in_aabb**

- [x] 3.3 Implement `generate_chunk()`
  - Convert chunk position to world AABB
  - Convert to octree local space
  - Use `VoxelColliderBuilder::from_cube_with_region_scaled()`
  - Return None for empty chunks

- [x] 3.4 Implement `update()` for chunk loading/unloading
  - Collect required chunks from dynamic AABBs
  - Unload chunks not in required set
  - Load new chunks
  - Track metrics (chunks_loaded, chunks_unloaded, faces_generated)

- [ ] 3.5 Unit test: chunks load when objects approach
  - Create object at known position
  - Verify expected chunks are loaded

- [ ] 3.6 Unit test: chunks unload when objects leave
  - Move object away from loaded chunks
  - Verify chunks are unloaded

## 4. Hybrid Octree Strategy

- [x] 4.1 Implement `HybridOctreeCollider` struct
  - Fields: cube, world_size, border_materials
  - No Rapier colliders for world

- [x] 4.2 Implement `resolve_collision()`
  - Get body AABB
  - Convert to RegionBounds
  - Use `visit_faces_in_region()` to query faces
  - Compute and return penetration correction
  - **Fixed bug: face center was computed as voxel center, not actual face surface position**
  - **Fixed bug: corrections from multiple coplanar faces were summed, now takes max per axis**

- [x] 4.3 Implement box-face penetration test
  - `box_face_penetration(box_aabb: &Aabb, face: &FaceInfo) -> Option<Penetration>`
  - Handle all 6 face orientations
  - Return normal and depth
  - **Fixed bug: penetration direction was inverted, now correctly pushes in direction of face normal**

- [x] 4.4 Integrate resolution into physics loop
  - After `physics.step()`, call `resolve_collision()` for each body
  - Apply correction to body position
  - **Added velocity dampening to prevent continuous falling**

- [x] 4.5 Unit test: penetration detection
  - Box overlapping face returns correct normal/depth
  - Box not overlapping returns None
  - **Added tests: test_box_face_penetration_upward_facing, test_box_face_penetration_no_overlap, test_box_face_penetration_outside_face_extent**
  - **Added integration test: test_hybrid_resolve_collision_with_solid_cube**
  - **Added test_hybrid_with_half_solid_world, test_hybrid_query_depth_scaling**
  - **Added test_face_info_position_debug for debugging face positions**

- [ ] 4.6 Unit test: octree query efficiency
  - Compare face count from region query vs full traversal
  - Verify significant reduction

## 5. Configuration Integration

- [x] 5.1 Add physics config to proto-gl
  - Add `world_collision_strategy: String` to PhysicsConfig
  - Add `ChunkedConfig { chunk_size, load_radius }` struct
  - Parse from config.toml

- [x] 5.2 Create strategy factory function
  - `create_world_collider(config: &PhysicsConfig) -> Box<dyn WorldCollider>`
  - Return appropriate implementation based on config
  - **Added test_create_world_collider**

- [x] 5.3 Update config.toml with example configuration
  - Document all strategy options
  - Add chunked strategy parameters

## 6. Proto-GL Integration

- [x] 6.1 Refactor `App::init()` to use WorldCollider
  - Replace direct `VoxelColliderBuilder` call
  - Store `Box<dyn WorldCollider>` in App state

- [x] 6.2 Add `collect_dynamic_aabbs()` helper
  - Iterate CubeObjects, extract body handles
  - Query body positions, compute AABBs
  - Return Vec<(RigidBodyHandle, Aabb)>

- [ ] 6.3 Call `update()` before physics step
  - In main loop, after input handling
  - Pass dynamic AABBs

- [x] 6.4 Call `resolve_collision()` after physics step (hybrid only)
  - Only for HybridOctreeCollider
  - Apply corrections to body positions

- [ ] 6.5 Display metrics in debug overlay
  - Show active_colliders, total_faces, update_time
  - Toggle with debug key

## 7. Benchmarking

- [ ] 7.1 Create benchmark test in `crates/physics/tests/collision_benchmark.rs`
  - Generate test world (depth 10 terrain)
  - Spawn 100 dynamic bodies
  - Run 300 frames for each strategy

- [ ] 7.2 Implement benchmark harness
  - Measure init time
  - Measure per-frame update + step time
  - Collect and aggregate metrics

- [ ] 7.3 Format benchmark output
  - Table with strategy name, init_time, avg_frame_time, speedup
  - Log to console and optionally to file

- [ ] 7.4 Add cargo bench integration
  - Use criterion or similar
  - Enable `cargo bench --bench collision`

## 8. Documentation

- [ ] 8.1 Update `doc/architecture/physics.md`
  - Add section on world collision strategies
  - Explain when to use each strategy
  - Document performance characteristics

- [ ] 8.2 Add inline documentation
  - Document `WorldCollider` trait
  - Document each strategy's design rationale
  - Document configuration options

## 9. Cleanup and Polish

- [ ] 9.1 Run `cargo clippy` and fix warnings
- [ ] 9.2 Run `cargo fmt`
- [ ] 9.3 Verify all tests pass: `cargo test --workspace`
- [ ] 9.4 Build in release mode and verify performance gains

## Dependencies

- Tasks 1.x must complete before 2.x-4.x (trait needed for implementations)
- Tasks 2.x, 3.x, 4.x can proceed in parallel after 1.x
- Tasks 5.x depend on at least one strategy implementation
- Tasks 6.x depend on 5.x
- Tasks 7.x depend on 2.x, 3.x, 4.x (need all strategies for comparison)
- Tasks 8.x, 9.x can proceed in parallel after 6.x

## Parallelizable Work

After task 1.x completes, three developers could work in parallel:
- Developer A: Monolithic strategy (2.x) + benchmarking (7.x)
- Developer B: Chunked strategy (3.x)
- Developer C: Hybrid strategy (4.x)

Then converge on:
- Configuration (5.x)
- Proto-GL integration (6.x)
- Documentation and cleanup (8.x, 9.x)
