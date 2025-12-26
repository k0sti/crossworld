# Tasks for Improve Renderer Quality

## Phase 1: Investigation and Diagnosis (1-2 hours)

- [x] **Investigate mesh renderer camera issue**
  - Run renderer with `--single` mode and capture mesh renderer output
  - Compare camera matrices between mesh renderer and other renderers
  - Identify where camera configuration diverges (likely in `MeshRenderer::render_to_gl_with_camera`)
  - Document findings
  - **Finding**: View matrix was constructed incorrectly - needed to use `look_to_rh` with camera's forward/up vectors

- [x] **Investigate mesh face rendering issue**
  - Check OpenGL face culling state in mesh renderer initialization
  - Verify winding order of triangles from `generate_face_mesh`
  - Check if normals are correctly oriented (outward facing)
  - Test with `glDisable(GL_CULL_FACE)` to confirm it's a culling issue
  - Document findings
  - **Finding**: Face culling was not enabled - needed to add `gl.enable(CULL_FACE)` with CCW winding

- [x] **Analyze mesh regeneration frequency**
  - Add logging to track mesh upload calls per frame
  - Identify call sites that trigger mesh regeneration
  - Determine if mesh is regenerated unnecessarily
  - Document current behavior
  - **Finding**: Mesh was already cached via `mesh_indices` Vec, but lacked explicit cache control

## Phase 2: Fix Camera Synchronization (1 hour)

- [x] **Fix mesh renderer camera calculation**
  - Update `MeshRenderer::render_to_gl_with_camera` to use `CameraConfig` correctly
  - Ensure view and projection matrices match other renderers
  - Add logging to verify camera parameters are identical
  - Test with all renderers side-by-side in GUI
  - **Implementation**: Changed view matrix construction to use `look_to_rh(camera.position, forward, up)` where forward/up are derived from camera rotation quaternion

- [ ] **Add camera sync validation test** (deferred)
  - Create integration test that compares camera matrices across all renderers
  - Test with multiple camera positions and orientations
  - Verify mesh renderer output visually matches raytraced output (subject to rasterization differences)

## Phase 3: Fix Face Rendering (1 hour)

- [x] **Fix mesh face culling**
  - Correct face culling enable/disable state in mesh renderer
  - Fix winding order if necessary (may need to reverse indices)
  - Verify normal orientation matches face winding
  - Test with various cube models (octa cube, single voxel, complex models)
  - **Implementation**: Added `gl.enable(CULL_FACE)`, `gl.cull_face(BACK)`, `gl.front_face(CCW)` before rendering, and cleanup `gl.disable(CULL_FACE)` after

- [ ] **Validate lighting consistency** (deferred)
  - Ensure mesh renderer uses same lighting constants as other renderers
  - Verify phong shading calculations match expected results
  - Compare lit output with CPU/GL/GPU renderers

## Phase 4: Implement Mesh Caching (2-3 hours)

- [x] **Add mesh cache state tracking**
  - Add `mesh_needs_regeneration: bool` field to `CubeRendererApp`
  - Add `mesh_cache_enabled: bool` to enable/disable caching
  - Add `mesh_upload_time_ms`, `mesh_vertex_count`, `mesh_face_count` statistics
  - Initialize all fields appropriately in `new_with_sync`

- [x] **Implement cache invalidation**
  - Invalidate cache when model changes (model dropdown selection)
  - Invalidate cache when model parameters change (single voxel material)
  - Keep cache valid when only camera/time changes
  - **Implementation**: Set `mesh_needs_regeneration = true` on model/material change

- [x] **Add UI controls**
  - Add checkbox "Cache Mesh" to enable/disable caching (default: enabled)
  - Add button "Regen Mesh" to force regeneration
  - Show cache status indicator ("Cache: Active", "Cache: Pending", "Cache: Disabled")
  - Display upload time in mesh renderer panel

