# Octant Indexing

Defines the coordinate system and indexing scheme for octree octant addressing.

## MODIFIED Requirements

### Requirement: Octant coordinates use 0/1 representation
**ID**: `octant-indexing.coordinates.binary`
**Priority**: High
**Status**: Modified

Octant coordinates within an octree node SHALL be represented using IVec3 with components of 0 or 1, where:
- 0 represents the lower half along an axis
- 1 represents the upper half along an axis

The coordinate representation MUST directly correspond to the binary structure of octant indices, such that component values map to bit positions in the index.

#### Scenario: Converting octant index to coordinates

Given an octant index from 0 to 7, the coordinate components are extracted using bit operations:
```rust
fn from_octant_index(index: usize) -> IVec3 {
    IVec3::new(
        ((index >> 2) & 1) as i32,  // x component
        ((index >> 1) & 1) as i32,  // y component
        (index & 1) as i32           // z component
    )
}
```

**Before**: Coordinates used -1/+1 and required lookup table
**After**: Coordinates use 0/1 with direct bit extraction

#### Scenario: Converting coordinates to octant index

Given octant coordinates as IVec3(x, y, z) where each component is 0 or 1:
```rust
fn to_octant_index(coords: IVec3) -> usize {
    ((coords.x << 2) | (coords.y << 1) | coords.z) as usize
}
```

**Before**: Required comparison-based conversion: `(x > 0) as usize * 4 + ...`
**After**: Direct bit manipulation without comparisons

#### Scenario: Octant positions lookup table

The OCTANT_POSITIONS constant provides pre-computed coordinates:
```rust
pub const OCTANT_POSITIONS: [IVec3; 8] = [
    IVec3::new(0, 0, 0),  // 0: x=0,y=0,z=0 (---)
    IVec3::new(0, 0, 1),  // 1: x=0,y=0,z=1 (--+)
    IVec3::new(0, 1, 0),  // 2: x=0,y=1,z=0 (-+-)
    IVec3::new(0, 1, 1),  // 3: x=0,y=1,z=1 (-++)
    IVec3::new(1, 0, 0),  // 4: x=1,y=0,z=0 (+--)
    IVec3::new(1, 0, 1),  // 5: x=1,y=0,z=1 (+-+)
    IVec3::new(1, 1, 0),  // 6: x=1,y=1,z=0 (++-)
    IVec3::new(1, 1, 1),  // 7: x=1,y=1,z=1 (+++)
];
```

**Before**: Used -1/+1 components
**After**: Uses 0/1 components matching binary index structure

### Requirement: Step function returns 0/1 coordinates
**ID**: `octant-indexing.step-function.binary`
**Priority**: High
**Status**: Modified

The `step0()` function SHALL convert arbitrary integer coordinates to their corresponding octant coordinates (0 or 1). Negative coordinates MUST map to 0, and non-negative coordinates MUST map to 1.

#### Scenario: Normalizing coordinates to octant space

Given any IVec3 position, determine which half of the cube it occupies:
```rust
fn step0(pos: IVec3) -> IVec3 {
    IVec3::new(
        (pos.x >= 0) as i32,  // 0 if negative, 1 if non-negative
        (pos.y >= 0) as i32,
        (pos.z >= 0) as i32,
    )
}
```

**Before**: Returned -1 for negative, +1 for non-negative
**After**: Returns 0 for negative, 1 for non-negative

#### Scenario: Octant determination in raycast

During ray-octree traversal, determine which octant contains a point:
```rust
let pos = IVec3::new(-5, 3, 0);
let octant_coords = pos.step0();  // IVec3(0, 1, 1)
let octant_index = octant_coords.to_octant_index();  // 3
```

**Before**: `step0()` would return `IVec3(-1, 1, 1)`, requiring conversion
**After**: `step0()` returns `IVec3(0, 1, 1)`, ready for direct indexing

### Requirement: Raycast octant calculation uses 0/1 coordinates
**ID**: `octant-indexing.raycast.binary`
**Priority**: High
**Status**: Modified

Ray-octree intersection algorithms SHALL use 0/1 octant coordinates for traversal decisions. The `compute_octant()` function MUST return IVec3 with components of 0 or 1.

#### Scenario: Computing initial octant from ray origin

When starting a raycast, determine the initial octant based on ray position and direction:
```rust
fn compute_octant(pos: Vec3, dir_sign: Vec3) -> IVec3 {
    let positive = pos.cmpgt(Vec3::ZERO)
        | (pos.cmpeq(Vec3::ZERO) & dir_sign.cmpgt(Vec3::ZERO));
    Vec3::select(positive, Vec3::ONE, Vec3::ZERO).as_ivec3()
}
```

This function now directly returns 0/1 coordinates instead of requiring conversion.

**Before**: Returned -1/+1 values requiring conversion to index
**After**: Returns 0/1 values ready for direct bit operations

#### Scenario: Octant traversal step

When the ray crosses an octant boundary, update the octant coordinates:
```rust
// Ray crossed the x-axis boundary toward positive
let mut octant = IVec3::new(0, 1, 0);
octant.x = 1;  // Now IVec3(1, 1, 0) = octant index 6
```

**Before**: Would flip between -1 and +1, requiring conversion
**After**: Flips between 0 and 1, directly usable as index bits

### Requirement: Neighbor grid indexing uses 0/1 coordinates
**ID**: `octant-indexing.neighbor-grid.binary`
**Priority**: Medium
**Status**: Modified

Neighbor grid calculations for face visibility SHALL use 0/1 octant coordinates. Child octant index calculation MUST use direct bit manipulation instead of dot product operations.

#### Scenario: Child octant calculation from position

When building a neighbor grid, calculate child octant indices from position:
```rust
// Position in parent space (0 or 1 for each component)
let child_pos = IVec3::new(1, 0, 1);
// Direct conversion to octant index using bit operations
let child_octant = (child_pos.x << 2) | (child_pos.y << 1) | child_pos.z;  // 5
```

**Before**: Used dot product with `IVec3::new(4, 2, 1)` on -1/+1 coordinates
**After**: Direct bit manipulation on 0/1 coordinates

**Related**: This change eliminates the need for the dot product calculation in `neighbor_grid.rs:201`

## ADDED Requirements

None. All requirements are modifications to existing octant indexing behavior.

## REMOVED Requirements

None. The octant indexing functionality is preserved; only the internal representation changes.
