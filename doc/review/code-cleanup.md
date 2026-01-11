# Code Clean-up Review

## Summary

Comprehensive analysis of the Crossworld codebase identified significant duplicate code, unused features, and architectural redundancy across the `app`, `game`, `editor`, and Bevy-based crates.

## Major Findings

### 1. Duplicate Crates: Bevy vs. Native OpenGL

**Issue**: The codebase maintains **two parallel implementations** of the same functionality:
- **Bevy-based crates**: `app-bevy`, `editor-bevy`, `proto-bevy` (~2,018 lines)
- **Native OpenGL crates**: `app`, `game`, `editor` (actively maintained)

**Evidence**:
- Git history shows Bevy crates haven't been actively developed
- Last significant commit: "Simplify Camera to quaternion-only, remove unused editor files"
- Both systems provide: windowing, rendering, camera controls, input handling
- The native OpenGL stack (glow/egui/winit) is the actively used system

**Recommendation**: **Remove all Bevy crates**
- `crates/app-bevy/` - Duplicate of `crates/app/`
- `crates/editor-bevy/` - Duplicate of `crates/editor/`
- `crates/proto-bevy/` - Prototype, no longer needed
- `crates/proto-gl/` - Also appears to be a prototype

**Impact**: Removes ~2,500+ lines of unmaintained code, simplifies dependency tree, reduces confusion.

---

### 2. Duplicate Lua Configuration Code

**Issue**: Two separate `lua_config.rs` files with overlapping functionality:

**Location 1**: `crates/app/src/lua_config.rs` (371 lines)
- Base Lua configuration with `vec3`, `quat_euler` helpers
- Generic utilities: `parse_vec3`, `extract_u32`, `extract_f32`
- Designed to be reusable across crates

**Location 2**: `crates/editor/src/lua_config.rs` (389 lines)
- Editor-specific test configuration (mouse events, frame captures)
- **Re-imports utilities from `app::lua_config`** but duplicates logic elsewhere

**Recommendation**: **Consolidate Lua configuration**
- Keep base utilities in `crates/app/src/lua_config.rs`
- Move editor-specific types (`EditorTestConfig`, `MouseEvent`) into a separate module
- Consider moving test configuration to `crates/editor/src/test_config.rs` for clarity

**Impact**: Reduces confusion, makes Lua integration more maintainable.

---

### 3. `game` Crate: Thin Wrapper with Limited Purpose

**Issue**: The `game` crate is a minimal wrapper around the `app` framework:

**What it does**:
- Implements `App` trait for `VoxelGame`
- Loads world from Lua config (`config/world.lua`)
- Provides FPS camera controls
- Renders a procedurally generated voxel world

**Overlap with `editor`**:
- Both use `app` framework
- Both use `MeshRenderer`, `SkyboxRenderer`
- Both implement camera controls (FPS vs. Orbit)
- Both load/render voxel cubes
- `game` uses hot-reload system that `editor` doesn't need

**Key difference**: `game` has hot-reload support via `libloading` and `notify` crates.

**Recommendation**: **Evaluate if `game` crate is necessary**
- If hot-reload is essential, keep `game` as a development/testing tool
- If not actively used, consider merging into `testbed` or removing
- Document the purpose clearly: Is this a game runtime or a dev tool?

**Impact**: Clarifies architecture, reduces maintenance burden.

---

### 4. Redundant Dependencies in `game` Crate

**Issue**: `game/Cargo.toml` includes dependencies only used by the binary, not the library:

```toml
[dependencies]
# These are only used by src/main.rs (hot-reload runtime):
glutin = "0.32"
glutin-winit = "0.5"
raw-window-handle = "0.6"
libloading = "0.9"
notify = "8.2"
notify-debouncer-mini = "0.7"

# But lib.rs doesn't use them!
```

**Current structure**:
- `src/lib.rs`: Implements `VoxelGame` (uses `app`, `renderer`, `cube`, `world`)
- `src/main.rs`: Hot-reload runner (uses `libloading`, `notify`)

**Recommendation**: **Split dependencies properly**
- Move hot-reload deps to `[dev-dependencies]` or a separate `[[bin]]` section
- Or remove library target entirely if not used elsewhere

**Impact**: Faster compilation, clearer dependency boundaries.

---

### 5. Unused `gilrs` Gamepad Feature

**Issue**: `app` crate has optional gamepad support via `gilrs`:

```toml
[features]
default = ["gilrs"]
gilrs = ["dep:gilrs"]
```

**Usage analysis**:
- ✅ `game` crate uses gamepad input (controller stick movement)
- ❌ `editor` crate doesn't use gamepad at all
- `app` provides `GilrsBackend` abstraction

**Recommendation**: **Keep feature, but make it non-default**
- Change `default = []` (no gamepad by default)
- Enable in `game` explicitly: `app = { path = "../app", features = ["runtime", "gilrs"] }`
- Removes unnecessary dependency from `editor` builds

**Impact**: Smaller binary size for editor, faster compilation.

---

### 6. Common Code Patterns Across `game` and `editor`

**Similarities**:
- Both implement `App` trait
- Both use `Camera`, `MeshRenderer`, `SkyboxRenderer`
- Both load/render cube models
- Both have similar initialization patterns

