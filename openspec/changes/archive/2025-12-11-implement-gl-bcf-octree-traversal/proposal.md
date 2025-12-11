# Change: Implement GL Fragment Shader BCF Octree Traversal

## Why

The GL fragment shader (`octree_raycast.frag`) has a broken implementation of BCF octree traversal that uses an incorrect AABB bounds-tracking approach. Meanwhile, the BCF CPU tracer (`bcf_raycast.rs`) has a proven, working implementation using [-1,1]³ normalized space. We need to replace the broken GL implementation with a correct one based directly on the CPU BCF tracer algorithm.

**Current situation:**
- `bcf_raycast.rs` - Working BCF octree traversal using [-1,1]³ normalized space (CPU-side, proven correct)
- `octree_raycast.frag` - Broken GL implementation using AABB bounds tracking (produces incorrect results)
- `bcf_cpu_tracer.rs` - Documented GPU translation guide showing how to port to GLSL
- GL tracer produces incorrect rendering due to flawed traversal algorithm

**Problem:** The existing GL shader implementation is fundamentally broken. It uses AABB bounds tracking which doesn't correctly handle the ray-space transformations required for hierarchical octree descent. This produces rendering artifacts and incorrect voxel geometry.

**Impact:** GPU rendering is broken and cannot be used for voxel scenes. The GL tracer must be replaced with a correct implementation before it can be useful.

**Scope:** Replace the broken GL implementation with a clean port of `bcf_raycast.rs` algorithm to GLSL, using [-1,1]³ normalized space approach for correct hierarchical traversal. Achieve pixel-perfect equivalence with CPU implementation.

## What Changes

### Phase 1: Remove Broken Implementation
- **`crates/renderer/src/shaders/octree_raycast.frag`** - Remove broken AABB-based traversal
  - Remove broken `raycastBcfOctree` function (lines 531-719)
  - Keep only BCF reading infrastructure (readU8, readPointer, parseBcfNode)
  - Keep helper functions (sign3, computeOctant, octantToIndex, minTimeAxis)
  - Prepare clean slate for correct implementation

### Phase 2: Implement Correct Algorithm from bcf_raycast.rs
- **`crates/renderer/src/shaders/octree_raycast.frag`** - Port bcf_raycast_impl algorithm
  - Use [-1,1]³ normalized space at each octree level (matching CPU)
  - Transform ray to child local space when descending: `child_origin = ray_origin * 2.0 - offset`
  - Stack stores: offset, local_origin, ray_dir, normal, coord (NO bounds)
  - DDA octant stepping in normalized space
  - Handle all BCF node types: inline leaf, extended leaf, octa-leaves, octa-pointers
  - Use fixed iteration bounds (`MAX_ITERATIONS = 256`, `MAX_STACK_DEPTH = 16`)

### Phase 3: Stack Management with Correct State
- **`crates/renderer/src/shaders/octree_raycast.frag`** - Normalized-space stack
  - Define `TraversalState` struct matching bcf_raycast.rs:
    - `uint offset` - BCF data offset
    - `vec3 local_origin` - Ray origin in THIS node's [-1,1]³ space
    - `vec3 ray_dir` - Ray direction (unchanged across levels)
    - `int normal` - Entry normal (encoded Axis)
    - `uvec3 coord` - Octree coordinate for this node
  - Fixed-size arrays for each field (MAX_STACK_DEPTH = 16)
  - Stack pointer tracking (0 = empty)
  - NO AABB bounds tracking (bounds are always [-1,1]³ in local space)

### Phase 4: Testing and Validation
- **Validation approach** - Pixel-perfect equivalence with CPU
  - Render same BCF scene with CPU and GL tracers
  - Compare output pixel-by-pixel using existing diff view
  - Test all model depths (0, 1, 2, 3) from `default_models.rs`
  - Verify error materials displayed correctly
  - Validate normal calculation matches CPU

### Not Changed
- CPU BCF raycast implementation (remains as reference)
- BCF binary format specification
- GL tracer infrastructure (texture upload, shader compilation)
- Material palette system
- Lighting calculations
- Camera controls

## Impact

### Affected Specs
- **MODIFIED**: `renderer-raycast` - Extend with GL BCF octree traversal requirements
- **ADDED**: `gl-bcf-octree-raycast` - New spec for GLSL BCF traversal algorithm
- **MODIFIED**: `gl-error-coloring` - May add new error codes for stack/traversal issues

### Dependencies
- **Requires**: `add-cpu-bcf-traversal-raytracer` (84/85 tasks complete) - CPU reference implementation
- **Requires**: `implement-gpu-raytracer` (52/57 tasks) - GL infrastructure and shader compilation
- **Uses**: Existing BCF reader in `cube::io::bcf`
- **Uses**: Existing GL tracer texture upload in `gl_tracer.rs`

### Breaking Changes
None - This is additive functionality. GL tracer will transition from simple box rendering to full octree traversal.

### Success Criteria
- GL fragment shader renders identical output to CPU BCF raycast for all test models
- Pixel diff between GL and CPU shows zero difference (or < 0.1% due to floating-point)
- All depth levels (0-3) render correctly
- Empty voxels (value == 0) are skipped correctly
- Error materials (1-7) display when traversal fails
- Performance: GL tracer at least 10x faster than CPU for 512x512 render
- Code passes `cargo clippy` with no warnings
- Shader compiles without errors on WebGL 2.0
