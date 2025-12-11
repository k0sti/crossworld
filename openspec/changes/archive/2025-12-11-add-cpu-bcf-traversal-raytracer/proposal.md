# Change: Add CPU-Based BCF Traversal Raytracer

## Why

We need a CPU implementation of BCF (Binary Cube Format) octree traversal that can be directly translated to GLSL for the GL renderer. The current situation:

**Current State:**
- ✅ BCF format verified with comprehensive tests (`add-bcf-symmetric-readwrite`)
- ✅ Existing CPU raytracer works with native `Cube<T>` structures
- ❌ GL renderer broken - uses wrong coordinate system (grid [0,8) vs octree [-1,1]³)
- ❌ No BCF-based traversal algorithm that maps to GPU operations

**The Problem:**
The existing CPU raytracer (`crates/renderer/src/cpu_tracer.rs`) uses high-level Rust operations:
- Recursive function calls (not GPU-friendly)
- `Rc<Cube<T>>` smart pointers (no equivalent in GLSL)
- Pattern matching on Rust enums (must be if/else chains in GLSL)
- Native coordinate transformations (must be explicit math in GLSL)

**What We Need:**
A CPU raytracer that:
1. Reads BCF binary data directly (byte array)
2. Uses only GPU-compatible operations (bit shifts, masks, pointer arithmetic)
3. Implements iterative traversal (no recursion)
4. Has 1:1 mapping to GLSL fragment shader code
5. Produces identical results to existing CPU raytracer (validation)

**Why This Matters:**
- GL renderer needs working implementation before we can validate GPU correctness
- CPU version is easier to debug than GLSL (printf, breakpoints, unit tests)
- Direct translation ensures GPU code correctness
- Reference implementation documents the algorithm for future maintainers

## What Changes

### Phase 1: BCF Binary Reader Module
- **`crates/cube/src/io/bcf/reader.rs`** (new file)
  - Struct `BcfReader { data: &[u8], offset: usize }`
  - Method `read_u8(&mut self, offset: usize) -> Result<u8, BcfError>`
  - Method `read_pointer(&mut self, offset: usize, ssss: u8) -> Result<usize, BcfError>`
  - Method `decode_type_byte(type_byte: u8) -> (bool, u8, u8)` - Returns (is_extended, type_id, size_bits)
  - Method `read_header(&self) -> Result<BcfHeader, BcfError>`
  - All operations use explicit bounds checking
  - Zero allocations (no Vec, no String)
  - Simple, predictable control flow (maps to GLSL easily)

### Phase 2: BCF Node Type Definitions
- **`crates/cube/src/io/bcf/reader.rs`**
  - Enum `BcfNodeType { InlineLeaf(u8), ExtendedLeaf(u8), OctaLeaves([u8; 8]), OctaPointers { ssss: u8, pointers: [usize; 8] } }`
  - Method `read_node_at(&self, offset: usize) -> Result<BcfNodeType, BcfError>`
  - Handles all BCF node types from specification
  - Returns parsed node structure (not Cube<T>)

### Phase 3: Iterative Octree Traversal Algorithm
- **`crates/renderer/src/bcf_cpu_tracer.rs`** (new file)
  - Struct `BcfCpuTracer { bcf_data: Vec<u8>, bounds: CubeBounds }`
  - Method `trace_ray(bcf_data: &[u8], ray_origin: Vec3, ray_dir: Vec3) -> Option<BcfHit>`
  - Struct `BcfHit { value: u8, normal: Vec3, pos: Vec3 }`
  - Iterative traversal using stack (no recursion)
  - Uses DDA (Digital Differential Analyzer) for ray marching
  - Octant selection via bit operations (same as GLSL will use)
  - Coordinate transformations explicit and documented

### Phase 4: Ray-AABB Intersection
- **`crates/renderer/src/bcf_cpu_tracer.rs`**
  - Function `ray_aabb_intersect(ray_origin: Vec3, ray_dir: Vec3, aabb_min: Vec3, aabb_max: Vec3) -> Option<(f32, f32)>`
  - Returns (t_near, t_far) or None if no intersection
  - Uses slab method (efficient for AABBs)
  - Handle edge cases: ray parallel to axis, ray origin inside AABB
  - Same algorithm as GLSL version will use

### Phase 5: Octant Selection and Traversal
- **`crates/renderer/src/bcf_cpu_tracer.rs`**
  - Function `select_octant(pos: Vec3) -> usize` - Returns 0-7 octant index
  - Function `octant_to_offset(octant: usize, pos_sign: IVec3) -> IVec3` - Maps octant to child offset
  - Function `compute_child_bounds(parent_bounds: AABB, octant: usize) -> AABB`
  - All using explicit bit operations and arithmetic
  - Mirrors GPU implementation exactly

