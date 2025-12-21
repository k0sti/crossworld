## 1. Core CubeBox Implementation

- [x] 1.1 Create `CubeBox<T>` struct in `crates/cube/src/core/cubebox.rs`
- [x] 1.2 Implement `CubeBox::new()`, `octree_size()`, `bounds()`, `fits_octree()` methods
- [x] 1.3 Implement `CubeBox::place_in()` method using `update_depth_tree`
- [x] 1.4 Add unit tests for CubeBox creation and methods
- [x] 1.5 Export CubeBox from `crates/cube/src/lib.rs`

## 2. Vox Loading Updates

- [x] 2.1 Add `load_vox_to_cubebox()` function in `crates/cube/src/io/vox/loader.rs`
- [x] 2.2 Mark `load_vox_to_cube()` as deprecated
- [x] 2.3 Add tests for `load_vox_to_cubebox()` with various model sizes
- [x] 2.4 Export new function from `crates/cube/src/io/mod.rs` and `crates/cube/src/lib.rs`

## 3. WASM Bindings

- [x] 3.1 Create `WasmCubeBox` struct in `crates/cube/src/wasm/cube_wasm.rs`
- [x] 3.2 Implement `sizeX`, `sizeY`, `sizeZ`, `depth` getters
- [x] 3.3 Implement `cube()` method to get inner WasmCube
- [x] 3.4 Implement `placeIn()` method
- [x] 3.5 Add `loadVoxBox()` function
- [x] 3.6 Add TypeScript type definitions to `packages/app/src/types/cube-wasm.d.ts`

## 4. Proto-GL Integration

- [x] 4.1 Update `VoxModel` in `crates/proto-gl/src/models.rs` to use `CubeBox`
- [x] 4.2 Update `load_vox_models()` to use `load_vox_to_cubebox()`
- [x] 4.3 Update `StructureModel` in `crates/proto-gl/src/structures.rs` to use `CubeBox`
- [x] 4.4 Update `load_structure_models()` to use `load_vox_to_cubebox()`
- [x] 4.5 Update `place_structures()` to use CubeBox methods
- [x] 4.6 Remove redundant `calculate_cube_depth()` functions

## 5. World/Avatar Integration

- [x] 5.1 Update `AvatarManager` in `crates/world/src/avatar/manager.rs` to use `CubeBox`
- [x] 5.2 Add `get_avatar_bounds()` method using CubeBox size

## 6. TypeScript Updates

- [x] 6.1 Update `loadVoxFromUrl()` in `packages/app/src/utils/voxLoader.ts` to use new API
- [x] 6.2 Update `loadVoxFromFile()` to use new API
- [x] 6.3 Add helper for getting model bounds from WasmCubeBox
- [x] 6.4 Update any other TypeScript code using loadVox

## 7. Validation

- [x] 7.1 Run `cargo test --workspace` - CubeBox tests pass (pre-existing test failures unrelated)
- [x] 7.2 Run `cargo clippy -p cube` - no warnings in cube crate
- [ ] 7.3 Run `just build-wasm-dev` - WASM builds successfully
- [ ] 7.4 Run `just dev` - app runs and vox models load correctly
- [ ] 7.5 Verify avatars load and display correctly
- [ ] 7.6 Verify structures place correctly in proto-gl
