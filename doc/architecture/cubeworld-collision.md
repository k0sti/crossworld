# Voxel Terrain Collision System Design

## Overview

Implement terrain collision using Rapier's `TypedSimdCompositeShape` trait, allowing lazy on-demand triangle generation from voxel data. The terrain appears as a single collider to Rapier, but geometry is generated only for regions actively queried during collision detection.

This design integrates with the existing cube crate's octree traversal system, using `visit_faces_in_region()` and `RegionBounds` for efficient spatial queries.

## Architecture

```
┌─────────────────────────────────────────────────────────┐
│                     Rapier Physics Pipeline             │
├─────────────────────────────────────────────────────────┤
│  Broadphase: terrain AABB vs object AABBs               │
│       ↓                                                 │
│  Narrowphase: query VoxelTerrainCollider                │
│       ↓                                                 │
│  QBVH traversal → yields region IDs near object         │
│       ↓                                                 │
│  map_typed_part_at() → generate triangles from faces    │
│       ↓                                                 │
│  Contact solver receives (object, triangle) pairs       │
└─────────────────────────────────────────────────────────┘

Integration with Cube Crate:

┌─────────────────────────────────────────────────────────┐
│                    Cube Octree                          │
├─────────────────────────────────────────────────────────┤
│  visit_faces_in_region(cube, bounds, visitor)           │
│       ↓                                                 │
│  FaceInfo { face, position, size, material_id }         │
│       ↓                                                 │
│  face_to_triangles(face_info) → [Triangle; 2]           │
│       ↓                                                 │
│  Cached in active region for QBVH queries               │
└─────────────────────────────────────────────────────────┘
```

## Core Data Structures

### RegionId (Cube-Compatible)

Use cube crate's corner-based coordinates for region identification:

```rust
use cube::RegionBounds;
use glam::{IVec3, Vec3};

/// Identifies a collision region in the octree
///
/// Uses corner-based coordinates at a fixed depth, compatible with
/// cube::RegionBounds for efficient octree queries.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct RegionId {
    /// Corner position at region_depth (in [0, 2^depth) per axis)
    pub pos: IVec3,
    /// Depth level (determines region granularity)
    pub depth: u32,
}

impl RegionId {
    pub fn new(pos: IVec3, depth: u32) -> Self {
        Self { pos, depth }
    }

    /// Convert to cube::RegionBounds for octree queries
    pub fn to_region_bounds(&self) -> RegionBounds {
        RegionBounds::new(self.pos, self.depth, IVec3::ONE)
    }

    /// Compute AABB in local [0,1] space
    pub fn local_aabb(&self) -> (Vec3, Vec3) {
        let scale = 1.0 / (1 << self.depth) as f32;
        let min = self.pos.as_vec3() * scale;
        let max = min + Vec3::splat(scale);
        (min, max)
    }

    /// Create from world AABB (returns all regions that intersect)
    pub fn from_world_aabb(
        world_aabb: &Aabb,
        world_size: f32,
        depth: u32,
    ) -> impl Iterator<Item = RegionId> {
        let half_world = world_size / 2.0;
        let local_min = (world_aabb.min + Vec3::splat(half_world)) / world_size;
        let local_max = (world_aabb.max + Vec3::splat(half_world)) / world_size;

        let scale = (1 << depth) as f32;
        let min_pos = (local_min * scale).floor().as_ivec3().max(IVec3::ZERO);
        let max_pos = (local_max * scale).ceil().as_ivec3()
            .min(IVec3::splat((1 << depth) - 1));

        (min_pos.x..=max_pos.x).flat_map(move |x| {
            (min_pos.y..=max_pos.y).flat_map(move |y| {
                (min_pos.z..=max_pos.z).map(move |z| {
                    RegionId::new(IVec3::new(x, y, z), depth)
                })
            })
        })
    }
}
```

### PartId Encoding

Encode region position and triangle index into a single u64:

