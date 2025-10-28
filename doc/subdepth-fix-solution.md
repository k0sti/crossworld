# Subdepth Voxel Rendering - Solution

## Problem Summary
When placing voxels at depths greater than `macro_depth` (e.g., depth=5 when macro_depth=3), the mesh appeared corrupted or at incorrect world positions. This occurred because the mesh generator calculated voxel size once at the beginning, assuming all voxels were at the same depth.

## Root Cause
The issue had two components:

1. **Insufficient Traversal Depth**: When passing `macro_depth` to the mesh generator, traversal stopped after `macro_depth` levels, never reaching subdivided voxels placed at greater depths.

2. **Incorrect Voxel Size Calculation**: The `voxel_size` was calculated once at the beginning of `generate_face_mesh`:
   ```rust
   let grid_size = 1 << max_depth;
   let voxel_size = 1.0 / grid_size as f32;  // Calculated ONCE for all voxels
   ```
   This assumed all leaf voxels were at the same depth, causing position errors for voxels at different levels.

## Solution

### 1. Dynamic Render Depth (cube_ground/mod.rs)
Added `render_depth` field to `CubeGround`:
```rust
pub struct CubeGround {
    octree: Octree,
    macro_depth: u32,   // World size = 2^macro_depth
    render_depth: u32,  // Maximum traversal depth = macro_depth + micro_depth
    face_mesh_mode: bool,
}
```

The render depth is calculated as:
```rust
let render_depth = macro_depth + micro_depth;
```

This ensures the mesh generator traverses deep enough to find all voxels, including user-placed subdivisions.

### 2. Per-Voxel Size Calculation (face_builder.rs)
Changed voxel size calculation from once-at-start to per-voxel inside the visitor:

**Before:**
```rust
let grid_size = 1 << max_depth;
let voxel_size = 1.0 / grid_size as f32;  // Calculated once

traverse_with_neighbors(&grid, &mut |view, coord| {
    let x = coord.pos.x as f32 * voxel_size;  // Wrong for voxels at different depths
    // ...
}, max_depth);
```

**After:**
```rust
traverse_with_neighbors(&grid, &mut |view, coord| {
    // Calculate voxel size based on actual depth of this voxel
    let voxel_size = 1.0 / (1 << (max_depth - coord.depth)) as f32;

    let x = coord.pos.x as f32 * voxel_size;  // Correct for this voxel's level
    // ...
}, max_depth);
```

## How It Works

The `coord.depth` field counts down from `max_depth` as we traverse deeper:
- At root: `depth = max_depth`
- At level N: `depth = max_depth - N`

The coordinate space size at any level is `2^(max_depth - coord.depth)`, so:
```rust
voxel_size = 1.0 / (1 << (max_depth - coord.depth))
```

### Example (macro_depth=3, micro_depth=3, render_depth=6)

**Terrain voxel at level 3:**
- `coord.depth = 3` (6 - 3 levels traversed)
- `voxel_size = 1.0 / 2^(6-3) = 1.0 / 8`
- Position in octree space: [0, 8)
- Normalized position: pos * 1/8 → [0, 1) ✓

**User voxel at level 5:**
- `coord.depth = 1` (6 - 5 levels traversed)
- `voxel_size = 1.0 / 2^(6-1) = 1.0 / 32`
- Position in octree space: [0, 32)
- Normalized position: pos * 1/32 → [0, 1) ✓

Both voxel types now correctly normalize to [0, 1] space, and the existing world scaling (`world_size = 2^macro_depth`) correctly converts them to world coordinates.

## Hierarchical Mesh Builder
The hierarchical mesh builder (`mesh_builder.rs`) was already handling this correctly by calculating size per-voxel:
```rust
let scale_factor = 1 << remaining_depth;
let size = voxel_size * scale_factor as f32;
```

Only the face-based mesh generator needed this fix.

## Impact
- ✅ Terrain voxels (at macro_depth) render correctly
- ✅ User-placed voxels (at macro+micro_depth) render at correct positions
- ✅ Mixed-depth octrees render correctly with proper face culling
- ✅ World scale remains constant regardless of subdivision depth
- ✅ All existing tests pass
