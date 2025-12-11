# voxel-raycast Specification

## Purpose
TBD - created by archiving change reimplement-raycast. Update Purpose after archive.
## Requirements
### Requirement: Octree Ray Traversal
The GPU raycast system SHALL traverse octree structures using recursive DDA (Digital Differential Analyzer) to find the first solid voxel intersection along a ray path.

#### Scenario: Solid voxel hit
- **WHEN** a ray intersects a solid voxel (non-zero value)
- **THEN** return hit information with coordinate, position, and normal
- **AND** stop traversal immediately (early termination)

#### Scenario: Empty voxel traversal
- **WHEN** a ray passes through an empty voxel (value = 0)
- **THEN** continue traversal to next octant
- **AND** use DDA stepping to skip empty space efficiently

#### Scenario: Octree subdivision traversal
- **WHEN** a ray enters a subdivided octant (Cube::Cubes with children)
- **THEN** recursively traverse into the appropriate child octant
- **AND** transform ray position to child coordinate space [0,1]³
- **AND** continue until hitting solid voxel or exiting bounds

#### Scenario: Maximum depth limit
- **WHEN** traversal reaches depth 0 (root level)
- **THEN** treat as solid leaf if depth limit prevents further subdivision
- **AND** return hit information if voxel is non-empty

### Requirement: Coordinate Space Transformations
The raycast system SHALL correctly transform between world space, normalized cube space [0,1]³, and octree coordinate space throughout traversal.

#### Scenario: World to normalized space
- **WHEN** ray enters cube bounding box in world coordinates
- **THEN** transform hit point to normalized [0,1]³ coordinate space
- **AND** maintain ray direction vector unchanged

#### Scenario: Octant subdivision transform
- **WHEN** entering a child octant during traversal
- **THEN** transform position from parent space to child space using `(pos * 2.0 - octant_offset)`
- **AND** update octree coordinate path by shifting and adding octant bits

#### Scenario: Hit point world space conversion
- **WHEN** raycast finds a voxel hit in normalized space
- **THEN** transform hit position back to world coordinates
- **AND** preserve normal vector orientation

### Requirement: Surface Normal Calculation
The raycast system SHALL calculate accurate surface normals based on the entry face of the hit voxel.

#### Scenario: Entry from minimum face
- **WHEN** ray enters voxel from face at coordinate 0
- **THEN** return normal pointing in negative axis direction (-1, 0, 0), (0, -1, 0), or (0, 0, -1)
- **AND** determine axis based on minimum distance to face

#### Scenario: Entry from maximum face
- **WHEN** ray enters voxel from face at coordinate 1
- **THEN** return normal pointing in positive axis direction (1, 0, 0), (0, 1, 0), or (0, 0, 1)
- **AND** determine axis based on minimum distance to face

#### Scenario: Corner and edge entry
- **WHEN** ray enters voxel at corner or edge
- **THEN** select single dominant normal based on closest face
- **AND** use epsilon comparison to handle floating-point precision

### Requirement: DDA Stepping Algorithm
The raycast system SHALL use DDA stepping to efficiently skip empty octants and move to the next octant boundary.

#### Scenario: Next octant boundary calculation
- **WHEN** current octant is empty and traversal must continue
- **THEN** calculate next integer boundary along ray direction
- **AND** step to next octant position using minimal time increment
- **AND** clamp result to valid [0,1]³ range

#### Scenario: Axis-aligned ray handling
- **WHEN** ray direction has zero component on one or more axes
- **THEN** handle division by zero gracefully using epsilon checks
- **AND** avoid infinity/NaN in DDA calculations
- **AND** step only along non-zero axes

#### Scenario: Grazing ray precision
- **WHEN** ray is nearly tangent to octant boundary
- **THEN** use epsilon comparisons for boundary tests
- **AND** clamp positions to prevent floating-point drift
- **AND** ensure consistent traversal across precision boundaries

### Requirement: Octant Indexing
The raycast system SHALL correctly determine octant indices (0-7) based on position and ray direction.

#### Scenario: Octant selection from position
- **WHEN** determining which child octant contains a position
- **THEN** calculate octant bits using `(x_bit << 2) | (y_bit << 1) | z_bit`
- **AND** use floor of (position * 2) adjusted by ray direction sign
- **AND** validate octant index is in valid range [0, 7]

#### Scenario: Directional octant adjustment
- **WHEN** ray direction is negative on an axis
- **THEN** adjust octant calculation to account for direction sign
- **AND** ensure correct child selection for backward-traveling rays

### Requirement: Hit Information Return
The raycast system SHALL return complete hit information including voxel coordinate, position, normal, distance, and voxel value.