```rust
/// Identifies a specific triangle within the terrain
///
/// Encodes region coordinates and face/triangle index compactly.
/// Each face produces 2 triangles, so triangle_idx encodes:
/// - face_index * 2 + triangle_within_face (0 or 1)
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct TerrainPartId(u64);

impl TerrainPartId {
    /// Bits: [depth:4][x:16][y:16][z:16][tri_idx:12]
    const DEPTH_SHIFT: u32 = 60;
    const X_SHIFT: u32 = 44;
    const Y_SHIFT: u32 = 28;
    const Z_SHIFT: u32 = 12;
    const TRI_MASK: u64 = 0xFFF;  // 12 bits = 4096 triangles per region

    pub fn new(region: RegionId, triangle_idx: u16) -> Self {
        debug_assert!(region.depth <= 15, "depth must fit in 4 bits");
        debug_assert!(triangle_idx < 4096, "triangle_idx must fit in 12 bits");

        let packed = ((region.depth as u64) << Self::DEPTH_SHIFT)
            | ((region.pos.x as u64 & 0xFFFF) << Self::X_SHIFT)
            | ((region.pos.y as u64 & 0xFFFF) << Self::Y_SHIFT)
            | ((region.pos.z as u64 & 0xFFFF) << Self::Z_SHIFT)
            | (triangle_idx as u64);
        Self(packed)
    }

    pub fn region(&self) -> RegionId {
        RegionId {
            pos: IVec3::new(
                ((self.0 >> Self::X_SHIFT) & 0xFFFF) as i32,
                ((self.0 >> Self::Y_SHIFT) & 0xFFFF) as i32,
                ((self.0 >> Self::Z_SHIFT) & 0xFFFF) as i32,
            ),
            depth: ((self.0 >> Self::DEPTH_SHIFT) & 0xF) as u32,
        }
    }

    pub fn triangle_idx(&self) -> u16 {
        (self.0 & Self::TRI_MASK) as u16
    }

    /// Get face index (each face has 2 triangles)
    pub fn face_idx(&self) -> u16 {
        self.triangle_idx() / 2
    }

    /// Get triangle within face (0 or 1)
    pub fn triangle_in_face(&self) -> u8 {
        (self.triangle_idx() % 2) as u8
    }
}
```

### Triangle Generation from Faces

Convert cube crate `FaceInfo` to Rapier `Triangle`:

```rust
use cube::FaceInfo;
use rapier3d::parry::shape::Triangle;
use rapier3d::math::Point;

/// Generate two triangles from a voxel face
///
/// Each voxel face is a quad that gets split into 2 triangles.
/// Uses consistent winding order for correct normals.
pub fn face_to_triangles(face: &FaceInfo, world_size: f32) -> [Triangle; 2] {
    let half_world = world_size / 2.0;

    // Face position is the voxel corner in [0,1] space
    // Convert to world space centered at origin
    let voxel_pos = face.position * world_size - Vec3::splat(half_world);
    let size = face.size * world_size;

    // Get the 4 corners of the face quad based on face direction
    let corners = match face.face {
        Face::Left => [  // -X face
            voxel_pos + Vec3::new(0.0, 0.0, 0.0),
            voxel_pos + Vec3::new(0.0, size, 0.0),
            voxel_pos + Vec3::new(0.0, size, size),
            voxel_pos + Vec3::new(0.0, 0.0, size),
        ],
        Face::Right => [  // +X face
            voxel_pos + Vec3::new(size, 0.0, 0.0),
            voxel_pos + Vec3::new(size, 0.0, size),
            voxel_pos + Vec3::new(size, size, size),
            voxel_pos + Vec3::new(size, size, 0.0),
        ],
        Face::Bottom => [  // -Y face
            voxel_pos + Vec3::new(0.0, 0.0, 0.0),
            voxel_pos + Vec3::new(0.0, 0.0, size),
            voxel_pos + Vec3::new(size, 0.0, size),
            voxel_pos + Vec3::new(size, 0.0, 0.0),
        ],
        Face::Top => [  // +Y face
            voxel_pos + Vec3::new(0.0, size, 0.0),
            voxel_pos + Vec3::new(size, size, 0.0),
            voxel_pos + Vec3::new(size, size, size),
            voxel_pos + Vec3::new(0.0, size, size),
        ],
        Face::Back => [  // -Z face
            voxel_pos + Vec3::new(0.0, 0.0, 0.0),
            voxel_pos + Vec3::new(size, 0.0, 0.0),
            voxel_pos + Vec3::new(size, size, 0.0),
            voxel_pos + Vec3::new(0.0, size, 0.0),
        ],
        Face::Front => [  // +Z face
            voxel_pos + Vec3::new(0.0, 0.0, size),
            voxel_pos + Vec3::new(0.0, size, size),
            voxel_pos + Vec3::new(size, size, size),
            voxel_pos + Vec3::new(size, 0.0, size),
        ],
    };

    // Split quad into 2 triangles (0,1,2) and (0,2,3)
    let to_point = |v: Vec3| Point::new(v.x, v.y, v.z);
    [
        Triangle::new(to_point(corners[0]), to_point(corners[1]), to_point(corners[2])),
        Triangle::new(to_point(corners[0]), to_point(corners[2]), to_point(corners[3])),
    ]
}

/// Get a single triangle by index (0 or 1) from a face
pub fn face_to_triangle(face: &FaceInfo, tri_idx: u8, world_size: f32) -> Triangle {
    face_to_triangles(face, world_size)[tri_idx as usize]
}
```

