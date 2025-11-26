# Design: Integrate Cube Raycast into CPU Tracer

## Overview

Integrate the cube crate's octree raycast directly into `cpu_tracer.rs`, replacing the current usage of the `gpu_tracer::raycast()` stub.

## Architecture

```
┌─────────────────────────────────────────┐
│  CPU Tracer (cpu_tracer.rs)            │
│  ┌───────────────────────────────────┐  │
│  │ render_ray(ray: Ray)              │  │
│  │   1. Bounding box intersection    │  │
│  │   2. Transform to [0,1]³ space    │  │  ◄── NEW: Direct integration
│  │   3. Call cube.raycast()          │  │  ◄── NEW: Use cube crate
│  │   4. Convert RaycastHit            │  │  ◄── NEW: cube→renderer
│  │   5. Lighting calculation         │  │
│  └───────────────────────────────────┘  │
└─────────────────────────────────────────┘
                 │
                 ▼
┌─────────────────────────────────────────┐
│  Cube Raycast (cube/raycast/mod.rs)    │
│  ┌───────────────────────────────────┐  │
│  │ Cube::raycast(pos, dir, depth)   │  │
│  │   - Recursive octree traversal    │  │
│  │   - DDA-based stepping            │  │
│  │   - Normal calculation            │  │
│  └───────────────────────────────────┘  │
└─────────────────────────────────────────┘

Note: gpu_tracer.rs stub remains unchanged for future GPU implementation
```

## Coordinate System Transformation

### Current State
- `cpu_tracer.rs` calls `gpu_tracer::raycast()` stub after bounding box hit
- Stub returns miss for all subdivided cubes
- Position is in world space from bounding box

### Required Transformation (in cpu_tracer.rs)
- **Input**: `pos` in world space (from bounding box intersection)
- **Transform**: World space → Normalized [0,1]³ cube space
- **Raycast**: Call `cube.raycast()` in normalized space
- **Output**: Convert back to world space if needed

### Transformation Logic
```rust
// World space to normalized [0,1]³
let bounds = CubeBounds::default();
let normalized_pos = (pos - bounds.min) / (bounds.max - bounds.min);

// Call cube raycast in normalized space
let cube_hit = cube.raycast(
    normalized_pos,
    dir.normalize(),
    max_depth,
    &|v: &i32| *v == 0  // is_empty predicate
)?;

// Convert cube::RaycastHit to gpu_tracer::RaycastHit
```

## Data Structure Mapping

### cube::raycast::RaycastHit (BEFORE Enhancement)
```rust
pub struct RaycastHit {
    pub coord: CubeCoord,    // Octree coordinate (precise voxel path)
    pub position: Vec3,      // Hit position in [0,1]³ space
    pub normal: Vec3,        // Surface normal
}
// Returns: Option<RaycastHit> (None = miss)
```

**Problem**: Missing voxel value - renderer must traverse tree using `coord` to get it.

### cube::raycast::RaycastHit (AFTER Enhancement) ✨
```rust
pub struct RaycastHit<T> {  // Now generic!
    pub coord: CubeCoord,    // Octree coordinate (precise voxel path)
    pub position: Vec3,      // Hit position in [0,1]³ space
    pub normal: Vec3,        // Surface normal
    pub value: T,            // Voxel value at hit position ✨ NEW
}
// Returns: Option<RaycastHit<T>> (None = miss)
```

**Benefits**:
- ✅ Complete information: All data needed for rendering
- ✅ No tree traversal: Value extracted during raycast
- ✅ Type-safe: Works with any voxel type (`i32`, custom materials, etc.)
- ✅ Future-proof: Enables material systems, voxel colors, procedural data

### renderer::HitInfo (for lighting)
```rust
pub struct HitInfo {
    pub hit: bool,           // Did we hit?
    pub t: f32,              // Distance along ray
    pub point: Vec3,         // Hit point in world space
    pub normal: Vec3,        // Surface normal
}
```

### Conversion Strategy (in cpu_tracer.rs)
1. **None → background color**: If `cube.raycast()` returns `None`, use background
2. **Some(cube_hit) → HitInfo**:
   - `hit = true`
   - `point` = transform `cube_hit.position` from [0,1]³ to world space
   - `normal` = use `cube_hit.normal` directly
   - `t` = calculate distance from ray origin to world-space hit point
3. **Voxel value available**: `cube_hit.value` contains voxel data (for future materials)
4. **Pass HitInfo to lighting**: Existing `calculate_lighting()` unchanged

**Note**: We don't need `gpu_tracer::RaycastHit` for CPU tracer - convert directly to `HitInfo`

**Future**: When material system is added, `cube_hit.value` can be used for:
- Voxel colors/textures
- Material properties (metallic, roughness, etc.)
- Procedural generation data
- Custom per-voxel attributes

## Implementation Approach

### Phase 1: Enhance cube::RaycastHit
1. Make `RaycastHit` generic over voxel type `T`
2. Add `value: T` field to structure
3. Update `raycast_recursive()` to extract voxel value at hit
4. Update all 30 tests to use `RaycastHit<i32>`
5. Verify backward compatibility (signature changes but semantics same)

### Phase 2: Integrate into CPU Tracer
1. Import `cube::raycast::RaycastHit`
2. Add coordinate transformation (world → normalized)
3. Call `cube.raycast()` with correct parameters
4. Convert `Option<RaycastHit<i32>>` to `HitInfo`
5. Transform hit position back to world space
6. Pass to lighting calculation

### Phase 3: Testing & Validation
1. Test solid cube rendering (baseline)
2. Test subdivided octree rendering (new capability)
3. Verify voxel values are correct
4. Visual validation with test scenes
5. Performance validation

### Phase 4: Future Enhancements (Not in this proposal)
- Material system using `value` field
- Voxel color mapping
- Procedural voxel attributes
- GPU shader integration

## Edge Cases

### 1. Miss (No Hit)
- `cube.raycast()` returns `None`
- Return `RaycastHit { hit: false, .. }`

### 2. Empty Voxel Hit
- `is_empty` predicate filters out value==0 voxels
- Only solid voxels (value != 0) trigger hits

### 3. Coordinate Bounds
- Ensure normalized position is clamped to [0,1]³
- Handle floating-point precision at boundaries

### 4. Max Depth
- Use reasonable default (e.g., 8 or 10)
- May make configurable in future

## Testing Strategy

### Unit Tests
- Test coordinate transformation (world ↔ normalized)
- Test RaycastHit conversion
- Test is_empty predicate

### Integration Tests
- Render solid cube (should still work)
- Render subdivided octree (should work now)
- Compare output before/after (solid cube unchanged)
- Verify normals are correct
- Verify hit positions are accurate

### Visual Tests
- Render test scene with cubscript `">a [1 2 3 4 5 6 7 8]"`
- Save output image, verify visually
- Compare with expected octree rendering

## Migration Path

Since this is replacing a stub, migration is straightforward:
1. Replace stub implementation
2. Existing solid cube renders should work unchanged
3. Subdivided cubes will render correctly (new capability)
4. No API changes needed

## Future Work

Not included in this change:
- GPU shader implementation (separate change)
- Performance optimizations
- Configurable max_depth
- Voxel color mapping system
