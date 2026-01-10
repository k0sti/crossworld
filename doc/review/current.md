# Task Review: Add drag paint target gizmo

**Task ID:** 909b
**Branch:** vk/909b-add-drag-paint-t
**Date:** 2026-01-10

## Summary

Added a drag paint target gizmo to the voxel editor with improved coordinate handling. The implementation includes:

1. **Dual gizmo system**: Yellow gizmo for paint start position, cyan gizmo for current target position
2. **Alpha transparency grid**: 2D grid with gradient fade (opaque at cursor, transparent at distance 3)
3. **Grid alignment**: Grid aligns with global voxel boundaries while staying on the edit plane
4. **Refactored coordinate system**: Cleaner conversion between world space and voxel space
5. **Far/Near mode support**: Cursor correctly offsets based on focus mode during drag painting

## Changes Made

### Files Modified
- `crates/renderer/src/renderers/mesh_renderer.rs` - Added alpha transparency support for wireframes
- `crates/editor/src/lib.rs` - Added drag target gizmo, grid rendering, and coordinate refactoring

### Key Changes

#### Renderer Changes (mesh_renderer.rs)
1. **Updated fragment shader** to support RGBA colors (changed `uColorOverride` from `vec3` to `vec4`)
2. **Added `render_cubebox_wireframe_colored_alpha()`** function with OpenGL blending support
3. **Updated existing functions** to use the new RGBA shader uniforms

#### Editor Changes (lib.rs)

**Coordinate System Refactoring:**
1. **Replaced `world_to_voxel()` with `world_to_nearest_voxel_corner()`**
   - Changed from `floor()` to `round()` for consistent corner snapping
   - Eliminates offset bugs from boundary conditions

2. **Added `voxel_corner_to_world()`**
   - Bidirectional conversion for cleaner code
   - Used for grid origin calculation

**Key Improvements:**
1. **Fixed plane origin calculation** - Now uses exact raycast hit position instead of voxel center with 0.5 offset
2. **Added `drag_target_world_pos` field** - Stores exact ray-plane intersection for precise gizmo positioning
3. **Grid origin snapping** - Snaps to global voxel grid at current depth, then projects onto plane
4. **Far/Near mode during drag** - Applies proper offset based on focus mode (lines 1014-1020):
   - Far mode: `nearest_corner + normal_ivec * 2` (place voxels two steps away)
   - Near mode: `nearest_corner + normal_ivec` (place voxels one step away)
5. **Target gizmo rendering** - Cyan gizmo at exact mouse ray-plane intersection (unsnapped)
6. **Grid transparency** - Gradient alpha from 1.0 at cursor to 0.0 at distance 3 voxels

## Refactoring Benefits

The coordinate system refactoring addresses the structural issues:

**Before:**
- `world_to_voxel()` used `floor()` → inconsistent at boundaries
- Multiple coordinate conversions scattered throughout code
- Offset logic mixed with coordinate calculations

**After:**
- `world_to_nearest_voxel_corner()` uses `round()` → consistent snapping
- `voxel_corner_to_world()` provides bidirectional conversion
- Clean separation: coordinate conversion → offset application → rendering

This makes the code easier to understand and maintain.

## Testing

### Tests Run
```bash
just check
```
All checks passed:
- ✅ Cargo check --workspace
- ✅ Cargo clippy --workspace
- ✅ Cargo fmt --check
- ✅ WASM build (release mode)
- ✅ TypeScript build

### Manual Testing Required
- Test Far mode offset during drag painting (should place voxels two steps away from nearest corner)
- Test Near mode during drag painting (should place voxels one step away from nearest corner)
- Verify grid aligns with voxel boundaries on all faces
- Verify grid transparency gradient works correctly
- Verify grid stays on the edit plane
- Test on different cube faces (X, Y, Z axes)

## Implementation Details

### Offset Calculation (lib.rs:1014-1020)
The offset is applied after calculating the nearest voxel corner from the ray-plane intersection:

```rust
// Calculate normal as integer vector
let normal_ivec = IVec3::new(
    plane.normal.x.round() as i32,
    plane.normal.y.round() as i32,
    plane.normal.z.round() as i32,
);

let voxel_coord = if self.cursor.focus_mode == FocusMode::Far {
    // Far mode: offset two voxels in the normal direction from nearest corner
    nearest_corner + normal_ivec * 2
} else {
    // Near mode: offset one voxel in the normal direction from nearest corner
    nearest_corner + normal_ivec
};
```

### Coordinate Conversion
```rust
// World to voxel corner (using round for consistent snapping)
fn world_to_nearest_voxel_corner(&self, world_pos: Vec3, ...) -> IVec3 {
    let scale = (1 << self.depth) as f32 / 2.0;
    IVec3::new(
        ((cube_pos.x + 1.0) * scale).round() as i32,  // round, not floor!
        ((cube_pos.y + 1.0) * scale).round() as i32,
        ((cube_pos.z + 1.0) * scale).round() as i32,
    )
}

// Voxel corner to world (bidirectional)
fn voxel_corner_to_world(&self, voxel_pos: IVec3, ...) -> Vec3 {
    let cube_pos = Vec3::new(
        voxel_pos.x as f32 / scale - 1.0,
        voxel_pos.y as f32 / scale - 1.0,
        voxel_pos.z as f32 / scale - 1.0,
    );
    cube_position + cube_pos * half_scale
}
```

### Grid Alignment Algorithm
```rust
// 1. Snap target position to nearest voxel corner
let nearest_corner = plane.world_to_nearest_voxel_corner(target_world_pos, CUBE_POSITION, CUBE_SCALE);

// 2. Convert back to world space
let corner_world_pos = plane.voxel_corner_to_world(nearest_corner, CUBE_POSITION, CUBE_SCALE);

// 3. Project onto plane to maintain plane alignment
let offset = corner_world_pos - plane.origin;
let distance_along_normal = offset.dot(plane.normal);
let grid_origin = corner_world_pos - plane.normal * distance_along_normal;
```

### Alpha Transparency
- OpenGL blending enabled when alpha < 1.0: `glBlendFunc(SRC_ALPHA, ONE_MINUS_SRC_ALPHA)`
- Grid uses gradient alpha: `alpha = (1.0 - (distance / max_dist)).clamp(0.0, 1.0)`
- Max distance set to 3 voxels as requested

## Known Issues / Testing Notes

The voxel placement offset was recently updated to use `nearest_corner + normal_ivec * 2` for Far mode and `nearest_corner + normal_ivec` for Near mode. This adds the additional +1 offset that was requested.

---

## Reviewer Response

Enter one or more commands (one per line). **At least one command is required.**

### Available Commands

| Command | Usage | Description |
|---------|-------|-------------|
| `APPROVE` | `APPROVE` | Approve changes, mark task as done |
| `CONTINUE` | `CONTINUE: <feedback>` | Request changes, provide feedback |
| `SPAWN` | `SPAWN: <task title>` | Create follow-up task (can use multiple times) |
| `DISCARD` | `DISCARD` | Cancel task and discard all changes |
| `REBASE` | `REBASE` | Rebase branch onto main before merge |
| `MERGE` | `MERGE` | Merge branch to main |
| `COMMENT` | `COMMENT: <note>` | Add a comment without changing status |

### Example Responses

**Simple approval:**
```
APPROVE
```

**Approve with follow-up tasks:**
```
APPROVE
SPAWN: Add unit tests for edge cases
SPAWN: Update API documentation
MERGE
```

**Request changes:**
```
CONTINUE: Please handle the null case in line 42
```

**Approve after rebase:**
```
APPROVE
REBASE
MERGE
```