### Region Cache

Cache faces per region to avoid repeated octree traversal:

```rust
use cube::{Cube, FaceInfo, visit_faces_in_region};
use std::collections::HashMap;

/// Cached collision data for a region
pub struct RegionCollisionData {
    pub region: RegionId,
    pub aabb: Aabb,
    pub faces: Vec<FaceInfo>,
    pub version: u64,
}

impl RegionCollisionData {
    /// Build from octree using cube crate traversal
    pub fn from_octree(
        cube: &Cube<u8>,
        region: RegionId,
        world_size: f32,
        border_materials: [u8; 4],
    ) -> Self {
        let bounds = region.to_region_bounds();
        let mut faces = Vec::new();

        visit_faces_in_region(cube, &bounds, |face_info| {
            faces.push(face_info.clone());
        }, border_materials);

        // Compute world-space AABB
        let (local_min, local_max) = region.local_aabb();
        let half_world = world_size / 2.0;
        let aabb = Aabb::new(
            local_min * world_size - Vec3::splat(half_world),
            local_max * world_size - Vec3::splat(half_world),
        );

        Self {
            region,
            aabb,
            faces,
            version: 0,
        }
    }

    /// Number of triangles (2 per face)
    pub fn triangle_count(&self) -> usize {
        self.faces.len() * 2
    }

    /// Get triangle by index
    pub fn get_triangle(&self, idx: u16, world_size: f32) -> Option<Triangle> {
        let face_idx = idx as usize / 2;
        let tri_in_face = (idx % 2) as u8;

        self.faces.get(face_idx).map(|face| {
            face_to_triangle(face, tri_in_face, world_size)
        })
    }

    /// Get triangle AABB by index
    pub fn get_triangle_aabb(&self, idx: u16, world_size: f32) -> Option<Aabb> {
        self.get_triangle(idx, world_size).map(|tri| {
            let min = tri.a.coords.inf(&tri.b.coords).inf(&tri.c.coords);
            let max = tri.a.coords.sup(&tri.b.coords).sup(&tri.c.coords);
            Aabb::new(
                Vec3::new(min.x, min.y, min.z),
                Vec3::new(max.x, max.y, max.z),
            )
        })
    }
}
```

### Main Terrain Collider

```rust
pub struct VoxelTerrainCollider {
    /// Coarse QBVH indexing regions
    region_qbvh: Qbvh<RegionId>,

    /// Fine QBVH indexing individual triangles within active region
    triangle_qbvh: Qbvh<TerrainPartId>,

    /// Reference to voxel octree
    cube: Arc<Cube<u8>>,

    /// World size in units
    world_size: f32,

    /// Border materials for octree traversal
    border_materials: [u8; 4],

    /// Region depth for collision queries (higher = finer regions)
    region_depth: u32,

    /// Cache of region collision data
    region_cache: HashMap<RegionId, RegionCollisionData>,

    /// Terrain modification counter for cache invalidation
    terrain_version: u64,

    /// Total bounding box of terrain (world space)
    global_aabb: Aabb,
}
```

