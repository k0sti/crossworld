# Implementation Tasks - Centralize Material System

## 1. Cube Crate Implementation
- [x] 1.1 Create `crates/cube/src/material.rs` module
- [x] 1.2 Define `Material` struct and `MATERIAL_REGISTRY` constant
- [x] 1.3 Implement `get_material_color(index)` with R2G3B2 support
- [x] 1.4 Port 0-127 material definitions from `assets/materials.json` to Rust code
- [x] 1.5 Expose `material` module in `crates/cube/src/lib.rs`
- [x] 1.6 Add unit tests for material lookup and R2G3B2 decoding

## 2. Renderer Crate Updates
- [x] 2.1 Update `cpu_tracer.rs` to use `cube::material::get_material_color`
- [x] 2.2 Update `gl_tracer.rs` to upload material palette to GPU
- [x] 2.3 Update `octree_raycast.frag` to use uniform palette instead of hardcoded array
- [x] 2.4 Remove `crates/renderer/src/materials.rs`
- [x] 2.5 Fix any compilation errors in renderer tests

## 3. World Crate Updates
- [x] 3.1 Update `crates/world/src/lib.rs` to remove local material handling if any
- [x] 3.2 Ensure world generation uses correct material indices

## 4. Verification
- [x] 4.1 Run `cargo test -p cube`
- [x] 4.2 Run `cargo test -p renderer`
- [x] 4.3 Verify `color_verification` test passes
