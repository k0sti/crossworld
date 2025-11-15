# Implementation Tasks - Cube Raycast Reimplementation

## 0. Preparation
- [ ] 0.1 Read `docs/raycast.md` design document thoroughly
- [ ] 0.2 Review existing `crates/cube/src/raycast/mod.rs` implementation
- [ ] 0.3 Run existing tests to establish baseline: `cargo test -p cube --test raycast`
- [ ] 0.4 Document current test count and coverage
- [ ] 0.5 Create implementation branch from main

## 1. Test Infrastructure Setup
- [ ] 1.1 Set up test file structure (decide: inline tests vs `tests/` directory)
- [ ] 1.2 Create test helper functions for building test octrees
- [ ] 1.3 Add test helper for creating rays with specific properties
- [ ] 1.4 Add test assertion helpers for comparing hit results
- [ ] 1.5 Set up test fixtures for common octree patterns (single voxel, subdivided, etc.)

## 2. Write Tests for Core Algorithm (TDD)
Following test-driven development: write these tests BEFORE implementation

### 2.1 Basic Raycast Tests
- [ ] 2.1.1 Test: Ray hits solid voxel at depth 0
- [ ] 2.1.2 Test: Ray misses empty voxel (value = 0)
- [ ] 2.1.3 Test: Ray through center of cube hits solid voxel
- [ ] 2.1.4 Test: Ray from outside cube bounds misses

### 2.2 Octant Indexing Tests
- [ ] 2.2.1 Test: Octant index calculation for all 8 octants
- [ ] 2.2.2 Test: Octant selection with positive ray directions
- [ ] 2.2.3 Test: Octant selection with negative ray directions
- [ ] 2.2.4 Test: Octant selection with mixed direction signs

### 2.3 Subdivision Traversal Tests
- [ ] 2.3.1 Test: Traverse into subdivided cube (depth 1)
- [ ] 2.3.2 Test: Traverse through multiple octants at same level
- [ ] 2.3.3 Test: Recursive traversal to depth 2
- [ ] 2.3.4 Test: Recursive traversal to depth 3
- [ ] 2.3.5 Test: Hit solid voxel in deep octree (depth 4+)

### 2.4 Normal Calculation Tests
- [ ] 2.4.1 Test: Normal from minimum X face (-1, 0, 0)
- [ ] 2.4.2 Test: Normal from maximum X face (1, 0, 0)
- [ ] 2.4.3 Test: Normal from minimum Y face (0, -1, 0)
- [ ] 2.4.4 Test: Normal from maximum Y face (0, 1, 0)
- [ ] 2.4.5 Test: Normal from minimum Z face (0, 0, -1)
- [ ] 2.4.6 Test: Normal from maximum Z face (0, 0, 1)
- [ ] 2.4.7 Test: Normal at corner entry (should pick dominant face)
- [ ] 2.4.8 Test: Normal at edge entry (should pick dominant face)

### 2.5 Coordinate Transform Tests
- [ ] 2.5.1 Test: Parent to child coordinate transformation
- [ ] 2.5.2 Test: Position scaling (pos * 2.0 - octant_bit)
- [ ] 2.5.3 Test: Octree coordinate path updates correctly
- [ ] 2.5.4 Test: Round-trip transform (parent → child → parent)

### 2.6 DDA Stepping Tests
- [ ] 2.6.1 Test: Calculate next octant boundary (positive direction)
- [ ] 2.6.2 Test: Calculate next octant boundary (negative direction)
- [ ] 2.6.3 Test: Step to next X boundary
- [ ] 2.6.4 Test: Step to next Y boundary
- [ ] 2.6.5 Test: Step to next Z boundary
- [ ] 2.6.6 Test: Step when multiple boundaries equidistant

### 2.7 Edge Case Tests
- [ ] 2.7.1 Test: Axis-aligned ray in +X direction
- [ ] 2.7.2 Test: Axis-aligned ray in -X direction
- [ ] 2.7.3 Test: Axis-aligned ray in +Y direction
- [ ] 2.7.4 Test: Axis-aligned ray in -Y direction
- [ ] 2.7.5 Test: Axis-aligned ray in +Z direction
- [ ] 2.7.6 Test: Axis-aligned ray in -Z direction
- [ ] 2.7.7 Test: Grazing ray (tangent to face)
- [ ] 2.7.8 Test: Ray with very small direction component (near-zero)
- [ ] 2.7.9 Test: Ray starting exactly on octant boundary
- [ ] 2.7.10 Test: Ray starting at cube corner

