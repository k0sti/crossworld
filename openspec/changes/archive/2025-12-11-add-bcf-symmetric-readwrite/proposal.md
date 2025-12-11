# Change: Add Symmetric BCF Read/Write with Round-Trip Testing

## Why

The GL renderer is broken because it uses the wrong approach to serialize octree data. The current BCF (Binary Cube Format) has write capability but lacks comprehensive read/write symmetry testing and validation. Before we can fix the GL renderer, we need:

1. **Verified BCF round-trip**: Ensure serialize → deserialize → serialize produces identical binary
2. **Complete test coverage**: Test all Cube variants (Solid, Cubes/octree) with edge cases
3. **Documented invariants**: Clear specification of what equality means for BCF binary data

**Current BCF status** (from `add-binary-cube-format` change):
- ✅ Phases 1-4 complete: Solid and Cubes (octree) variants work
- ✅ Basic tests exist (9 tests passing)
- ⚠️ No comprehensive round-trip validation for complex trees
- ⚠️ Planes and Slices serialize as empty leaves (limitation documented)
- ⚠️ No testing of maximal-depth trees or stress cases

**Why this is blocking GL renderer fix**:
- GL renderer needs to upload BCF binary to GPU
- GPU shader must parse BCF to traverse octree
- Cannot validate GPU code without knowing CPU parsing is 100% correct
- Need reference implementation that works on both CPU and can be translated to GLSL

**The real problem**:
Current GL renderer tries to sample octree as voxel grid using wrong coordinates. It calls `cube.get_id(depth, pos)` with [0,8) grid coordinates, but octree uses [-1,1]³ center-based coordinates. This is fundamentally incompatible.

**The solution path**:
1. First: Ensure BCF format is rock-solid with symmetric read/write (this change)
2. Then: Create CPU-based BCF traversal raytracer that mirrors what GPU will do
3. Finally: Translate CPU BCF traversal to GLSL fragment shader

This proposal handles step 1, ensuring BCF is production-ready before building traversal code.

## What Changes

### Phase 1: Round-Trip Validation Tests
- **`crates/cube/tests/bcf_roundtrip_tests.rs`** (new file)
  - Test: `serialize(cube) -> bytes1, deserialize(bytes1) -> cube2, serialize(cube2) -> bytes2, assert_eq!(bytes1, bytes2)`
  - Test all Cube variants: Solid (inline/extended), Cubes (leaves/pointers)
  - Test edge cases: empty cube (0), max value (255), all-same octree, checkerboard pattern
  - Test depths: depth 1 (8 leaves), depth 2 (64 voxels), depth 3 (512 voxels)
  - Test pointer sizes: 1-byte, 2-byte, 4-byte pointer octas
  - Verify binary size matches expected (no padding issues)

### Phase 2: Structural Equality Tests
- **`crates/cube/tests/bcf_equality_tests.rs`** (new file)
  - Test: `cube1 == cube2` implies `serialize(cube1) == serialize(cube2)` (deterministic)
  - Test: Different logical cubes produce different binary (no collisions)
  - Test: Optimization correctness (octa-with-leaves vs octa-with-pointers for same data)
  - Test: Header validation (magic, version, root offset)

### Phase 3: Performance Benchmarks (optional, informational only)
- **`crates/cube/benches/bcf_bench.rs`** (new file, if benches/ exists)
  - Benchmark serialize speed for various tree depths
  - Benchmark deserialize speed
  - Compare BCF vs CSM file sizes and parse times
  - Generate report for documentation

### Phase 4: API Documentation Improvements
- **`crates/cube/src/io/bcf/mod.rs`**
  - Document round-trip guarantees: `deserialize(serialize(cube))` always succeeds
  - Document determinism: Multiple calls to `serialize(cube)` produce identical bytes
  - Document current limitations: Planes/Slices serialize as Solid(0)
  - Add examples showing correct usage patterns

### Phase 5: Error Handling Validation
- **`crates/cube/tests/bcf_error_tests.rs`** (new file)
  - Test all BcfError variants: InvalidMagic, UnsupportedVersion, TruncatedData, InvalidOffset, RecursionLimit
  - Test malformed data: corrupt headers, invalid pointers, circular references
  - Test edge cases: zero-length buffer, root offset beyond EOF, excessive recursion depth
  - Verify error messages are helpful for debugging

### Not Changed
- BCF format specification (already defined in `add-binary-cube-format`)
- Serializer implementation (already works for Solid/Cubes)
- Deserializer implementation (already works for Solid/Cubes)
- Module structure (`crates/cube/src/io/bcf/`)
- Planes/Slices support (remains unimplemented, falls back to Solid(0))

## Impact

### Affected Specs
- **MODIFIED**: `binary-cube-format` - Add round-trip testing requirements and equality guarantees

### Affected Code
- `crates/cube/tests/bcf_roundtrip_tests.rs` - NEW: Round-trip validation
- `crates/cube/tests/bcf_equality_tests.rs` - NEW: Structural equality tests
- `crates/cube/tests/bcf_error_tests.rs` - NEW: Error handling validation
- `crates/cube/benches/bcf_bench.rs` - NEW (optional): Performance benchmarks
- `crates/cube/src/io/bcf/mod.rs` - Enhanced documentation
- No changes to serializer/parser implementation (already correct)

### Benefits
- **Confidence in BCF format**: 100% verified correctness before GPU usage
- **Regression protection**: Catch serialization bugs early with comprehensive tests
- **Clear semantics**: Documented guarantees about round-trip and determinism
- **Debugging aid**: Better error messages and validation tests
- **Foundation for GL renderer**: Can now build GPU traversal on solid base

### Compatibility
- No breaking changes (additive tests only)
- Existing BCF serialization/deserialization unchanged
- Tests run in CI with `cargo test --workspace`

### Success Criteria
- All round-trip tests pass: `serialize → deserialize → serialize` produces identical bytes
- All edge case tests pass: empty, max value, deep trees, pointer sizes
- All error handling tests pass: invalid data rejected with clear errors
- Documentation clearly states guarantees and limitations
- Code passes `cargo test --workspace`
- Code passes `cargo clippy --workspace -- -D warnings`

### Dependencies
- **BUILDS ON**: `add-binary-cube-format` phases 1-4 (already complete)
- **BLOCKS**: GL renderer BCF integration (cannot trust GPU code without CPU verification)
- **BLOCKS**: CPU-based BCF traversal raytracer (needs verified format first)

### Breaking Changes
None - this is additive testing and documentation only

## Implementation Status

### Current State
- ❌ No comprehensive round-trip tests (only basic 9 tests exist)
- ❌ No structural equality validation
- ❌ No stress tests for deep trees or edge cases
- ⚠️ Basic error tests exist but incomplete
- ✅ Serializer and parser implementations complete for Solid/Cubes

### Timeline Estimate
- Phase 1 (Round-trip tests): 1-2 hours
- Phase 2 (Equality tests): 1 hour
- Phase 3 (Benchmarks): 1 hour (optional)
- Phase 4 (Documentation): 30 minutes
- Phase 5 (Error tests): 1 hour
- **Total**: 4-5 hours (3.5 hours without benchmarks)

### Risk Assessment
**Low Risk:**
- No changes to production code (tests only)
- Tests run in isolation
- Easy to rollback if tests fail

**Potential Issues:**
- Tests might expose existing bugs in serializer/parser
- May discover that current BCF format has ambiguities
- Could reveal performance issues requiring optimization

**Mitigation:**
- Run tests incrementally, fix bugs as discovered
- Update specification if ambiguities found
- Performance issues addressed in future optimization pass