**Key file structure overlap**:
```
game/src/
  ├── lib.rs        (VoxelGame implements App)
  ├── config.rs     (Lua world config)
  └── main.rs       (hot-reload runner)

editor/src/
  ├── lib.rs        (EditorApp implements App)
  ├── config.rs     (editor settings)
  ├── lua_config.rs (test config)
  ├── ui.rs         (egui panels)
  ├── editing.rs    (voxel placement/removal)
  ├── raycast.rs    (mouse picking)
  ├── cursor.rs     (3D cursor)
  └── palette.rs    (color/material/model palettes)
```

**Recommendation**: **Extract shared utilities to `app` or new `app-utils` crate**
- Common camera setup patterns
- Mesh upload/render boilerplate
- GL initialization helpers

**Impact**: Reduces duplication, easier to maintain.

---

### 7. Unused Prototype Crate

**Issue**: `crates/proto-gl/` appears in git history but unclear if still needed.

**Recommendation**: **Remove if superseded by `game`/`editor`**

**Impact**: Cleaner repository.

---

## Architectural Recommendations

### Proposed Crate Structure

**Core libraries** (keep):
- `core` - Shared types (input, controller)
- `cube` - Voxel octree data structure
- `world` - Procedural terrain generation
- `physics` - Rapier3D wrapper
- `renderer` - OpenGL rendering (mesh, skybox, wireframe)
- `assets` - Asset management

**Application framework** (keep, refactor):
- `app` - Base application framework (App trait, camera, input, egui integration)
  - Remove `gilrs` from default features
  - Keep `runtime` feature for egui support

**Applications** (consolidate):
- `editor` - Voxel editor (keep as primary app)
- `game` - **Decision needed**: Keep for hot-reload testing or merge into `testbed`?
- `testbed` - Testing/prototyping (consolidate examples here)

**Remove**:
- `app-bevy` - Unmaintained Bevy duplicate
- `editor-bevy` - Unmaintained Bevy duplicate
- `proto-bevy` - Prototype, no longer needed
- `proto-gl` - Prototype, superseded

**Backend/tools** (keep):
- `server` - Game server (WebTransport)
- `worldtool` - Nostr event management CLI
- `test-client` - Testing client

---

## Detailed Refactoring Plan

### Phase 1: Remove Dead Code (Low Risk)
1. **Delete Bevy crates**
   - Remove `crates/app-bevy/`
   - Remove `crates/editor-bevy/`
   - Remove `crates/proto-bevy/`
   - Remove `crates/proto-gl/` (if confirmed unused)
   - Update `Cargo.toml` workspace members

2. **Clean up unused features**
   - Make `gilrs` feature non-default in `app/Cargo.toml`
   - Add explicit `features = ["gilrs"]` in `game/Cargo.toml`

### Phase 2: Consolidate Configuration (Medium Risk)
3. **Refactor Lua configuration**
   - Keep base Lua utils in `app/src/lua_config.rs`
   - Move editor test config to `editor/src/test_config.rs`
   - Update imports in `editor/src/lib.rs`

### Phase 3: Clarify `game` Crate Purpose (Needs Decision)
4. **Decide on `game` crate fate**
   - Option A: Keep for hot-reload development workflow
   - Option B: Merge into `testbed` as example
   - Option C: Remove entirely if not used

5. **If keeping `game`**: Fix dependencies
   - Move hot-reload deps to correct section
   - Document its purpose in README

### Phase 4: Extract Common Utilities (Optional, Lower Priority)
6. **Create shared helpers**
   - Extract camera setup patterns
   - Extract GL boilerplate
   - Consider `app-utils` crate if `app` gets too large

---

## Metrics

**Before cleanup**:
- Total crates: 19
- Bevy-related LOC: ~2,018 lines
- Duplicate Lua config: 2 files, ~760 lines total
- Unused features: `gilrs` compiled even when not needed

**After cleanup** (estimated):
- Total crates: 14-15 (depending on `game` decision)
- Removed dead code: ~2,500+ lines
- Consolidated config: 1 base + 1 editor-specific module
- Clearer dependency boundaries

---

## Testing Plan

After each phase:
1. Run `cargo check --workspace` (verify compilation)
2. Run `cargo clippy --workspace` (verify lints)
3. Run `cargo test --workspace` (verify tests pass)
4. Build editor: `cargo build --release --bin editor`
5. Build game (if keeping): `cargo build --release --bin game`
6. Test editor functionality manually
7. Run integration tests in `testbed`

---

## Questions for Review

1. **Is the `game` crate actively used?** If not, should it be removed or merged?
2. **Are the Bevy crates needed for any reason?** (compatibility, future plans?)
3. **Should hot-reload stay in `game` or move to `testbed`?**
4. **Are there any dependencies on `proto-gl` or `proto-bevy`?**
5. **Should we extract more common code to reduce duplication between apps?**

---

## Priority

**High Priority** (clear wins, low risk):
- Remove Bevy crates (unmaintained duplicates)
- Make `gilrs` feature non-default

**Medium Priority** (cleanup, moderate effort):
- Consolidate Lua configuration
- Decide on `game` crate purpose

**Low Priority** (nice to have, more invasive):
- Extract shared utilities
- Further architectural refactoring