### Phase 6: Validation Tests
- **`crates/renderer/tests/bcf_cpu_tracer_tests.rs`** (new file)
  - Test: BCF tracer vs existing CPU tracer (pixel-by-pixel comparison)
  - Test: Simple solid cube (single voxel)
  - Test: Octa-leaves pattern (8 different colors)
  - Test: Depth-2 octree (64 voxels)
  - Test: Ray miss (no intersection)
  - Test: Ray from inside cube
  - Test: Boundary rays (epsilon testing)
  - Test: Performance comparison (BCF should be comparable or faster)

### Phase 7: Integration with Renderer
- **`crates/renderer/src/bcf_cpu_tracer.rs`**
  - Implement `CubeTracer` trait for `BcfCpuTracer`
  - Methods: `render(&mut self, width: u32, height: u32, time: f32) -> &ImageBuffer`
  - Methods: `render_with_camera(&mut self, width: u32, height: u32, camera: &CameraConfig) -> &ImageBuffer`
  - Same interface as existing `CpuCubeTracer`
  - Drop-in replacement for testing

### Not Changed
- Existing `CpuCubeTracer` (remains as reference implementation)
- BCF serialization/deserialization (`parse_bcf`, `serialize_bcf`)
- Material palette system
- Camera and lighting calculations
- Egui UI and renderer app structure

## Impact

### Affected Specs
- **NEW**: `bcf-raycast-traversal` - BCF-based octree traversal algorithm
- **DEPENDS ON**: `binary-cube-format` (phases 1-4, already complete)
- **DEPENDS ON**: `add-bcf-symmetric-readwrite` (validation tests)

### Affected Code
- `crates/cube/src/io/bcf/reader.rs` - NEW: BCF binary reader with GPU-compatible operations
- `crates/renderer/src/bcf_cpu_tracer.rs` - NEW: CPU raytracer using BCF directly
- `crates/renderer/tests/bcf_cpu_tracer_tests.rs` - NEW: Validation tests
- `crates/renderer/src/lib.rs` - Export new tracer
- `crates/renderer/src/main.rs` - Add CLI option for BCF tracer (optional)

### Benefits
- **Reference implementation** for GLSL translation (1:1 mapping)
- **Validation baseline** - can compare GPU output to CPU output
- **Debuggability** - easier to debug CPU code than GLSL
- **Documentation** - algorithm is explicit and well-commented
- **Performance** - BCF traversal can be faster than Rc pointer chasing
- **Correctness** - proves BCF format works for raycasting before GPU implementation

### Dependencies
- **REQUIRED**: `add-bcf-symmetric-readwrite` (must be complete for trust in BCF format)
- **BLOCKS**: GL renderer BCF integration (needs CPU reference for validation)
- **BLOCKS**: GPU compute shader BCF traversal (same algorithm, different target)

### Success Criteria
- BCF CPU tracer renders identical output to existing CPU tracer (pixel-perfect match)
- All validation tests pass (ray intersection, octant selection, traversal)
- Performance within 2x of existing CPU tracer (preferably faster)
- Code structure maps directly to GLSL (documented translation guide)
- Zero undefined behavior (all bounds checked, no unsafe code)
- Code passes `cargo clippy --workspace -- -D warnings`

### Breaking Changes
None - this is additive (new tracer alongside existing one)

## Implementation Status

### Current State (2025-11-26)
- ✅ BCF format validated with 49 comprehensive tests
- ✅ Existing CPU raytracer works correctly
- ❌ No BCF-based traversal implementation
- ❌ GL renderer still broken

### Timeline Estimate
- Phase 1 (BCF reader): 1 hour
- Phase 2 (Node types): 30 minutes
- Phase 3 (Traversal): 3-4 hours (core algorithm)
- Phase 4 (Ray-AABB): 1 hour
- Phase 5 (Octant logic): 1 hour
- Phase 6 (Validation): 1-2 hours
- Phase 7 (Integration): 1 hour
- **Total**: 8-11 hours

### Risk Assessment
**Medium Risk:**
- Octree traversal algorithm is complex (lots of edge cases)
- Must match existing raytracer exactly (validation can fail)
- Performance requirements may require optimization
- Coordinate system transformations easy to get wrong

**Mitigation:**
- Start with simplest cases (single solid cube)
- Build up complexity incrementally (octa-leaves → octa-pointers → depth-2)
- Extensive testing at each step
- Reference existing CPU raytracer for expected behavior
- Document coordinate transformations clearly

### Next Phase (After This)
**`translate-bcf-traversal-to-glsl`** - Convert CPU BCF tracer to fragment shader:
1. Map Rust types to GLSL types (Vec3 → vec3, usize → uint, etc.)
2. Convert BcfReader methods to GLSL functions
3. Adapt traversal loop to GLSL (no break/continue restrictions)
4. Upload BCF binary to GPU (SSBO or texture buffer)
5. Validate GPU output matches CPU output (pixel diff)
