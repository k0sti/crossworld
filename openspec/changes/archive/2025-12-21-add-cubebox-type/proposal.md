# Change: Add CubeBox Type for Bounded Voxel Models

## Why

The current vox loading system uses an `align` parameter (Vec3 0.0-1.0) to position models within their octree bounds. This approach has several problems:

1. **Lost bounds information**: After loading, the original model dimensions are discarded - only the power-of-2 cube size is known
2. **Inconsistent alignment**: Different call sites use different alignments (0.5/0.5/0.5 for centered, 0.5/0.0/0.5 for ground-aligned) making it error-prone
3. **Redundant depth calculation**: Multiple places (`models.rs`, `structures.rs`, `voxLoader.ts`) recalculate cube depth after loading
4. **No size information for physics**: Collider generation needs actual model bounds, not just the enclosing octree size
5. **Scale confusion**: The `scale_exp` and `depth` fields in `VoxModel`/`StructureModel` are used inconsistently to represent model scale

## What Changes

### New Type: `CubeBox`
A wrapper that pairs a `Cube<T>` with its actual bounds:
- `cube: Cube<T>` - The octree data
- `size: IVec3` - Original model dimensions in voxels (e.g., 16x30x12 for an avatar)
- `depth: u32` - Octree depth (determines coordinate scale: size is in units of 2^depth)

### API Changes
- **ADDED**: `CubeBox<T>` struct in `cube` crate
- **ADDED**: `load_vox_to_cubebox()` function that returns `CubeBox<u8>`
- **ADDED**: `CubeBox::place_in()` method to combine a box model into a larger cube at a position
- **ADDED**: `WasmCubeBox` WASM wrapper with equivalent functionality
- **MODIFIED**: Deprecate `load_vox_to_cube()` with align parameter (keep for backward compatibility)
- **MODIFIED**: Update `VoxModel` and `StructureModel` to use `CubeBox` internally

### Benefits
- Clear separation between octree structure and model bounds
- Bounds information preserved for physics, rendering, and placement
- Simpler API: no alignment parameter needed for loading
- Consistent placement logic via `place_in()` method

## Impact

- **Affected specs**: None existing (new capability)
- **Affected code**:
  - `crates/cube/src/io/vox/loader.rs` - Add new loader function
  - `crates/cube/src/lib.rs` - Export new types
  - `crates/cube/src/wasm/cube_wasm.rs` - Add WASM bindings
  - `crates/proto-gl/src/models.rs` - Use CubeBox for VoxModel
  - `crates/proto-gl/src/structures.rs` - Use CubeBox for StructureModel
  - `crates/world/src/avatar/manager.rs` - Use CubeBox for avatar loading
  - `packages/app/src/utils/voxLoader.ts` - Use new WASM API
