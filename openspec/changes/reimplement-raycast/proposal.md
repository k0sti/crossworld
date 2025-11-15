# Change: Reimplement Cube Raycast with Comprehensive Tests

## Why

The existing raycast implementation in `crates/cube/src/raycast/mod.rs` has 33 passing tests, but needs a complete reimplementation following the comprehensive design in `docs/raycast.md`. The current implementation should be rebuilt from scratch with proper test coverage for every scenario before integrating with renderer or GPU implementations.

**Scope**: Focus exclusively on the `cube` crate raycast library. Renderer integration is separate future work.

**Approach**: Test-driven reimplementation - write tests first based on design document scenarios, then implement to pass all tests.

## What Changes

- **Reimplement `crates/cube/src/raycast/mod.rs`** from scratch
  - Implement recursive octree traversal using DDA algorithm
  - Add coordinate space transformations for normalized [0,1]³ space
  - Calculate surface normals based on entry face
  - Implement octant indexing and child traversal
  - Add DDA stepping to efficiently skip empty octants
  - Handle all edge cases (axis-aligned rays, grazing rays, depth limits)
  - Protect against floating-point precision issues
- **Comprehensive test suite** in `crates/cube/src/raycast/tests.rs` or `crates/cube/tests/`
  - Unit tests for octant indexing calculation
  - Tests for DDA stepping algorithm
  - Tests for normal calculation (min face vs max face entry)
  - Tests for coordinate transformations
  - Edge case tests: axis-aligned rays, grazing rays, corners
  - Depth limit tests (depth 0, 1, 2, 3+)
  - Division-by-zero protection tests
  - Visual correctness tests with known octree patterns
  - Expand from 33 to 50+ comprehensive tests covering all scenarios

## Impact

### Affected Specs
- **NEW**: `voxel-raycast` - Raycast system specification for cube crate

### Affected Code
- `crates/cube/src/raycast/mod.rs` - Complete reimplementation
- `crates/cube/src/raycast/` - May split into submodules (e.g., `dda.rs`, `normals.rs`)
- `crates/cube/tests/raycast_tests.rs` or `crates/cube/src/raycast/tests.rs` - Expanded test suite

### Not Affected (Future Work)
- `crates/renderer/` - Renderer integration is separate change
- `crates/renderer/src/gpu_tracer.rs` - GPU implementation is separate change
- Other cube modules - Only raycast module changes

### Dependencies
- Uses existing `Cube<T>` enum and octree structures from cube crate
- Uses existing `glam` math library (Vec3, IVec3)
- Uses existing `CubeCoord` structure
- No new external dependencies required

### Testing Scope
- **Expanded coverage**: From 33 tests to 50+ tests
- **Test-driven development**: Write tests first, implement to pass
- **Test categories**:
  - Unit tests for helper functions (octant indexing, DDA stepping, normals)
  - Integration tests for complete raycast algorithm
  - Edge case tests (boundary conditions, precision issues)
  - Performance benchmarks (optional, for future optimization)

### Public API Changes
- May refine `RaycastHit` structure fields
- May add helper functions for common operations
- Core `Cube::raycast()` signature remains compatible
- All changes maintain backward compatibility with existing 33 tests

### Breaking Changes
None - existing tests must continue to pass

### Success Criteria
- All 33 existing tests pass
- 20+ new tests added covering design document scenarios
- All scenarios from `docs/raycast.md` have corresponding test
- Code passes `cargo clippy` with no warnings
- Documentation updated with algorithm explanations
