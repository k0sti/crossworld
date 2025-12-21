# Capability: cube-collision

Provides WASM-compatible collision primitives for voxel objects.

## Purpose

Enable efficient collision detection between voxel cubes by providing AABB bounding volumes, intersection region calculation, and region-bounded face traversal - all fully compatible with WASM targets.
## Requirements
### Requirement: WASM-Compatible Bounding Volume
The system SHALL provide AABB (Axis-Aligned Bounding Box) types using only glam types for full WASM compatibility.

The AABB implementation MUST:
- Use only `glam::Vec3` for min/max bounds (no parry/nalgebra dependencies)
- Support transformation to world space given position, rotation (Quat), and scale
- Compute tight world-space AABB from rotated OBB (8 corner transformation)
- Provide AABB-AABB intersection testing
- Compile without conditional compilation flags for WASM

#### Scenario: Create AABB for unit cube
- **GIVEN** a unit cube [0,1]³
- **WHEN** creating its local AABB
- **THEN** min SHALL be (0, 0, 0)
- **AND** max SHALL be (1, 1, 1)

#### Scenario: Transform AABB to world space with rotation
- **GIVEN** a unit AABB [0,1]³
- **AND** world position (10, 0, 0), 45° Y rotation, scale 1.0
- **WHEN** transforming to world space
- **THEN** world AABB SHALL enclose the rotated cube
- **AND** world AABB size SHALL be approximately (√2, 1, √2)

#### Scenario: AABB intersection test
- **GIVEN** AABB A with min (0,0,0) max (1,1,1)
- **AND** AABB B with min (0.5,0.5,0.5) max (1.5,1.5,1.5)
- **WHEN** testing intersection
- **THEN** result SHALL be true
- **AND** intersection volume SHALL have min (0.5,0.5,0.5) max (1,1,1)

### Requirement: Intersection Region Calculation
The system SHALL calculate the octree region where a world-space AABB intersects a Cube.

The intersection region MUST:
- Output a CubeCoord identifying the base octant (min corner)
- Output a size IVec3 with values 1 or 2 per axis indicating extent
- Return None if AABB does not intersect cube bounds
- Transform AABB to cube's local [0,1] space before calculation
- Provide iterator over all covered octant coordinates

#### Scenario: AABB fully inside single octant
- **GIVEN** a local AABB with min (0.1, 0.1, 0.1) max (0.2, 0.2, 0.2)
- **AND** octree depth 2 (4 divisions per axis)
- **WHEN** calculating intersection region
- **THEN** size SHALL be (1, 1, 1)
- **AND** coord SHALL identify the octant containing the AABB

#### Scenario: AABB spans adjacent octants
- **GIVEN** a local AABB with min (0.4, 0.4, 0.4) max (0.6, 0.6, 0.6)
- **AND** octree depth 2
- **WHEN** calculating intersection region
- **THEN** size SHALL be (2, 2, 2)
- **AND** region SHALL cover up to 8 octants

#### Scenario: AABB outside cube bounds
- **GIVEN** a local AABB with min (1.5, 0.0, 0.0) max (2.0, 0.5, 0.5)
- **WHEN** calculating intersection region
- **THEN** result SHALL be None

### Requirement: Region-Bounded Face Traversal
The system SHALL traverse only faces within a specified intersection region.

The traversal MUST:
- Accept an IntersectionRegion specifying bounds
- Visit only voxels whose centers fall within the region
- Call visitor function for each exposed face
- Support early termination via visitor return value

#### Scenario: Traverse faces in small region
- **GIVEN** a solid cube and intersection region covering 1 octant
- **WHEN** traversing faces in region
- **THEN** only faces within that octant SHALL be visited
- **AND** face count SHALL be less than full traversal

#### Scenario: Traverse faces in multi-octant region
- **GIVEN** a solid cube and intersection region with size (2, 2, 1)
- **WHEN** traversing faces in region
- **THEN** faces in 4 octants (2×2×1) SHALL be visited

### Requirement: Cube to CubeObject Collision
The system SHALL detect and generate colliders for collisions between a static Cube (ground) and dynamic CubeObjects.

The collision MUST:
- Calculate CubeObject's world-space AABB (from rotated OBB)
- Determine intersection region with ground Cube
- Generate compound collider from faces in intersection region
- Support objects at any position, rotation, and scale relative to ground
- Be fully WASM-compatible

#### Scenario: CubeObject resting on ground
- **GIVEN** a static ground Cube at position (0, 0, 0) with scale 1.0
- **AND** a CubeObject at position (0.5, 0.1, 0.5) with AABB size 0.2
- **WHEN** calculating collision
- **THEN** intersection region SHALL identify top surface of ground
- **AND** collider SHALL contain upward-facing ground faces

