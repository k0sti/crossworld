# Change: Implement GPU Raytracer with Cube Raytracing

## Why

The renderer currently only supports CPU-based raytracing through `cpu_tracer.rs`, which integrates the cube crate's raycast implementation. To enable high-performance voxel rendering in the browser, we need a GPU-based raytracer that can leverage WebGL/WebGPU compute capabilities.

**Current situation:**
- `cpu_tracer.rs` - Uses `cube::Cube::raycast()` for CPU-based raytracing
- `gpu_tracer.rs` - Contains only stub implementation
- No GPU shader implementation for octree traversal

**Impact:** Without GPU raytracing, rendering performance is limited by JavaScript/WASM CPU execution speed, making real-time voxel rendering at higher resolutions impractical.

**Scope:** Implement GPU raytracer with shader-based octree traversal, matching the functionality of the CPU raytracer but optimized for parallel execution on the GPU.

## What Changes

### Phase 1: GPU Shader Implementation
- **`crates/renderer/src/shaders/`** - Create raycast shader
  - Implement octree traversal in GLSL/WGSL
  - Port cube raycast DDA algorithm to shader code
  - Add voxel value lookup and empty space detection
  - Calculate surface normals from entry face
  - Support configurable max depth for traversal
  - Handle coordinate space transformations (world space ↔ normalized [0,1]³)

### Phase 2: GPU Tracer Integration
- **`crates/renderer/src/gpu_tracer.rs`** - Implement GPU raytracer
  - Replace stub implementation with actual GPU integration
  - Setup shader program and uniforms
  - Implement octree data upload to GPU (texture or buffer)
  - Create framebuffer for raycast output
  - Add lighting calculations (same as CPU tracer)
  - Support same API as CPU tracer for consistency

### Phase 3: Testing and Validation
- **Validation approach** - Compare GPU vs CPU output
  - Render same scene with both CPU and GPU tracers
  - Generate diff image to verify pixel-perfect match
  - Add test scenes with known expected outputs
  - Benchmark performance improvements

### Not Changed
- CPU tracer implementation (remains as reference)
- Cube crate raycast library (already complete)
- Renderer API or camera systems
- Lighting model (matches CPU implementation)

## Impact

### Affected Specs
- **NEW**: `gpu-raytracer` - Spec for GPU-based raytracing with cube octrees

### Dependencies
- Requires completed `reimplement-raycast` change (for algorithm reference)
- Requires completed `integrate-cube-raycast` change (for CPU reference implementation)
- Uses WebGL/WebGPU APIs available in browser

### Breaking Changes
None - GPU tracer is additive, CPU tracer remains available

### Success Criteria
- GPU tracer produces identical output to CPU tracer (verified by diff)
- Performance improvement of 10x or more for typical scenes
- Handles all octree depths correctly
- Correctly renders empty spaces (voxel value == 0)
- All shader code passes validation
- Code passes clippy with no warnings
