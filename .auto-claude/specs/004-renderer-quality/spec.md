# Specification: Improve Renderer Quality and Architecture

## Overview

Fix rendering quality issues in the renderer crate's mesh renderer, improve code architecture by renaming components for clarity, add mesh caching options, and update documentation.

## Source

Migrated from: `openspec/changes/improve-renderer-quality/`

## Current Status

**Completion: 67% (18/27 tasks complete)**

### Completed Work
- Phase 1: Investigation and Diagnosis (3/3) - all issues identified
- Phase 2: Fix Camera Synchronization (1/2) - camera fixed, validation test deferred
- Phase 3: Fix Face Rendering (1/2) - culling fixed, lighting validation deferred
- Phase 4: Implement Mesh Caching (4/4) - complete with UI controls
- Phase 7: Validation and Testing (3/3) - manual testing done

### Pending Work
- Camera sync validation test (deferred)
- Lighting consistency validation (deferred)
- Phase 5: Rename DualRenderer to CubeRenderer (3 tasks)
- Phase 6: Update Documentation (4 tasks)

## Problem Statement

The renderer crate currently has several quality and usability issues:

1. **Mesh renderer camera calculation is incorrect** - The mesh renderer doesn't properly synchronize camera configuration with other renderers (CPU, GL, GPU), resulting in different viewpoints and making comparison difficult.

2. **Mesh face rendering has visual artifacts** - Front/back face culling may be inverted or incorrectly configured, causing rendering issues.

3. **Inefficient mesh regeneration** - The mesh is potentially regenerated every frame even when the voxel data hasn't changed, wasting GPU upload bandwidth and CPU cycles.

4. **Unclear naming** - "DualRenderer" is now a misnomer as the system has expanded to five renderers (CPU, GL, BCF CPU, GPU, Mesh). The name should reflect its role as a comprehensive cube rendering comparison tool.

5. **Outdated documentation** - README and documentation still refer to "dual" renderer and don't document the mesh renderer or recent additions.

## Solutions Implemented

### Camera Synchronization (Fixed)
- **Root cause**: View matrix was constructed incorrectly
- **Fix**: Changed to use `look_to_rh(camera.position, forward, up)` where forward/up are derived from camera rotation quaternion
- **Result**: Mesh renderer now displays same viewpoint as other renderers

### Face Culling (Fixed)
- **Root cause**: Face culling was not enabled
- **Fix**: Added `gl.enable(CULL_FACE)`, `gl.cull_face(BACK)`, `gl.front_face(CCW)` before rendering, with cleanup `gl.disable(CULL_FACE)` after
- **Result**: Faces render correctly without visual artifacts

### Mesh Caching (Implemented)
- **State tracking**: Added `mesh_needs_regeneration: bool`, `mesh_cache_enabled: bool`, and mesh statistics fields
- **Cache invalidation**: Set `mesh_needs_regeneration = true` on model/material change
- **UI controls**: Added "Cache Mesh" checkbox (default: enabled), "Regen Mesh" button, cache status indicator
- **Result**: Mesh upload time displayed in UI; caching prevents re-upload each frame

## Pending Solutions

### DualRenderer Rename (Phase 5)
- Rename `DualRendererApp` -> `CubeRendererApp` in `egui_app.rs`
- Rename `run_dual_renderer` -> `run_cube_renderer` functions
- Update variable names and comments throughout

### Documentation Update (Phase 6)
- Update README.md with multi-implementation title, mesh renderer features
- Update inline documentation with mesh caching behavior
- Create TEST_SUMMARY.md with test results
- Update CLI help text

## Affected Files

### Primary
- `crates/renderer/src/mesh_renderer.rs` - Camera fix, face culling fix
- `crates/renderer/src/egui_app.rs` - Mesh caching, rename
- `crates/renderer/src/main.rs` - Function renames, CLI help
- `crates/renderer/README.md` - Documentation update

### Reference
- `crates/renderer/src/cpu_renderer.rs` - Camera reference
- `crates/renderer/src/gl_renderer.rs` - Camera reference
- `crates/renderer/src/gpu_renderer.rs` - Camera reference

## Goals

1. Fix mesh renderer camera synchronization to match other renderers
2. Fix mesh renderer face culling/rendering issues
3. Add option to cache/regenerate mesh on demand instead of every frame
4. Rename `DualRendererApp` to `CubeRendererApp` throughout the codebase
5. Update README.md and documentation to reflect current capabilities

## Non-Goals

- Rewriting the mesh generation algorithm
- Adding new rendering backends
- Changing the GUI framework or layout
- Performance optimization beyond mesh caching

## Impact

### Users
- **Positive**: Mesh renderer will produce correct output for visual comparison
- **Positive**: Mesh caching option will improve frame rates when editing voxel data
- **Positive**: Clearer naming makes code easier to understand
- **Neutral**: Documentation updates reflect current state

### Codebase
- **Low Impact**: Mostly local changes to renderer crate
- **Renaming**: `DualRendererApp` -> `CubeRendererApp` (search and replace)
- **New Feature**: Mesh cache toggle in UI settings

### Dependencies
- **None**: No external dependency changes

## Success Criteria

1. Mesh renderer displays same viewpoint as other renderers
2. Mesh faces render correctly without visual artifacts
3. Mesh caching option available and functional
4. Mesh is not regenerated unnecessarily
5. Code uses "CubeRenderer" naming consistently
6. Documentation accurately describes current capabilities
7. All existing tests pass
8. Performance improvement measurable with caching enabled

## Development Environment

```bash
# Run renderer tests
cargo test --package renderer

# Check renderer builds
cargo check -p renderer

# Run renderer in single mode
cargo run -p renderer --single

# Run renderer in GUI mode
cargo run -p renderer
```

## Test Results

### Manual Testing
- Tested via `cargo run -p renderer --single` and GUI mode
- All five renderers display correct viewpoint
- Mesh caching UI controls work correctly
- Cache status indicator updates properly

### Automated Testing
- All tests pass except pre-existing `test_lighting_toggle` which has a threshold issue unrelated to these changes
- Cargo clippy and fmt checks pass

### Performance
- Mesh upload time displayed in UI
- Caching prevents re-upload each frame
- Frame rate improvement visible for static scenes