#### Scenario: Successful raycast hit
- **WHEN** raycast finds a solid voxel
- **THEN** return RaycastHit with:
  - `hit: true`
  - `t: f32` - distance along ray from origin
  - `point: Vec3` - hit point in world coordinates
  - `normal: Vec3` - surface normal unit vector
  - `voxel_pos: IVec3` - voxel integer position
  - `voxel_value: i32` - voxel ID or color value

#### Scenario: Raycast miss
- **WHEN** raycast exits octree bounds without hitting solid voxel
- **THEN** return RaycastHit with `hit: false`
- **AND** other fields may be undefined or default values

#### Scenario: Voxel value extraction
- **WHEN** hit occurs on a Cube::Solid(value) leaf node
- **THEN** extract and return the voxel value in RaycastHit
- **AND** support both scalar values and palette indices

### Requirement: Bounding Box Integration
The renderer SHALL integrate bounding box intersection with octree raycast for complete ray-object intersection.

#### Scenario: Ray-box intersection test
- **WHEN** casting ray at octree object
- **THEN** first test ray against axis-aligned bounding box
- **AND** if miss, return background color without octree traversal
- **AND** if hit, use entry point as starting position for octree raycast

#### Scenario: Early exit on bounding box miss
- **WHEN** ray does not intersect object bounding box
- **THEN** skip octree traversal entirely
- **AND** return immediately with background color
- **AND** avoid unnecessary computation

### Requirement: Lighting Calculation
The renderer SHALL calculate lighting for raycast hits using normals and light direction.

#### Scenario: Diffuse lighting
- **WHEN** raycast hits a voxel surface
- **THEN** calculate diffuse lighting using normal dot light direction
- **AND** clamp negative values to zero (no negative light)
- **AND** modulate voxel color by light intensity

#### Scenario: Ambient occlusion (future)
- **WHEN** rendering with ambient occlusion enabled
- **THEN** cast additional short rays to detect nearby geometry
- **AND** darken surface based on occlusion factor
- **AND** blend with diffuse lighting

#### Scenario: Shadow rays (future)
- **WHEN** shadow casting is enabled
- **THEN** cast ray from hit point toward light source
- **AND** check for blocking geometry
- **AND** apply shadow multiplier if occluded

### Requirement: Robustness and Edge Cases
The raycast system SHALL handle edge cases and floating-point precision issues robustly.

#### Scenario: Division by zero protection
- **WHEN** ray direction component is zero or near-zero
- **THEN** check against epsilon (1e-8) before division
- **AND** avoid infinity and NaN in calculations
- **AND** fall back to safe default behavior

#### Scenario: Floating-point precision handling
- **WHEN** accumulated floating-point errors occur during traversal
- **THEN** renormalize positions when entering children
- **AND** clamp to [0,1]³ after each step
- **AND** use epsilon comparisons for boundary tests

#### Scenario: Maximum iteration protection
- **WHEN** traversal might loop indefinitely due to precision issues
- **THEN** implement iteration counter with reasonable maximum (e.g., 1000)
- **AND** exit traversal if counter exceeded
- **AND** log warning for debugging

#### Scenario: Depth 0 root handling
- **WHEN** raycast is at depth 0 (root cube)
- **THEN** do not attempt subdivision
- **AND** treat as solid or empty based on cube type
- **AND** prevent recursion errors

### Requirement: Performance Optimization
The raycast implementation SHALL optimize for performance without sacrificing correctness.

#### Scenario: Early termination on hit
- **WHEN** first solid voxel is found
- **THEN** immediately return hit information
- **AND** do not traverse remaining octree
- **AND** minimize total traversal time

#### Scenario: Empty space skipping
- **WHEN** traversing through empty octants
- **THEN** use DDA stepping to skip directly to next boundary
- **AND** avoid recursive calls into empty regions
- **AND** minimize traversal operations

#### Scenario: Coordinate clamping
- **WHEN** position exceeds [0,1]³ bounds
- **THEN** clamp to valid range to enable boundary checks
- **AND** prevent out-of-bounds array access
- **AND** maintain traversal correctness

### Requirement: Integration Testing
The raycast system SHALL include comprehensive integration tests verifying the complete render pipeline.

#### Scenario: Full pipeline test
- **WHEN** rendering a complete scene
- **THEN** verify bounding box intersection → octree raycast → lighting sequence
- **AND** compare output against expected visual results
- **AND** validate for various octree structures

#### Scenario: CubeScript integration
- **WHEN** loading octree from CubeScript format
- **THEN** parse script into octree structure
- **AND** successfully raycast through parsed octree
- **AND** produce correct rendering

#### Scenario: Performance benchmark
- **WHEN** testing large octrees (depth 5-6)
- **THEN** measure raycast performance (rays per second)
- **AND** verify 60 FPS capability for full-screen rendering
- **AND** profile and identify hotspots