## QBVH Strategy

### Two-Level Hierarchy

1. **Region-level QBVH**: Coarse spatial index of region AABBs. Cheap to update when terrain changes.

2. **Triangle-level QBVH**: Fine index of individual triangle AABBs. Built lazily for regions near dynamic objects.

### Update Flow

```rust
impl VoxelTerrainCollider {
    /// Called when terrain voxels are modified
    pub fn on_terrain_modified(&mut self, affected_regions: &[RegionId]) {
        self.terrain_version += 1;

        for &region in affected_regions {
            // Invalidate cache for this region
            self.region_cache.remove(&region);

            // Update region AABB in coarse QBVH
            let data = RegionCollisionData::from_octree(
                &self.cube,
                region,
                self.world_size,
                self.border_materials,
            );

            if data.faces.is_empty() {
                self.region_qbvh.remove(region);
            } else {
                self.region_qbvh.pre_update_or_insert(region);
                self.region_cache.insert(region, data);
            }
        }

        // Refit coarse QBVH
        self.region_qbvh.refit(0.01, |region| {
            self.region_cache.get(region)
                .map(|data| data.aabb.to_rapier())
                .unwrap_or_else(|| rapier3d::parry::bounding_volume::Aabb::new_invalid())
        });
    }

    /// Called before physics step to rebuild fine QBVH for active region
    pub fn update_triangle_qbvh(&mut self, active_aabb: &Aabb) {
        let mut entries = Vec::new();

        // Find all regions intersecting active area
        for region in RegionId::from_world_aabb(active_aabb, self.world_size, self.region_depth) {
            // Ensure region is cached
            let data = self.region_cache.entry(region).or_insert_with(|| {
                RegionCollisionData::from_octree(
                    &self.cube,
                    region,
                    self.world_size,
                    self.border_materials,
                )
            });

            // Add all triangles from this region
            for tri_idx in 0..data.triangle_count() as u16 {
                let part_id = TerrainPartId::new(region, tri_idx);
                if let Some(aabb) = data.get_triangle_aabb(tri_idx, self.world_size) {
                    entries.push((part_id, aabb.to_rapier()));
                }
            }
        }

        self.triangle_qbvh.clear_and_rebuild(entries.into_iter(), 0.01);
    }
}
```

## TypedSimdCompositeShape Implementation

```rust
impl TypedSimdCompositeShape for VoxelTerrainCollider {
    type PartShape = Triangle;
    type PartId = TerrainPartId;
    type PartNormalConstraints = ();

    fn map_typed_part_at(
        &self,
        part_id: TerrainPartId,
        mut f: impl FnMut(
            Option<&Isometry<Real>>,
            &Self::PartShape,
            Option<&Self::PartNormalConstraints>,
        ),
    ) {
        let region = part_id.region();
        let tri_idx = part_id.triangle_idx();

        // Look up in cache (should be populated by update_triangle_qbvh)
        if let Some(data) = self.region_cache.get(&region) {
            if let Some(tri) = data.get_triangle(tri_idx, self.world_size) {
                f(None, &tri, None);
            }
        }
    }

    fn map_untyped_part_at(
        &self,
        part_id: TerrainPartId,
        mut f: impl FnMut(
            Option<&Isometry<Real>>,
            &dyn Shape,
            Option<&Self::PartNormalConstraints>,
        ),
    ) {
        self.map_typed_part_at(part_id, |iso, tri, constraints| {
            f(iso, tri as &dyn Shape, constraints);
        });
    }

    fn typed_qbvh(&self) -> &Qbvh<TerrainPartId> {
        &self.triangle_qbvh
    }
}
```

## Shape Trait Implementation

Required for Rapier to accept as a collider shape:

