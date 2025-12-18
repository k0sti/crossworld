# Design: Cube Collision System

## Context

The physics system needs efficient collision detection between:
1. **Static ground Cube** - Large terrain octree, fixed position
2. **Dynamic CubeObjects** - Small voxel models that move and rotate

Current `VoxelColliderBuilder` generates compound colliders from ALL exposed faces, which is expensive and doesn't scale for dynamic collision updates.

### Stakeholders
- Physics simulation (Rapier3D integration)
- proto-gl renderer (uses physics for falling cubes)
- WASM physics module (must be fully WASM-compatible)

### Constraints
- **Must compile to WASM** (`wasm32-unknown-unknown`) - no `#[cfg(not(wasm))]` exclusions
- Must integrate with Rapier3D colliders
- Must support existing `CubeObject` API
- Cube crate already has `traverse_octree` and `NeighborGrid` infrastructure
- Use only glam types (Vec3, IVec3, Quat, Mat4) - no parry/nalgebra dependencies in core logic

## Goals / Non-Goals

### Goals
- Efficient collision between CubeObjects and static Cube ground
- Efficient collision between two CubeObjects
- AABB/OBB bounding volume for tight octree region calculation
- Region-bounded face traversal (1-8 octants based on bounds intersection)
- SDF-based collision design for fabric/procedural cubes
- Complete documentation of collision math
- Full WASM compatibility

### Non-Goals
- Real-time mesh deformation
- Soft-body physics
- Multi-threaded collision detection (future work)
- Broad-phase optimizations (Rapier handles this)

## Decisions

### Decision 1: Axis-Aligned Bounding Box (AABB) with OBB Support

**What**: Each CubeObject has an AABB calculated from its octree. When rotated, compute world-space AABB from the transformed OBB corners.

**Why OBB/AABB over Sphere**:
- Cubes are inherently box-shaped - OBB is a tight fit (volume ratio 1:1)
- Spheres waste ~47% volume on a unit cube (sphere volume π/6 ≈ 0.52 vs cube volume 1.0)
- AABB-AABB and AABB-octant tests are trivial (min/max comparisons)
- OBB transforms to world AABB with 8 corner transforms + min/max
- Aligns naturally with octree structure (axis-aligned subdivisions)

**Why not pure OBB-OBB tests**:
- OBB-OBB intersection (SAT) requires 15 axis tests - more complex
- For octree region lookup, we need AABB anyway (octants are axis-aligned)
- WASM-compatible: uses only glam types

**Calculation**:
```rust
/// Axis-Aligned Bounding Box using glam types (WASM-compatible)
#[derive(Clone, Copy, Debug)]
pub struct Aabb {
    pub min: Vec3,
    pub max: Vec3,
}

impl Aabb {
    /// Create AABB for unit cube [0,1]³
    pub fn unit() -> Self {
        Self {
            min: Vec3::ZERO,
            max: Vec3::ONE,
        }
    }

    /// Transform local AABB to world space given position, rotation, scale
    /// Computes tight AABB around the rotated box (OBB → AABB)
    pub fn to_world(&self, position: Vec3, rotation: Quat, scale: f32) -> Self {
        // 8 corners of the local box
        let corners = [
            Vec3::new(self.min.x, self.min.y, self.min.z),
            Vec3::new(self.max.x, self.min.y, self.min.z),
            Vec3::new(self.min.x, self.max.y, self.min.z),
            Vec3::new(self.max.x, self.max.y, self.min.z),
            Vec3::new(self.min.x, self.min.y, self.max.z),
            Vec3::new(self.max.x, self.min.y, self.max.z),
            Vec3::new(self.min.x, self.max.y, self.max.z),
            Vec3::new(self.max.x, self.max.y, self.max.z),
        ];

        // Transform corners and find min/max
        let mut world_min = Vec3::splat(f32::MAX);
        let mut world_max = Vec3::splat(f32::MIN);

        for corner in corners {
            let world_corner = position + rotation * (corner * scale);
            world_min = world_min.min(world_corner);
            world_max = world_max.max(world_corner);
        }

        Self { min: world_min, max: world_max }
    }

    /// Test intersection with another AABB
    pub fn intersects(&self, other: &Aabb) -> bool {
        self.min.x <= other.max.x && self.max.x >= other.min.x &&
        self.min.y <= other.max.y && self.max.y >= other.min.y &&
        self.min.z <= other.max.z && self.max.z >= other.min.z
    }

    /// Compute intersection volume (returns None if no overlap)
    pub fn intersection(&self, other: &Aabb) -> Option<Aabb> {
        let min = self.min.max(other.min);
        let max = self.max.min(other.max);

        if min.x < max.x && min.y < max.y && min.z < max.z {
            Some(Aabb { min, max })
        } else {
            None
        }
    }
}
```