### 2.8 Depth Limit Tests
- [ ] 2.8.1 Test: Depth 0 cube (root only, no subdivision)
- [ ] 2.8.2 Test: Max depth limit prevents further traversal
- [ ] 2.8.3 Test: Depth parameter correctly limits recursion

### 2.9 Robustness Tests
- [ ] 2.9.1 Test: Division by zero protection (ray_dir component = 0)
- [ ] 2.9.2 Test: Very small epsilon values don't cause NaN
- [ ] 2.9.3 Test: Position clamping keeps values in [0,1]³
- [ ] 2.9.4 Test: Floating-point precision accumulation
- [ ] 2.9.5 Test: No infinite loops (iteration limit)

## 3. Implementation
Now implement the code to pass all tests written above

- [ ] 3.1 Implement helper: `calculate_octant_index(pos: Vec3, dir: Vec3) -> usize`
- [ ] 3.2 Implement helper: `calculate_entry_normal(pos: Vec3) -> Vec3`
- [ ] 3.3 Implement helper: `next_integer_boundary(v: Vec3, sign: Vec3) -> Vec3`
- [ ] 3.4 Implement helper: `calculate_next_position(pos2: Vec3, dir: Vec3, sign: Vec3) -> Vec3`
- [ ] 3.5 Implement helper: `transform_to_child_space(pos: Vec3, octant_bit: IVec3) -> Vec3`
- [ ] 3.6 Implement main: `Cube::raycast(pos: Vec3, dir: Vec3, depth: u32) -> Option<RaycastHit>`
- [ ] 3.7 Implement recursive traversal logic
- [ ] 3.8 Add early termination on first solid hit
- [ ] 3.9 Add DDA stepping for empty octant skipping
- [ ] 3.10 Add all edge case protections (epsilon checks, clamping, iteration limits)

## 4. Verify All Tests Pass
- [ ] 4.1 Run all new tests: `cargo test -p cube raycast`
- [ ] 4.2 Verify all 33 existing tests still pass
- [ ] 4.3 Confirm test count increased from 33 to 50+
- [ ] 4.4 Check test coverage for each scenario in design doc

## 5. Code Quality
- [ ] 5.1 Run `cargo fmt` on cube crate
- [ ] 5.2 Run `cargo clippy -p cube` and fix all warnings
- [ ] 5.3 Add inline documentation for key functions
- [ ] 5.4 Add module-level documentation explaining algorithm
- [ ] 5.5 Document coordinate system conventions
- [ ] 5.6 Add code examples in doc comments

## 6. Performance Validation (Optional)
- [ ] 6.1 Add benchmark for simple raycast (depth 1)
- [ ] 6.2 Add benchmark for deep octree (depth 5)
- [ ] 6.3 Profile with `cargo flamegraph` if performance issues found
- [ ] 6.4 Document performance characteristics

## 7. Documentation
- [ ] 7.1 Update algorithm explanation in code comments
- [ ] 7.2 Add examples of usage
- [ ] 7.3 Document all public functions
- [ ] 7.4 Cross-reference `docs/raycast.md` in code
- [ ] 7.5 Update `crates/cube/README.md` if it exists

## 8. Final Validation
- [ ] 8.1 All tests pass: `cargo test -p cube`
- [ ] 8.2 No clippy warnings: `cargo clippy -p cube -- -D warnings`
- [ ] 8.3 Code formatted: `cargo fmt --check -p cube`
- [ ] 8.4 WASM build succeeds: `cd crates/cube && wasm-pack build --dev --target web`
- [ ] 8.5 Review code against design document - all requirements covered

## Success Criteria
- ✅ All 33 existing tests continue to pass
- ✅ 20+ new tests added (total 50+ tests)
- ✅ Each scenario in `docs/raycast.md` has a corresponding test
- ✅ No compiler warnings or clippy issues
- ✅ Code is well-documented with examples
- ✅ WASM compilation succeeds

## Dependencies
- Existing `Cube<T>` octree structure
- Existing `glam` library (Vec3, IVec3)
- Existing test infrastructure in cube crate

## Future Work (Not in This Change)
- Renderer integration (`crates/renderer/`)
- GPU implementation
- Bounding box integration
- Lighting calculations