```rust
impl Shape for VoxelTerrainCollider {
    fn compute_local_aabb(&self) -> rapier3d::parry::bounding_volume::Aabb {
        self.global_aabb.to_rapier()
    }

    fn compute_local_bounding_sphere(&self) -> BoundingSphere {
        self.global_aabb.to_rapier().bounding_sphere()
    }

    fn clone_box(&self) -> Box<dyn Shape> {
        Box::new(self.clone())
    }

    fn mass_properties(&self, _density: Real) -> MassProperties {
        // Static terrain - infinite mass
        MassProperties::new(Point::origin(), 0.0, 0.0)
    }

    fn shape_type(&self) -> ShapeType {
        ShapeType::Custom
    }

    fn as_typed_shape(&self) -> TypedShape {
        TypedShape::Custom(self)
    }

    fn ccd_thickness(&self) -> Real {
        0.0
    }

    fn ccd_angular_thickness(&self) -> Real {
        0.0
    }
}
```

## Cube Crate Integration

### New Traversal Function (Optional)

For better performance, add a function that collects faces with their AABBs:

```rust
// In crates/cube/src/traversal/visit_faces.rs

/// Collect all faces in a region with their world-space AABBs
///
/// This is optimized for collision detection where both face info
/// and AABB are needed together.
pub fn collect_faces_with_aabbs(
    root: &Cube<u8>,
    bounds: &RegionBounds,
    world_size: f32,
    border_materials: [u8; 4],
) -> Vec<(FaceInfo, Aabb)> {
    let half_world = world_size / 2.0;
    let mut result = Vec::new();

    visit_faces_in_region(root, bounds, |face_info| {
        // Compute face AABB in world space
        let voxel_pos = face_info.position * world_size - Vec3::splat(half_world);
        let size = face_info.size * world_size;

        // Face AABB depends on face direction
        let (face_min, face_max) = match face_info.face {
            Face::Left | Face::Right => {
                let x = if face_info.face == Face::Left { voxel_pos.x } else { voxel_pos.x + size };
                (Vec3::new(x, voxel_pos.y, voxel_pos.z),
                 Vec3::new(x, voxel_pos.y + size, voxel_pos.z + size))
            },
            Face::Bottom | Face::Top => {
                let y = if face_info.face == Face::Bottom { voxel_pos.y } else { voxel_pos.y + size };
                (Vec3::new(voxel_pos.x, y, voxel_pos.z),
                 Vec3::new(voxel_pos.x + size, y, voxel_pos.z + size))
            },
            Face::Back | Face::Front => {
                let z = if face_info.face == Face::Back { voxel_pos.z } else { voxel_pos.z + size };
                (Vec3::new(voxel_pos.x, voxel_pos.y, z),
                 Vec3::new(voxel_pos.x + size, voxel_pos.y + size, z))
            },
        };

        result.push((face_info.clone(), Aabb::new(face_min, face_max)));
    }, border_materials);

    result
}
```

### Required Imports

```rust
// In physics crate
use cube::{
    Cube, FaceInfo, RegionBounds,
    visit_faces_in_region, visit_voxels_in_region,
};
use cube::mesh::face::Face;
```

## Active Region Tracking

Determine which regions need triangle-level indexing:

```rust
pub struct ActiveRegionTracker {
    current_aabb: Aabb,
    margin: f32,
}

impl ActiveRegionTracker {
    pub fn new(margin: f32) -> Self {
        Self {
            current_aabb: Aabb::new(Vec3::ZERO, Vec3::ZERO),
            margin,
        }
    }

    /// Update active region based on dynamic body positions
    ///
    /// Returns Some(new_aabb) if region changed significantly, None otherwise.
    pub fn update(&mut self, dynamic_aabbs: &[Aabb]) -> Option<Aabb> {
        if dynamic_aabbs.is_empty() {
            return None;
        }

        // Compute union of all dynamic AABBs with margin
        let mut new_aabb = dynamic_aabbs[0];
        for aabb in &dynamic_aabbs[1..] {
            new_aabb = new_aabb.union(aabb);
        }

        // Add margin for velocity/prediction
        new_aabb.min -= Vec3::splat(self.margin);
        new_aabb.max += Vec3::splat(self.margin);

        // Only trigger rebuild if new region not contained in current
        if !self.contains(&new_aabb) {
            self.current_aabb = new_aabb.expanded(self.margin);
            Some(self.current_aabb)
        } else {
            None
        }
    }

    fn contains(&self, aabb: &Aabb) -> bool {
        self.current_aabb.min.x <= aabb.min.x
            && self.current_aabb.min.y <= aabb.min.y
            && self.current_aabb.min.z <= aabb.min.z
            && self.current_aabb.max.x >= aabb.max.x
            && self.current_aabb.max.y >= aabb.max.y
            && self.current_aabb.max.z >= aabb.max.z
    }
}
```

