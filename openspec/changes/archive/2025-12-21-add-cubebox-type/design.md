## Context

The voxel system loads MagicaVoxel (.vox) models and converts them to octree structures. Currently, the `align` parameter controls where within the power-of-2 octree bounds the model is positioned:
- `align = (0.5, 0.5, 0.5)` centers the model
- `align = (0.5, 0.0, 0.5)` aligns the model bottom to y=0

After loading, the original model dimensions are lost. This causes problems when:
1. Placing models into world terrain (need to know actual footprint)
2. Generating physics colliders (need actual bounds, not octree bounds)
3. Scaling models consistently (need original size for proper scale calculation)

## Goals / Non-Goals

**Goals:**
- Preserve original voxel model bounds after loading
- Provide a clean API for placing bounded models into larger cubes
- Support combining multiple models into a ground/world cube
- Maintain backward compatibility with existing `load_vox_to_cube()` API

**Non-Goals:**
- Changing the octree structure or traversal algorithms
- Adding animation or multi-frame model support
- Changing the BCF (Binary Cube Format) serialization

## Decisions

### Decision 1: CubeBox struct design

```rust
/// A bounded voxel model with explicit dimensions
pub struct CubeBox<T> {
    /// The octree containing the voxel data
    pub cube: Cube<T>,
    /// Original model size in voxels (not power-of-2 aligned)
    pub size: IVec3,
    /// Octree depth - size is measured in units of 2^depth
    pub depth: u32,
}
```

**Rationale:**
- `size` stores actual model dimensions (e.g., 16x30x12), not the power-of-2 octree size
- `depth` is needed to interpret `size` correctly: at depth=5, size=(16,30,12) means the model occupies 16x30x12 out of a 32x32x32 octree
- The model is always positioned at the origin (0,0,0) within the octree - no alignment offset stored

### Decision 2: Placement API

```rust
impl<T: Clone + Default + PartialEq> CubeBox<T> {
    /// Place this box model into a target cube at the specified position
    ///
    /// # Arguments
    /// * `target` - The cube to place into
    /// * `target_depth` - Depth of the target cube
    /// * `position` - Position in target cube coordinates (corner-based)
    /// * `scale` - Additional scale factor (0 = 1:1, 1 = 2x, -1 = 0.5x)
    ///
    /// # Returns
    /// New cube with the model placed
    pub fn place_in(&self, target: &Cube<T>, target_depth: u32, position: IVec3, scale: i32) -> Cube<T>;
}
```

**Rationale:**
- Uses existing `update_depth_tree` internally for efficiency
- `scale` is an exponent (2^scale) matching the existing scale_exp pattern
- Position uses corner-based coordinates matching octree conventions

**Alternatives considered:**
- Storing alignment offset in CubeBox: Rejected because it adds complexity and different use cases need different alignments
- Using center-based positioning: Rejected for consistency with existing octree coordinate system

### Decision 3: Vox loading changes

New function that returns bounds information:

```rust
pub fn load_vox_to_cubebox(bytes: &[u8]) -> Result<CubeBox<u8>, String>
```

**Rationale:**
- No alignment parameter - model always starts at (0,0,0)
- Returns actual dimensions from vox file
- Caller can use `place_in()` with desired alignment

Keep existing function for backward compatibility:

```rust
#[deprecated(note = "Use load_vox_to_cubebox() instead")]
pub fn load_vox_to_cube(bytes: &[u8], align: Vec3) -> Result<Cube<u8>, String>
```

### Decision 4: WASM bindings

```typescript
interface WasmCubeBox {
  cube: WasmCube;
  sizeX: number;
  sizeY: number;
  sizeZ: number;
  depth: number;

  // Place this model into a target cube
  placeIn(target: WasmCube, targetDepth: number, x: number, y: number, z: number, scale: number): WasmCube;
}

// New loading function
function loadVoxBox(bytes: Uint8Array): WasmCubeBox;
```

## Risks / Trade-offs

**Risk: Breaking existing code**
- Mitigation: Keep `load_vox_to_cube()` as deprecated but functional
- Mitigation: Phased migration with clear deprecation warnings

**Risk: Memory overhead of storing bounds**
- Trade-off: 16 bytes per CubeBox (IVec3 + u32) vs simpler API
- Acceptable: Most applications have few model instances

**Risk: Complexity in placement logic**
- Mitigation: `place_in()` handles coordinate conversion internally
- Testing: Unit tests for various alignment scenarios

## Migration Plan

1. Add `CubeBox` type and `load_vox_to_cubebox()` function
2. Update `proto-gl` models and structures to use `CubeBox`
3. Update WASM bindings with new API
4. Update TypeScript voxLoader to use new API
5. Deprecate old `load_vox_to_cube()` with align parameter
6. (Future) Remove deprecated function after migration complete

## Open Questions

1. **Should CubeBox store the original vox filename/source?**
   - Current decision: No, keep it minimal
   - Could be added later if debugging/tracing needs it

2. **Should placement support rotation?**
   - Current: Rotation handled separately (see `rotate_y_90` in structures.rs)
   - Could add rotation to `place_in()` for convenience
