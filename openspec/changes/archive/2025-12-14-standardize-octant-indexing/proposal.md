# Standardize Octant Indexing to 0/1 Coordinates

## Problem

Currently, the `cube` crate uses +1/-1 components in `IVec3` for octant indexing. This approach requires extra arithmetic to convert between octant coordinates and octant indices:

- Octant coordinates are represented as `IVec3` with components of -1 or +1
- Converting to octant index requires: `(x > 0) as usize * 4 + (y > 0) as usize * 2 + (z > 0) as usize`
- The `OCTANT_POSITIONS` lookup table stores vectors with -1/+1 components

This design adds unnecessary complexity and makes the code less intuitive. Standard octree implementations use 0/1 coordinates that map directly to bit positions in the octant index.

## Solution

Standardize all octant coordinate representations to use 0/1 components instead of -1/+1:

- Update `OCTANT_POSITIONS` constant to use 0/1 coordinates
- Simplify `to_octant_index()` to direct bit manipulation: `(x << 2) | (y << 1) | z`
- Update `from_octant_index()` to use bit extraction: `(index >> 2) & 1`, `(index >> 1) & 1`, `index & 1`
- Update `step0()` function to return 0/1 instead of -1/+1
- Fix all usages throughout the cube crate and dependent crates

## Benefits

1. **Simpler arithmetic**: Direct bit manipulation instead of comparison-based conversion
2. **More intuitive**: 0/1 coordinates match the binary structure of octant indices
3. **Consistent with standard octree literature**: Most octree implementations use 0/1
4. **Better performance**: Eliminates comparisons and conditional logic
5. **Clearer code intent**: Octant coordinates directly represent bit positions

## Impact

### Files to Update (cube crate)

- `crates/cube/src/core/cube.rs`: Core octant functionality
- `crates/cube/src/core/raycast.rs`: Ray-octree intersection
- `crates/cube/src/traversal/mod.rs`: Octree traversal algorithms
- `crates/cube/src/traversal/neighbor_grid.rs`: Neighbor grid calculation
- `crates/cube/tests/*.rs`: Test assertions

### Breaking Changes

This is an internal refactoring that should not affect external APIs. The change is purely internal to how octant coordinates are represented and manipulated within the cube crate.

### Testing

All existing tests (33 raycast tests plus unit tests) must pass after the change. The behavior of the octree system remains identical; only the internal representation changes.

## Implementation Strategy

1. Update core constants and trait methods in `cube.rs`
2. Fix raycast octant calculation in `raycast.rs`
3. Update traversal algorithms in `traversal/mod.rs`
4. Fix neighbor grid indexing in `neighbor_grid.rs`
5. Update all test assertions to use 0/1 coordinates
6. Run comprehensive test suite to verify correctness
7. Build WASM and verify no breaking changes to external API

## Relationship to Other Changes

This change is independent of other active OpenSpec changes. It's a focused refactoring that simplifies the internal octant indexing system without affecting higher-level functionality.

## Validation Criteria

- [ ] All Rust tests pass: `cargo test -p cube`
- [ ] No clippy warnings: `cargo clippy -p cube`
- [ ] WASM builds successfully: `just build-wasm-dev`
- [ ] External API unchanged (no TypeScript changes needed)
- [ ] Performance neutral or improved (micro-benchmark if needed)
