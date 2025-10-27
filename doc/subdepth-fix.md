# Subunit Voxel Rendering Issue

## Problem
When drawing subunit voxels (depth > macro_depth), the mesh appears corrupted or in the wrong location.

## Architecture Overview

### Coordinate Spaces
- **World space**: Physical units, size = 2^macro_depth (e.g., 8×8×8 units for macro=3)
- **Octree space at depth D**: Integer coordinates [0, 2^D)
- **Normalized space**: [0, 1] used by mesh generator

### Example Values (macro_depth=3)
- World size: 8 units (2^3)
- Half world: 4 units
- Depth 3 (unit voxels): octree space [0, 8)
- Depth 5 (0.25 unit): octree space [0, 32)
- Depth 6 (render_depth): octree space [0, 64)

## Drawing Flow

### 1. User places voxel at world position (2.0, 0.0, 2.5) with depth=5

**TypeScript coordinate conversion:**
```typescript
// depth=5, macro_depth=3
const scale = 1 << (depth - macroDepth); // 2^(5-3) = 4
const octreeX = Math.floor((worldX + halfWorld) * scale);
// octreeX = floor((2 + 4) * 4) = floor(24) = 24
```

Result: `CubeCoord { x: 24, y: 16, z: 26, depth: 5 }`

### 2. Rust receives and places voxel

```rust
pub fn set_voxel_at_depth(&mut self, x: i32, y: i32, z: i32, depth: u32, color_index: i32) {
    let pos = IVec3::new(x, y, z); // pos = (24, 16, 26)
    self.octree.root = self
        .octree
        .root
        .updated(Cube::Solid(color_index), depth, pos) // depth=5
        .simplified();
}
```

The octree subdivides to place a voxel at position (24, 16, 26) in depth-5 coordinate space.

### 3. Mesh generation

```rust
// Current implementation
let render_depth = macro_depth + 3; // 3 + 3 = 6

crossworld_cube::generate_face_mesh(
    &self.octree.root,
    &mut builder,
    |index| color_mapper.map(index),
    self.render_depth,  // Passing 6 here
);
```

**The mesh generator interprets this depth parameter as the coordinate space size!**

When generating mesh for a voxel at position (24, 16, 26) with `depth=6`:
- Octree space: [0, 64) (2^6)
- The generator creates vertices in normalized space relative to depth 6
- Position 24 → normalized position ≈ 24/64 = 0.375

### 4. Vertex scaling

```rust
let world_size = (1 << self.macro_depth) as f32; // 8.0
let half_world = world_size / 2.0; // 4.0

let x = chunk[0] * world_size - half_world;
// x = 0.375 * 8 - 4 = -1.0
```

**This is WRONG!** The voxel should be at world position 2.0, not -1.0!

## Root Cause

The depth parameter to `generate_face_mesh` defines the coordinate space, not the traversal depth.

**Mismatch:**
- Voxels placed at position 24 in depth-5 space (range [0, 32))
- Mesh generator interprets positions using depth-6 space (range [0, 64))
- Vertex scaling uses macro_depth=3 world size (8 units)

**Correct mapping for depth-5 voxel:**
```
Position 24 in depth-5 space:
→ 24 positions out of 32 total
→ 24/32 = 0.75 in normalized space
→ 0.75 * 8 - 4 = 2.0 world units ✓
```

**What's happening (incorrect):**
```
Position 24 interpreted as depth-6:
→ 24 positions out of 64 total
→ 24/64 = 0.375 in normalized space
→ 0.375 * 8 - 4 = -1.0 world units ✗
```

## Solution Options

### Option 1: Scale vertices correctly for render_depth
```rust
let coord_space_size = (1 << self.render_depth) as f32; // 64.0
let world_size = (1 << self.macro_depth) as f32; // 8.0

// Vertices come in [0,1] range normalized to render_depth coordinate space
// Need to scale by world_size directly since mesh gen already normalized
let x = chunk[0] * world_size - half_world;
```

**Wait, this doesn't solve it either!** The mesh generator outputs coordinates assuming a certain coordinate space.

### Option 2: Pass macro_depth to mesh generator, it auto-traverses deeper

The mesh generator might automatically traverse deeper than the specified depth when it encounters subdivided nodes. We should test passing `macro_depth` and see if it renders subdivisions.

```rust
crossworld_cube::generate_face_mesh(
    &self.octree.root,
    &mut builder,
    |index| color_mapper.map(index),
    self.macro_depth,  // Not render_depth
);
```

But we tried this before and it didn't render subdivisions...

### Option 3: The depth parameter means something else

Need to check crossworld_cube library documentation to understand what the depth parameter actually controls:
- Is it "coordinate space size"?
- Is it "maximum traversal depth"?
- Is it something else?

## Investigation Results

### Traversal Function Behavior
```rust
// From neighbor_traversal.rs:247
if coord.depth == 0 {
    return;
}
```

The traversal **stops at depth 0** after decrementing from `max_depth`. It does NOT continue deeper than specified!

- Pass `macro_depth=3`: only traverses to depth 3, won't find depth-5 voxels ✗
- Pass `render_depth=6`: traverses to depth 6, finds all voxels ✓

### Mesh Generator Coordinate Space
```rust
// From face_builder.rs:87-88
let grid_size = 1 << max_depth; // 2^max_depth
let voxel_size = 1.0 / grid_size as f32;
```

Vertices are generated as: `position = octree_coord * voxel_size`

The `max_depth` parameter defines the coordinate space grid size!

## The Real Solution

The issue is that `render_depth` and `macro_depth` serve different purposes:

1. **render_depth**: How deep to traverse (must be >= actual voxel depths)
2. **macro_depth**: World size (2^macro_depth units)

But currently, vertices are scaled incorrectly. The mesh generator outputs vertices in [0,1] space where 1 = `render_depth` coordinate space, but we scale by `macro_depth` world size directly.

### Correct Vertex Scaling

The mesh generator with `render_depth=6` outputs:
- Vertex for position 24 → 24/64 = 0.375 in normalized [0,1]

We need to recognize that this 0.375 represents 0.375 of the render_depth coordinate space (64 positions), not 0.375 of the world:

```rust
// Current (WRONG):
let x = chunk[0] * world_size - half_world;
// chunk[0]=0.375, world_size=8 → x = -1.0 ✗

// Should be:
// chunk[0] is already correctly normalized to [0,1] for the world!
// The mesh generator handles the coordinate space internally
let x = chunk[0] * world_size - half_world;
// This IS correct if mesh generator outputs correctly...
```

Wait, if the mesh generator is already normalizing correctly, then the scaling should work...

## ACTUAL ROOT CAUSE

After analyzing the code, the issue is that **the mesh generator uses `max_depth` for BOTH:**
1. Traversal depth limit
2. Coordinate space normalization

When we place a voxel at depth=5, it's stored in the octree. When the mesh generator traverses with `max_depth=6`, it finds the voxel but generates coordinates assuming ALL octree positions are in depth-6 space.

The fix is to **always use macro_depth** for both traversal AND coordinate space, since that's the actual octree root coordinate space. Subdivided voxels are children of the root space, and the mesh generator should handle them correctly at their actual stored depth.

But we tried that and subdivisions weren't rendered... which means the mesh generator stops at leaf nodes and doesn't see subdivisions.

## Alternative: Check if the `updated()` method works correctly

Maybe the voxels aren't actually being placed in the octree at the correct depth? Need to verify `octree.root.updated()` is working as expected.