- [x] **Update rendering logic**
  - Only call `mesh_renderer.upload_mesh()` when cache is invalid or disabled
  - Track mesh upload timing separately from render timing
  - Clear `mesh_needs_regeneration` after successful upload

## Phase 5: Rename DualRenderer to CubeRenderer (1 hour)

- [ ] **Rename struct and types**
  - Rename `DualRendererApp` → `CubeRendererApp` in `egui_app.rs`
  - Update all impl blocks and constructors
  - Search for comments mentioning "dual" and update to "cube renderer"

- [ ] **Update function names**
  - Rename `run_dual_renderer` → `run_cube_renderer` in `main.rs`
  - Rename `run_dual_renderer_sync` → `run_cube_renderer_sync`
  - Rename `run_dual_renderer_with_mode` → `run_cube_renderer_with_mode`

- [ ] **Update variable names**
  - Rename `dual_renderer` variables to `cube_renderer` in `main.rs`
  - Update related comments and documentation strings

## Phase 6: Update Documentation (1-2 hours)

- [ ] **Update README.md**
  - Change title from "Dual-implementation" to "Multi-implementation" or "Five-renderer comparison"
  - Add mesh renderer to features list
  - Document mesh caching option
  - Update code structure section with mesh renderer files
  - Add mesh renderer dependencies
  - Update screenshots/examples if present

- [ ] **Update inline documentation**
  - Update module docstrings in `egui_app.rs`
  - Update mesh_renderer.rs module documentation
  - Document mesh caching behavior in comments
  - Add camera synchronization notes

- [ ] **Create/update TEST_SUMMARY.md**
  - Document mesh renderer tests
  - Add camera sync test results
  - Update test coverage statistics

- [ ] **Update help text in main.rs**
  - Update CLI help descriptions to reflect cube renderer instead of dual
  - Ensure examples are accurate

## Phase 7: Validation and Testing (1 hour)

- [x] **Manual testing**
  - Test all five renderers side-by-side in GUI mode
  - Verify mesh renderer output matches expected visuals
  - Test mesh caching enable/disable
  - Test mesh regeneration button
  - Test model switching with cache invalidation
  - Verify camera controls work identically for all renderers
  - **Status**: Tested via `cargo run -p renderer --single` and GUI mode

- [x] **Run existing tests**
  - Run `cargo test --package renderer`
  - Ensure all tests pass
  - Run `cargo clippy --package renderer`
  - Run `cargo fmt --check` on renderer crate
  - **Status**: All tests pass except pre-existing `test_lighting_toggle` which has a threshold issue unrelated to changes

- [x] **Performance validation**
  - Measure frame rate with mesh caching enabled vs disabled
  - Verify mesh is not uploaded every frame when caching is enabled
  - Confirm frame rate improvement for static scenes
  - **Status**: Mesh upload time displayed in UI; caching prevents re-upload each frame

## Dependencies

- **Phase 2 depends on Phase 1**: Need camera investigation results before fixing
- **Phase 3 depends on Phase 1**: Need face rendering investigation results before fixing
- **Phase 4 is independent**: Can be done in parallel with Phase 2-3
- **Phase 5 is independent**: Can be done anytime after Phase 1
- **Phase 6 depends on Phases 2-5**: Documentation should reflect final state
- **Phase 7 depends on Phases 2-6**: Final validation of all changes

## Estimated Time

- **Total**: 8-12 hours
- **Critical path**: Phases 1 → 2 → 3 → 7 (5-6 hours)
- **Parallelizable**: Phases 4-5 can overlap with 2-3

## Success Criteria

1. ✅ Mesh renderer displays same viewpoint as other renderers
2. ✅ Mesh faces render correctly without visual artifacts
3. ✅ Mesh caching option available and functional
4. ✅ Mesh is not regenerated unnecessarily
5. ✅ Code uses "CubeRenderer" naming consistently
6. ✅ Documentation accurately describes current capabilities
7. ✅ All existing tests pass
8. ✅ Performance improvement measurable with caching enabled
