# Change: Add Octa Cube Rendering with Empty Spaces

## Why

To validate the GPU raytracer's ability to correctly handle sparse octrees and empty space traversal, we need a test scene that demonstrates octree subdivision with intentional gaps. An octa cube (8-voxel configuration in a 2x2x2 pattern) with 2 empty spaces provides a clear validation case.

**Current situation:**
- GPU raytracer implementation complete (from `implement-gpu-raytracer` change)
- No dedicated test scene for sparse octree rendering
- No automated validation of GPU vs CPU renderer output differences

**Impact:** Without proper validation scenes, we cannot confidently verify that the GPU raytracer correctly handles empty spaces and produces identical output to the CPU reference implementation.

**Scope:** Create test scene, render with both GPU and CPU tracers, and generate diff image to validate pixel-perfect equivalence.

## What Changes

### Phase 1: Test Scene Creation
- **`crates/renderer/tests/scenes/`** - Create octa cube test scene
  - Define octree with depth 1 (8 voxels, 2x2x2 subdivision)
  - Set 6 voxels to non-zero values (solid)
  - Set 2 voxels to value 0 (empty spaces)
  - Position at specific world coordinates for consistent rendering
  - Add camera configuration for optimal viewing angle

### Phase 2: Rendering and Validation
- **`crates/renderer/tests/`** - Add validation test
  - Render octa cube scene with CPU raytracer
  - Render same scene with GPU raytracer
  - Generate pixel-by-pixel difference image
  - Assert that diff image shows zero differences
  - Save output images for visual inspection (CPU, GPU, diff)

### Phase 3: Automated Testing
- **Test integration** - Add to test suite
  - Run as part of `cargo test --workspace`
  - Fail test if any pixel differences detected
  - Include tolerance for floating-point precision (if needed)
  - Generate report of any discrepancies

### Not Changed
- GPU or CPU raytracer implementations (already complete)
- Renderer API or camera systems
- Other test scenes or rendering features

## Impact

### Affected Specs
- **NEW**: `octa-cube-rendering` - Spec for octa cube test scene and validation

### Dependencies
- **REQUIRED**: `implement-gpu-raytracer` change must be completed first
- Uses GPU raytracer implementation
- Uses CPU raytracer as reference
- Requires image comparison capability

### Breaking Changes
None - test scene is additive

### Success Criteria
- Octa cube scene renders correctly with both GPU and CPU tracers
- Diff image shows zero pixel differences (pixel-perfect match)
- Empty spaces are correctly skipped by ray traversal
- Solid voxels are correctly hit with proper normals and lighting
- Test runs automatically in CI/test suite
- Visual inspection confirms expected rendering (6 visible cubes, 2 gaps)