### Decision 2: Intersection Region as CubeCoord + Size

**What**: When an AABB intersects a Cube, output octree coordinates identifying the intersection volume as `CubeCoord` (base position) + `IVec3` (size 1 or 2 per axis).

**Why**:
- Limits face traversal to relevant octants only
- Size Vec3I with values 1 or 2 indicates whether AABB extends into adjacent octant
- Natural fit with octree structure
- AABB intersection is exact (no wasted volume like spheres)

**Algorithm**:
```rust
/// Region of octree that intersects with a bounding volume
#[derive(Clone, Copy, Debug)]
pub struct IntersectionRegion {
    pub coord: CubeCoord,   // Base octant coordinate (min corner)
    pub size: IVec3,        // 1 or 2 in each dimension
}

impl IntersectionRegion {
    /// Calculate intersection region between world AABB and cube octree
    pub fn from_aabb(
        world_aabb: &Aabb,        // World space AABB
        cube_pos: Vec3,           // Cube world position
        cube_scale: f32,          // Cube world scale
        depth: u32,               // Octree depth for region granularity
    ) -> Option<Self> {
        // Transform AABB to cube's [0,1] local space
        let local_min = (world_aabb.min - cube_pos) / cube_scale;
        let local_max = (world_aabb.max - cube_pos) / cube_scale;

        // Quick rejection: AABB outside [0,1] bounds
        if local_max.x < 0.0 || local_min.x > 1.0 { return None; }
        if local_max.y < 0.0 || local_min.y > 1.0 { return None; }
        if local_max.z < 0.0 || local_min.z > 1.0 { return None; }

        // Clamp to [0,1] bounds
        let clamped_min = local_min.max(Vec3::ZERO);
        let clamped_max = local_max.min(Vec3::ONE);

        // Convert to octant coordinates at given depth
        let scale = (1 << depth) as f32;
        let min_octant = (clamped_min * scale).floor().as_ivec3();
        let max_octant = ((clamped_max * scale).ceil().as_ivec3() - IVec3::ONE).max(min_octant);

        // Size is difference + 1, clamped to 1 or 2
        let size = (max_octant - min_octant + IVec3::ONE).min(IVec3::splat(2));

        Some(Self {
            coord: CubeCoord::new(min_octant, depth),
            size,
        })
    }

    /// Number of octants covered (1 to 8)
    pub fn octant_count(&self) -> usize {
        (self.size.x * self.size.y * self.size.z) as usize
    }

    /// Iterate over all covered octant coordinates
    pub fn iter_coords(&self) -> impl Iterator<Item = CubeCoord> + '_ {
        let base = self.coord.pos;
        let depth = self.coord.depth;
        (0..self.size.x).flat_map(move |dx| {
            (0..self.size.y).flat_map(move |dy| {
                (0..self.size.z).map(move |dz| {
                    CubeCoord::new(base + IVec3::new(dx, dy, dz), depth)
                })
            })
        })
    }
}
```

### Decision 3: Region-Bounded Face Traversal

**What**: New traversal function that only visits voxels within a specified region.

**Why**:
- Existing `traverse_octree` visits ALL voxels
- Collision only needs faces in intersection volume
- Reduces face count by 70-90% for typical collisions

