# Capability: terrain-collision

Provides lazy on-demand collision geometry generation for voxel terrain using Rapier's TypedSimdCompositeShape trait.

## ADDED Requirements

### Requirement: Region Identification
The system SHALL provide compact identifiers for collision regions within the octree.

RegionId MUST:
- Use corner-based IVec3 coordinates at a specified depth
- Be convertible to cube::RegionBounds for octree queries
- Compute local [0,1] space AABB for the region
- Generate all regions intersecting a given world AABB

#### Scenario: Create RegionId at depth 3
- **GIVEN** a corner position (2, 1, 3) at depth 3
- **WHEN** creating a RegionId
- **THEN** the region SHALL represent 1/8 of the octree per axis
- **AND** local AABB SHALL be (0.25, 0.125, 0.375) to (0.375, 0.25, 0.5)

#### Scenario: Convert RegionId to RegionBounds
- **GIVEN** a RegionId with pos (1, 0, 2) at depth 2
- **WHEN** converting to RegionBounds
- **THEN** bounds.pos SHALL equal (1, 0, 2)
- **AND** bounds.depth SHALL equal 2
- **AND** bounds.size SHALL equal (1, 1, 1)

#### Scenario: Find regions intersecting world AABB
- **GIVEN** a world AABB from (-10, 0, -10) to (10, 5, 10)
- **AND** world_size 100.0 and region depth 2
- **WHEN** iterating regions from world AABB
- **THEN** iterator SHALL yield all 4 regions in the center quadrant

### Requirement: Triangle Part Identification
The system SHALL encode region and triangle index into a compact 64-bit identifier.

TerrainPartId MUST:
- Pack depth (4 bits), x/y/z (16 bits each), and triangle_idx (12 bits) into u64
- Support up to 4096 triangles per region
- Provide extraction methods for region and triangle_idx
- Provide face_idx (triangle_idx / 2) and triangle_in_face (0 or 1)

#### Scenario: Encode and decode TerrainPartId
- **GIVEN** RegionId with pos (5, 3, 7) at depth 4
- **AND** triangle_idx 42
- **WHEN** creating TerrainPartId and extracting components
- **THEN** region().pos SHALL equal (5, 3, 7)
- **AND** region().depth SHALL equal 4
- **AND** triangle_idx() SHALL equal 42

#### Scenario: Extract face index from part id
- **GIVEN** TerrainPartId with triangle_idx 7
- **WHEN** extracting face_idx and triangle_in_face
- **THEN** face_idx() SHALL equal 3
- **AND** triangle_in_face() SHALL equal 1

### Requirement: Face to Triangle Conversion
The system SHALL convert cube::FaceInfo to Rapier Triangle shapes.

The conversion MUST:
- Generate 2 triangles per face (quad split)
- Transform face position from [0,1] space to world space
- Use correct winding order for outward-facing normals
- Handle all 6 face directions (Left, Right, Bottom, Top, Back, Front)

#### Scenario: Convert Top face to triangles
- **GIVEN** FaceInfo with face=Top, position=(0.25, 0.25, 0.25), size=0.25
- **AND** world_size=100.0 (world from -50 to 50)
- **WHEN** converting to triangles
- **THEN** 2 triangles SHALL be generated
- **AND** all triangle vertices SHALL have Y = -25.0 (voxel top surface)
- **AND** triangle normals SHALL point +Y (upward)

#### Scenario: Convert Left face to triangles
- **GIVEN** FaceInfo with face=Left, position=(0.5, 0.5, 0.5), size=0.125
- **AND** world_size=64.0
- **WHEN** converting to triangles
- **THEN** 2 triangles SHALL be generated
- **AND** all triangle vertices SHALL have X = 0.0 (voxel left surface)

### Requirement: Region Collision Cache
The system SHALL cache collision data per region to avoid repeated octree traversal.

RegionCollisionData MUST:
- Store region identifier, AABB, and vector of FaceInfo
- Build from octree using visit_faces_in_region()
- Return triangle count as faces.len() * 2
- Retrieve individual triangles by index
- Compute triangle AABB for QBVH construction

#### Scenario: Build cache for solid region
- **GIVEN** an octree with solid voxels in region (0, 0, 0) at depth 2
- **WHEN** building RegionCollisionData
- **THEN** faces vector SHALL contain all exposed faces
- **AND** triangle_count() SHALL equal faces.len() * 2

#### Scenario: Retrieve triangle from cache
- **GIVEN** RegionCollisionData with 10 faces
- **WHEN** requesting triangle at index 15
- **THEN** face_idx SHALL be 7 (15 / 2)
- **AND** triangle_in_face SHALL be 1 (15 % 2)
- **AND** returned Triangle SHALL match face[7] second triangle