## Per-Frame Update Sequence

```rust
pub fn physics_frame(
    terrain_collider: &mut VoxelTerrainCollider,
    region_tracker: &mut ActiveRegionTracker,
    bodies: &RigidBodySet,
    colliders: &ColliderSet,
    pipeline: &mut PhysicsPipeline,
    // ... other rapier state
) {
    // 1. Collect dynamic body AABBs
    let dynamic_aabbs: Vec<_> = bodies
        .iter()
        .filter(|(_, b)| b.is_dynamic())
        .filter_map(|(handle, b)| {
            colliders.get(b.colliders()[0]).map(|c| {
                c.compute_aabb()
            })
        })
        .map(|aabb| Aabb::from_rapier(&aabb))
        .collect();

    // 2. Update active region and rebuild triangle QBVH if needed
    if let Some(new_region) = region_tracker.update(&dynamic_aabbs) {
        terrain_collider.update_triangle_qbvh(&new_region);
    }

    // 3. Run physics step - Rapier queries terrain via TypedSimdCompositeShape
    pipeline.step(...);
}
```

## Performance Considerations

### Region Depth Selection

| Depth | Regions per axis | Region size (world=1024) | Use case |
|-------|------------------|--------------------------|----------|
| 2     | 4                | 256 units               | Very coarse, few objects |
| 3     | 8                | 128 units               | Typical gameplay |
| 4     | 16               | 64 units                | Dense object placement |
| 5     | 32               | 32 units                | Fine-grained collision |

Recommended: depth 3-4 for typical voxel worlds.

### Triangle Count Limits

- TerrainPartId supports 4096 triangles per region (12 bits)
- At 2 triangles per face, this is 2048 faces per region
- Highly detailed terrain may need higher region depth

### Memory vs Compute Tradeoff

| Approach | Memory | CPU |
|----------|--------|-----|
| No caching | Low | High (regenerate each query) |
| Region cache | Medium | Low (cache hit) |
| Full triangle cache | High | Very low |

Recommended: Region cache with lazy population.

## File Structure

```
crates/physics/src/
  terrain/
    mod.rs              # Module exports
    region_id.rs        # RegionId, TerrainPartId
    triangle_gen.rs     # face_to_triangles, FaceInfo conversion
    region_cache.rs     # RegionCollisionData
    collider.rs         # VoxelTerrainCollider
    active_region.rs    # ActiveRegionTracker
    shape_impl.rs       # Shape trait impl

crates/cube/src/traversal/
  visit_faces.rs        # Existing + optional collect_faces_with_aabbs
```

## Implementation Steps

1. Implement `RegionId` and `TerrainPartId` encoding/decoding
2. Implement `face_to_triangles()` conversion
3. Implement `RegionCollisionData` with octree queries via `visit_faces_in_region`
4. Create `VoxelTerrainCollider` with region-level QBVH only
5. Implement `Shape` trait (minimal)
6. Implement `TypedSimdCompositeShape` with region cache lookup
7. Add triangle-level QBVH and `ActiveRegionTracker`
8. Profile and optimize hot paths

## Future Optimizations

1. **SIMD triangle generation**: Batch generate triangles using SIMD
2. **Async region loading**: Build region cache on background thread
3. **Predictive caching**: Pre-cache regions in movement direction
4. **LOD regions**: Use coarser regions for distant objects
