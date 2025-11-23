## 1. WebGL 2.0 Fragment Shader Implementation (Moved to gl_tracer.rs)
- [x] 1.1 Create shader directory structure (`crates/renderer/src/shaders/`)
- [x] 1.2 Implement octree data structure in shader
- [x] 1.3 Port DDA octree traversal algorithm to GLSL
- [x] 1.4 Implement voxel value lookup in shader
- [x] 1.5 Add empty space detection (voxel value == 0)
- [x] 1.6 Calculate surface normals from entry face
- [x] 1.7 Add coordinate space transformations
- [x] 1.8 Implement depth limiting
- [x] 1.9 Add shader compilation and validation

## 2. WebGL 2.0 Tracer Integration (Moved to gl_tracer.rs)
- [x] 2.1 Refactor fragment shader implementation into `gl_tracer.rs`
- [x] 2.2 Setup shader program initialization
- [x] 2.3 Implement octree data upload to GPU (3D texture)
- [x] 2.4 Create framebuffer for raycast output
- [x] 2.5 Implement ray generation from camera
- [x] 2.6 Add lighting calculations (matching CPU tracer)
- [x] 2.7 Add render loop integration

## 3. Architecture Refactoring (3-Tracer System)
- [x] 3.1 Create `shader_utils.rs` for shared shader compilation
- [x] 3.2 Refactor `gl_tracer.rs` as WebGL 2.0 fragment shader tracer
- [x] 3.3 Reimplement `gpu_tracer.rs` as compute shader stub
- [x] 3.4 Update `cpu_tracer.rs` to use `cube.raycast()` directly
- [x] 3.5 Redesign `egui_app.rs` with 2x2 grid layout
- [x] 3.6 Add dropdown ComboBoxes for diff source selection
- [x] 3.7 Implement camera controls for all views
- [x] 3.8 Display performance metrics for each renderer

## 4. Testing and Validation
- [x] 4.1 Create test scene with octa cube (2x2x2 octree)
- [x] 4.2 Render scene with CPU, GL, and GPU tracers
- [x] 4.3 Implement pixel diff comparison with 10x amplification
- [x] 4.4 Add dropdown selection for diff comparison sources
- [x] 4.5 Test in single-frame mode (verified working)
- [ ] 4.6 Add performance benchmarks (basic metrics displayed)
- [ ] 4.7 Test various octree depths (0, 1, 2, 3+)
- [ ] 4.8 Test edge cases (empty octrees, single voxels, etc.)

## 5. Code Quality
- [x] 5.1 Run `cargo clippy` and fix warnings
- [x] 5.2 Run `cargo fmt`
- [x] 5.3 Add code documentation
- [x] 5.4 Update relevant comments
- [x] 5.5 Create git commit with changes

## 6. GPU Tracer Phase 1: Basic Ray-Cube Intersection (NEW)
- [x] 6.1 Create basic compute shader (`basic_raycast.comp`)
- [x] 6.2 Implement ray-box intersection using slab method
- [x] 6.3 Add Blinn-Phong lighting calculation
- [x] 6.4 Support time-based orbit camera
- [x] 6.5 Support explicit camera with quaternion rotation
- [x] 6.6 Create blit shader for displaying compute shader output
- [x] 6.7 Implement GpuTracerGl struct with compute shader resources
- [x] 6.8 Implement init_gl() for shader compilation and texture creation
- [x] 6.9 Implement render_to_gl() for compute shader dispatch
- [x] 6.10 Implement render_to_gl_with_camera() for explicit camera
- [x] 6.11 Implement blit_texture_to_screen() for display
- [x] 6.12 Implement destroy_gl() for resource cleanup
- [ ] 6.13 Test GPU tracer in egui app (integration pending)
- [ ] 6.14 Verify ray-cube hit detection working

## 7. CPU-Side Raycast Testing (NEW - 2025-11-23)
- [x] 7.1 Add `raycast_octree()` method to GlCubeTracer using cube.raycast_debug()
- [x] 7.2 Add `raycast_octree()` method to GpuTracer using cube.raycast_debug()
- [x] 7.3 Update raycast test report to use real tracer implementations
- [x] 7.4 Wire up all three tracers (CPU, GL, GPU) to test framework
- [x] 7.5 Verify all 16 raycast tests pass for all 3 tracers
- [x] 7.6 Update test report summary to show results for all tracers

## Status Notes

**Completed (2025-11-18):**
- WebGL 2.0 fragment shader raytracer moved to `gl_tracer.rs`
- Compute shader stub created in `gpu_tracer.rs` (not yet implemented)
- 2x2 grid UI with CPU | GL | GPU | Diff comparison views
- Shared shader utilities in `shader_utils.rs`
- All three tracers rendering simultaneously with camera controls
- Commit: `6c2f590` - Refactor renderer into 3-tracer architecture with 2x2 grid UI

**Completed Phase 1 (2025-11-18):**
- GPU tracer Phase 1 implementation: Basic ray-cube bounding box intersection
- Compute shader (`basic_raycast.comp`) with ray-box slab method
- Full rendering pipeline with compute shader dispatch (8x8 work groups)
- Texture blit using fullscreen triangle for display
- Camera support (time-based orbit + explicit quaternion)
- Proper resource management (init, render, destroy)
- Files modified:
  - `crates/renderer/src/shaders/basic_raycast.comp` (new)
  - `crates/renderer/src/gpu_tracer.rs` (implemented from stub)
  - Added blit shaders (vertex + fragment) for texture display

**Completed CPU-Side Testing (2025-11-23):**
- Added `raycast_octree()` methods to GL and GPU tracers for CPU-side testing
- Wired up all three tracers to comprehensive raycast test report
- All 16 tests passing for CPU, GL, and GPU tracers
- Test report validates: axis-aligned rays, diagonal rays, boundary misses, edge cases
- Commits:
  - `d392da8` - Wire up all three tracers (CPU, GL, GPU) to raycast test report

**Pending:**
- GPU tracer integration into egui app (test Phase 1)
- Phase 2: Full octree traversal in compute shader
- Comprehensive edge case testing (4.7, 4.8)
- Performance benchmarks beyond basic timing (4.6)
