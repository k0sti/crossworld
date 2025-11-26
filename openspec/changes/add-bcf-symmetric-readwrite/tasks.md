# Implementation Tasks

## 1. Round-Trip Validation Tests
- [x] 1.1 Create `crates/cube/tests/bcf_roundtrip_tests.rs`
- [x] 1.2 Implement test helper: `fn assert_roundtrip(cube: &Cube<u8>)` that checks `serialize → deserialize → serialize` produces identical bytes
- [x] 1.3 Test solid cubes: inline (0-127) and extended (128-255)
- [x] 1.4 Test depth-1 octree: 8 solid leaves (octa-leaves encoding)
- [x] 1.5 Test depth-2 octree: mix of leaves and pointers
- [x] 1.6 Test depth-3 octree: 512 voxels with various patterns
- [x] 1.7 Test edge cases: empty cube (all zeros), all-255, checkerboard pattern
- [x] 1.8 Test pointer sizes: force 1-byte, 2-byte, 4-byte pointer scenarios
- [x] 1.9 Verify binary sizes match expected (header + nodes)
- [x] 1.10 Run tests: `cargo test --test bcf_roundtrip_tests`

## 2. Structural Equality Tests
- [x] 2.1 Create `crates/cube/tests/bcf_equality_tests.rs`
- [x] 2.2 Test determinism: multiple `serialize(cube)` calls produce identical bytes
- [x] 2.3 Test: `cube1 == cube2` implies `serialize(cube1) == serialize(cube2)`
- [x] 2.4 Test: different cubes produce different binaries (no collisions)
- [x] 2.5 Test optimization correctness: octa-leaves vs octa-pointers for equivalent data
- [x] 2.6 Test header consistency: magic, version, root offset always valid
- [x] 2.7 Run tests: `cargo test --test bcf_equality_tests`

## 3. Error Handling Validation
- [x] 3.1 Create `crates/cube/tests/bcf_error_tests.rs`
- [x] 3.2 Test InvalidMagic: corrupt magic number detection
- [x] 3.3 Test UnsupportedVersion: version mismatch detection
- [x] 3.4 Test TruncatedData: incomplete binary data
- [x] 3.5 Test InvalidOffset: root offset beyond EOF, child pointers out of bounds
- [x] 3.6 Test RecursionLimit: excessively deep trees
- [x] 3.7 Test zero-length buffer rejection
- [x] 3.8 Test partial header (less than 12 bytes)
- [x] 3.9 Verify error messages are descriptive
- [x] 3.10 Run tests: `cargo test --test bcf_error_tests`

## 4. API Documentation Improvements
- [x] 4.1 Open `crates/cube/src/io/bcf/mod.rs`
- [x] 4.2 Add module-level doc comment with overview
- [x] 4.3 Document round-trip guarantee: `deserialize(serialize(cube))` preserves structure
- [x] 4.4 Document determinism: `serialize(cube)` is deterministic (same input → same bytes)
- [x] 4.5 Document current limitations: Planes/Slices serialize as `Solid(0)` with warning
- [x] 4.6 Add usage examples: basic serialize/deserialize, error handling
- [x] 4.7 Document performance characteristics (O(n) where n = node count)
- [x] 4.8 Run doc tests: `cargo test --doc`

## 5. Performance Benchmarks (Optional)
- [ ] 5.1 Check if `crates/cube/benches/` directory exists (create if needed)
- [ ] 5.2 Create `crates/cube/benches/bcf_bench.rs`
- [ ] 5.3 Benchmark serialize for depth 1, 2, 3, 4 trees
- [ ] 5.4 Benchmark deserialize for same tree sizes
- [ ] 5.5 Compare BCF vs CSM file sizes
- [ ] 5.6 Compare BCF vs CSM parse times
- [ ] 5.7 Run benchmarks: `cargo bench --bench bcf_bench`
- [ ] 5.8 Document results in `crates/cube/docs/bcf-performance.md`

## 6. Integration and Validation
- [x] 6.1 Run full test suite: `cargo test --workspace`
- [x] 6.2 Run clippy: `cargo clippy --workspace -- -D warnings`
- [x] 6.3 Run formatter check: `cargo fmt --check`
- [x] 6.4 Verify all new tests pass (at least 20+ new tests) - 49 tests passing
- [x] 6.5 Check test coverage: all BCF code paths exercised
- [x] 6.6 Review documentation: clear, accurate, with examples
- [ ] 6.7 Commit changes with message: "test: Add comprehensive BCF round-trip and validation tests"