**API**:
```rust
/// Traverse faces within a bounded region
fn traverse_faces_in_region(
    grid: &NeighborGrid,
    region: IntersectionRegion,
    visitor: &mut dyn FnMut(FaceInfo) -> bool,
);
```

### Decision 4: Collision Modes

**What**: Support two collision detection modes:

1. **Face-based** (current): Generate thin cuboid colliders for exposed faces
2. **SDF-based** (future): Use signed distance function for smooth surfaces

**Why**:
- Face-based works for standard voxel models
- SDF-based enables smooth collision with fabric/procedural surfaces
- Fabric system already has `is_surface()` and `calculate_normal()` in `surface.rs`

**SDF Interface**:
```rust
trait SdfCollider {
    /// Signed distance from point to surface (negative = inside)
    fn sdf(&self, point: Vec3) -> f32;

    /// Surface normal at point (gradient of SDF)
    fn normal(&self, point: Vec3) -> Vec3;
}

// Fabric cubes implement this using quaternion magnitude
impl SdfCollider for FabricCube {
    fn sdf(&self, point: Vec3) -> f32 {
        let quat = self.sample(point);
        quat.length() - 1.0  // |Q| < 1 = inside, |Q| > 1 = outside
    }
}
```

### Decision 5: Reuse vs Drop from Current Code

**Reuse**:
- `VoxelColliderBuilder::rotation_from_normal()` - Still needed for face orientation
- `VoxelColliderBuilder::build_compound_collider()` - Final collider assembly
- `FaceRectangle` struct - Face representation
- `NeighborGrid` and `traverse_octree` infrastructure
- `is_surface()` from fabric/surface.rs for SDF detection

**Drop/Replace**:
- `VoxelColliderBuilder::from_cube()` - Replace with `from_cube_region()`
- Full octree traversal in collider generation - Replace with bounded traversal

**Refactor**:
- `from_cube_region()` exists but needs better integration with IntersectionRegion

## Risks / Trade-offs

### Risk 1: AABB Looseness Under Rotation
When a cube rotates 45°, its world AABB grows by √2 ≈ 1.41x per axis.

**Mitigation**: Acceptable trade-off. The expanded AABB still provides tighter bounds than a bounding sphere, and the octree traversal naturally handles the expanded region efficiently.

### Risk 2: SDF Complexity
SDF collision requires iterative root-finding, more complex than face-based.

**Mitigation**: Phase 2 implementation. Face-based handles all current use cases.

### Risk 3: WASM Compatibility
All collision code must work in WASM without conditional compilation.

**Mitigation**:
- Use only glam types (`Vec3`, `IVec3`, `Quat`) - no parry/nalgebra in core logic
- Custom `Aabb` struct instead of `rapier3d::parry::bounding_volume::Aabb`
- Rapier integration only at final collider generation step
- All tests must pass with `cargo test --target wasm32-unknown-unknown`

## Migration Plan

### Phase 1: Face-based Collision (This Change)
1. Add `BoundingSphere` calculation
2. Add `IntersectionRegion` calculation
3. Add region-bounded face traversal
4. Refactor `VoxelColliderBuilder` to use regions
5. Add collision.md documentation

### Phase 2: SDF Collision (Future)
1. Define `SdfCollider` trait
2. Implement for FabricCube
3. Add SDF-based contact generation
4. Hybrid mode: face-based for normal cubes, SDF for fabric

### Rollback
No breaking changes to public API. Old `from_cube()` remains available, just calls `from_cube_region()` with full bounds internally.

## Open Questions

1. **Depth selection for intersection region**: Should depth match object's octree depth or be configurable?
   - **Proposed**: Use object's native depth for accuracy

2. **Multi-object collision optimization**: When many CubeObjects collide with ground, should we batch region calculations?
   - **Proposed**: Single-object API first, batch optimization in Phase 2

3. **SDF collision tolerance**: What epsilon for surface detection?
   - **Proposed**: 0.001 world units (typical voxel size / 1000)