#### Scenario: Build cache for empty region
- **GIVEN** an octree with only air in region (1, 1, 1)
- **WHEN** building RegionCollisionData
- **THEN** faces vector SHALL be empty
- **AND** triangle_count() SHALL equal 0

### Requirement: Terrain Composite Collider
The system SHALL provide a Rapier-compatible collider that generates terrain geometry on demand.

VoxelTerrainCollider MUST:
- Maintain region-level QBVH for coarse spatial queries
- Maintain triangle-level QBVH for active regions only
- Cache RegionCollisionData for populated regions
- Rebuild triangle QBVH when active region changes significantly
- Invalidate cache when terrain is modified

#### Scenario: Initialize terrain collider
- **GIVEN** an octree representing terrain
- **AND** world_size=1024.0 and region_depth=3
- **WHEN** creating VoxelTerrainCollider
- **THEN** region_qbvh SHALL contain entries for non-empty regions
- **AND** triangle_qbvh SHALL be empty (no active region yet)

#### Scenario: Update for active region
- **GIVEN** initialized VoxelTerrainCollider
- **AND** active AABB covering 4 regions
- **WHEN** calling update_triangle_qbvh()
- **THEN** region_cache SHALL contain data for those 4 regions
- **AND** triangle_qbvh SHALL contain all triangles from those regions

#### Scenario: Terrain modification invalidation
- **GIVEN** VoxelTerrainCollider with cached region (1, 2, 3)
- **WHEN** calling on_terrain_modified() for that region
- **THEN** region_cache SHALL NOT contain (1, 2, 3)
- **AND** terrain_version SHALL be incremented

### Requirement: Active Region Tracking
The system SHALL track which regions need triangle-level collision data.

ActiveRegionTracker MUST:
- Compute bounding AABB of all dynamic body AABBs
- Add configurable margin for velocity prediction
- Trigger QBVH rebuild only when region expands beyond current bounds
- Use hysteresis to avoid thrashing on boundary movement

#### Scenario: Initial dynamic body positions
- **GIVEN** no previous active region
- **AND** dynamic bodies with combined AABB (0, 0, 0) to (10, 10, 10)
- **WHEN** calling update()
- **THEN** return value SHALL be Some(expanded_aabb)
- **AND** current_aabb SHALL include margin

#### Scenario: Small movement within bounds
- **GIVEN** previous active region (0, 0, 0) to (20, 20, 20) with margin 5
- **AND** dynamic bodies now at (2, 2, 2) to (12, 12, 12)
- **WHEN** calling update()
- **THEN** return value SHALL be None (no rebuild needed)

#### Scenario: Movement beyond bounds
- **GIVEN** previous active region (0, 0, 0) to (20, 20, 20)
- **AND** dynamic bodies now at (15, 15, 15) to (30, 30, 30)
- **WHEN** calling update()
- **THEN** return value SHALL be Some(new_expanded_aabb)

### Requirement: TypedSimdCompositeShape Implementation
The system SHALL implement Rapier's TypedSimdCompositeShape trait for terrain collision.

The implementation MUST:
- Set PartShape = Triangle
- Set PartId = TerrainPartId
- Implement map_typed_part_at() to retrieve triangles from cache
- Implement typed_qbvh() to return the triangle-level QBVH
- Handle missing cache entries gracefully (no panic)

#### Scenario: Query triangle via trait
- **GIVEN** VoxelTerrainCollider with cached region containing 5 faces
- **AND** TerrainPartId pointing to triangle 3
- **WHEN** Rapier calls map_typed_part_at()
- **THEN** callback SHALL receive the correct Triangle shape
- **AND** no isometry transform SHALL be applied (None)

#### Scenario: Query missing triangle
- **GIVEN** VoxelTerrainCollider with empty cache
- **AND** TerrainPartId for non-cached region
- **WHEN** Rapier calls map_typed_part_at()
- **THEN** callback SHALL NOT be invoked
- **AND** no panic SHALL occur

### Requirement: Shape Trait Implementation
The system SHALL implement Rapier's Shape trait for terrain collision.

The implementation MUST:
- Return global terrain AABB from compute_local_aabb()
- Return bounding sphere derived from AABB
- Return ShapeType::Custom from shape_type()
- Return zero mass properties (static terrain)

#### Scenario: Query terrain AABB
- **GIVEN** VoxelTerrainCollider with world_size=512
- **WHEN** calling compute_local_aabb()
- **THEN** AABB SHALL span (-256, -256, -256) to (256, 256, 256)

#### Scenario: Query mass properties
- **GIVEN** VoxelTerrainCollider
- **WHEN** calling mass_properties()
- **THEN** mass SHALL be 0 (infinite/static)
- **AND** center of mass SHALL be origin
