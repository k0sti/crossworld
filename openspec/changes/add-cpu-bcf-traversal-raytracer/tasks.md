# Implementation Tasks

## 1. BCF Binary Reader Module
- [ ] 1.1 Create `crates/cube/src/io/bcf/reader.rs`
- [ ] 1.2 Define `BcfReader` struct with `data: &[u8]` field
- [ ] 1.3 Implement `read_u8(offset: usize) -> Result<u8, BcfError>` with bounds checking
- [ ] 1.4 Implement `read_u16_le(offset: usize) -> Result<u16, BcfError>` for 2-byte pointers
- [ ] 1.5 Implement `read_u32_le(offset: usize) -> Result<u32, BcfError>` for 4-byte pointers
- [ ] 1.6 Implement `read_u64_le(offset: usize) -> Result<u64, BcfError>` for 8-byte pointers
- [ ] 1.7 Implement `read_pointer(offset: usize, ssss: u8) -> Result<usize, BcfError>` - dispatch to correct size
- [ ] 1.8 Implement `decode_type_byte(type_byte: u8) -> (bool, u8, u8)` using bit operations
- [ ] 1.9 Implement `read_header() -> Result<BcfHeader, BcfError>` - parse magic, version, root offset
- [ ] 1.10 Add unit tests for reader methods
- [ ] 1.11 Export reader module in `crates/cube/src/io/bcf/mod.rs`

## 2. BCF Node Type Definitions
- [ ] 2.1 Define `BcfNodeType` enum in `reader.rs`: InlineLeaf, ExtendedLeaf, OctaLeaves, OctaPointers
- [ ] 2.2 Implement `read_node_at(offset: usize) -> Result<BcfNodeType, BcfError>`
- [ ] 2.3 Handle inline leaf nodes (type byte 0x00-0x7F)
- [ ] 2.4 Handle extended leaf nodes (type byte 0x80-0x8F)
- [ ] 2.5 Handle octa-leaves nodes (type byte 0x90-0x9F) - read 8 value bytes
- [ ] 2.6 Handle octa-pointers nodes (type byte 0xA0-0xAF) - read 8 pointers of size 2^SSSS
- [ ] 2.7 Add error handling for unknown node types
- [ ] 2.8 Add unit tests for node parsing

## 3. Ray-AABB Intersection
- [ ] 3.1 Create `crates/renderer/src/bcf_cpu_tracer.rs`
- [ ] 3.2 Define `AABB` struct: `{ min: Vec3, max: Vec3 }`
- [ ] 3.3 Implement `ray_aabb_intersect(origin: Vec3, dir: Vec3, aabb: AABB) -> Option<(f32, f32)>`
- [ ] 3.4 Use slab method: compute t for each axis pair
- [ ] 3.5 Handle edge case: ray direction component = 0 (parallel to axis)
- [ ] 3.6 Handle edge case: ray origin inside AABB
- [ ] 3.7 Return (t_near, t_far) where ray enters/exits box
- [ ] 3.8 Add unit tests for ray-AABB intersection

## 4. Octant Selection Logic
- [ ] 4.1 Implement `select_octant(pos: Vec3) -> usize` - map position to 0-7 octant index
- [ ] 4.2 Use bit operations: `(x>0)<<2 | (y>0)<<1 | (z>0)`
- [ ] 4.3 Implement `octant_bounds(parent: AABB, octant: usize) -> AABB` - compute child AABB
- [ ] 4.4 Implement `octant_center(parent: AABB, octant: usize) -> Vec3` - compute child center
- [ ] 4.5 Add unit tests for octant logic

## 5. Iterative Octree Traversal
- [ ] 5.1 Define `TraversalState` struct: `{ offset: usize, bounds: AABB, depth: u8 }`
- [ ] 5.2 Define `BcfHit` struct: `{ value: u8, normal: Vec3, pos: Vec3, distance: f32 }`
- [ ] 5.3 Implement `trace_ray(bcf_data: &[u8], ray_origin: Vec3, ray_dir: Vec3, max_depth: u8) -> Option<BcfHit>`
- [ ] 5.4 Initialize with root node (offset from header, bounds [-1,1]³)
- [ ] 5.5 Check ray-AABB intersection for current node
- [ ] 5.6 If no intersection, return None (ray miss)
- [ ] 5.7 Read node type at current offset
- [ ] 5.8 If leaf node: check if non-zero value, return hit or miss
- [ ] 5.9 If octa node: select octant based on ray entry point
- [ ] 5.10 Push selected child to traversal stack
- [ ] 5.11 Loop until stack empty or hit found
- [ ] 5.12 Compute hit normal from entry face
- [ ] 5.13 Add bounds checking and error handling

## 6. BcfCpuTracer Integration
- [ ] 6.1 Define `BcfCpuTracer` struct: `{ bcf_data: Vec<u8>, bounds: CubeBounds, image_buffer: Option<ImageBuffer> }`
- [ ] 6.2 Implement `new_from_cube(cube: &Cube<u8>) -> Self` - serialize cube to BCF
- [ ] 6.3 Implement `new_from_bcf(bcf_data: Vec<u8>) -> Self` - use existing BCF
- [ ] 6.4 Implement `render(&mut self, width: u32, height: u32, time: f32) -> &ImageBuffer`
- [ ] 6.5 Implement `render_with_camera(&mut self, width, height, camera) -> &ImageBuffer`
- [ ] 6.6 For each pixel: create ray, call `trace_ray`, convert hit to color
- [ ] 6.7 Apply lighting (simple diffuse based on normal)
- [ ] 6.8 Convert material index to RGB using palette
- [ ] 6.9 Store result in image buffer

## 7. Validation Tests
- [ ] 7.1 Create `crates/renderer/tests/bcf_cpu_tracer_tests.rs`
- [ ] 7.2 Test: Single solid cube (Cube::Solid(42)) renders correctly
- [ ] 7.3 Test: Octa-leaves (8 different colors) renders 8 colored octants
- [ ] 7.4 Test: Compare BCF tracer vs existing CPU tracer (pixel-by-pixel)
- [ ] 7.5 Test: Ray miss (background color)
- [ ] 7.6 Test: Ray from inside cube
- [ ] 7.7 Test: Boundary conditions (ray exactly on octant border)
- [ ] 7.8 Test: Depth-2 octree (more complex scene)
- [ ] 7.9 Benchmark: BCF tracer vs CPU tracer performance
- [ ] 7.10 Run tests: `cargo test --test bcf_cpu_tracer_tests`

## 8. Documentation and Translation Guide
- [ ] 8.1 Add module-level doc comments to `bcf_cpu_tracer.rs`
- [ ] 8.2 Document coordinate system transformations (world → node local)
- [ ] 8.3 Document octant indexing scheme (x*4 + y*2 + z)
- [ ] 8.4 Document BCF node type encoding
- [ ] 8.5 Create translation guide: Rust → GLSL mapping
- [ ] 8.6 Document which operations map 1:1 to GLSL
- [ ] 8.7 Note GLSL limitations (no recursion, limited stack)

## 9. Integration and Validation
- [ ] 9.1 Export `BcfCpuTracer` in `crates/renderer/src/lib.rs`
- [ ] 9.2 Add CLI option to renderer binary: `--tracer bcf-cpu`
- [ ] 9.3 Run full test suite: `cargo test --workspace`
- [ ] 9.4 Run clippy: `cargo clippy --workspace -- -D warnings`
- [ ] 9.5 Verify visual output matches existing CPU tracer
- [ ] 9.6 Measure performance (render time per frame)
- [ ] 9.7 Commit changes with message: "feat: Add CPU-based BCF traversal raytracer"