#### Scenario: Rotated CubeObject on ground
- **GIVEN** a static ground Cube at position (0, 0, 0)
- **AND** a CubeObject at position (0.5, 0.1, 0.5) rotated 45° around Y
- **WHEN** calculating collision
- **THEN** world AABB SHALL be computed from rotated corners
- **AND** intersection region SHALL cover expanded footprint

#### Scenario: CubeObject not touching ground
- **GIVEN** a static ground Cube with top surface at Y=1
- **AND** a CubeObject at position (0.5, 2.0, 0.5) with AABB size 0.2
- **WHEN** calculating collision
- **THEN** intersection region SHALL be None
- **AND** no collider SHALL be generated

### Requirement: CubeObject to CubeObject Collision
The system SHALL detect and generate colliders for collisions between two dynamic CubeObjects.

The collision MUST:
- Calculate world-space AABBs for both objects (from rotated OBBs)
- Perform AABB-AABB intersection test first
- If AABBs intersect, calculate overlapping region in each object's local space
- Generate colliders for faces in both objects' intersection regions
- Be fully WASM-compatible

#### Scenario: Two CubeObjects colliding
- **GIVEN** CubeObject A at position (0, 0, 0) with AABB half-size 0.5
- **AND** CubeObject B at position (0.8, 0, 0) with AABB half-size 0.5
- **WHEN** calculating collision
- **THEN** AABB-AABB test SHALL detect intersection
- **AND** colliders SHALL be generated for overlapping faces

#### Scenario: Two CubeObjects not colliding
- **GIVEN** CubeObject A at position (0, 0, 0) with AABB half-size 0.5
- **AND** CubeObject B at position (2, 0, 0) with AABB half-size 0.5
- **WHEN** calculating collision
- **THEN** AABB-AABB test SHALL return false
- **AND** no collider generation SHALL occur

#### Scenario: Rotated CubeObjects colliding
- **GIVEN** CubeObject A at origin, rotated 30° around Y
- **AND** CubeObject B nearby, rotated 60° around Y
- **WHEN** calculating collision
- **THEN** world AABBs SHALL be computed from rotated corners
- **AND** intersection region SHALL use expanded world AABBs

### Requirement: SDF Collision Interface
The system SHALL define an interface for SDF-based collision detection to support fabric and procedural cubes.

The interface MUST:
- Define `sdf(point: Vec3) -> f32` returning signed distance (negative inside)
- Define `normal(point: Vec3) -> Vec3` returning surface normal
- Be implementable by fabric cubes using quaternion magnitude
- Support contact point generation from SDF gradients

#### Scenario: Fabric cube SDF evaluation
- **GIVEN** a fabric cube with quaternion field
- **AND** a point inside the surface (|Q| < 1.0)
- **WHEN** evaluating SDF
- **THEN** result SHALL be negative

#### Scenario: Fabric cube normal calculation
- **GIVEN** a fabric cube with quaternion field
- **AND** a point on the surface
- **WHEN** calculating normal
- **THEN** normal SHALL point from solid toward air
- **AND** normal SHALL be normalized

### Requirement: Collision Documentation
The system SHALL provide documentation describing all collision calculations.

The documentation MUST:
- Explain bounding sphere calculation with formulas
- Explain intersection region algorithm
- Describe face-based collision generation
- Describe SDF-based collision (current design + future implementation)
- Include diagrams for sphere-cube intersection
- Be located at `doc/architecture/collision.md`

#### Scenario: Developer reads collision documentation
- **GIVEN** a developer unfamiliar with the collision system
- **WHEN** reading `doc/architecture/collision.md`
- **THEN** they SHALL understand bounding sphere calculation
- **AND** they SHALL understand region-based face traversal
- **AND** they SHALL understand the SDF interface design

### Requirement: VoxelColliderBuilder Region Support
The VoxelColliderBuilder SHALL support generating colliders from a bounded region.

The `from_cube_region()` function MUST:
- Accept an optional AABB or IntersectionRegion parameter
- Filter voxels to those within the specified region
- Fall back to full traversal when region is None
- Reduce collider complexity proportional to region size

#### Scenario: Generate collider for partial region
- **GIVEN** a solid cube with 6 faces at depth 0
- **AND** an intersection region covering 25% of volume
- **WHEN** generating collider from region
- **THEN** collider SHALL contain approximately 25% of faces

#### Scenario: Generate collider without region (full)
- **GIVEN** a solid cube
- **AND** no region specified (None)
- **WHEN** generating collider
- **THEN** collider SHALL contain all exposed faces

