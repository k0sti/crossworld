# Spec: Renderer Raycast Integration

## MODIFIED Requirements

### Requirement: Voxel Value in RaycastHit
The cube raycast SHALL include the voxel value in the hit result to enable material systems and avoid tree traversal.

#### Scenario: RaycastHit includes voxel value
**Given** a raycast that hits a voxel with value `v`
**When** the raycast returns a hit
**Then** the `RaycastHit.value` field contains `v`
**And** no additional tree traversal is needed

#### Scenario: Generic voxel type support
**Given** a `Cube<T>` with voxel type `T`
**When** raycast is called
**Then** the result is `Option<RaycastHit<T>>`
**And** the generic type `T` is preserved throughout

#### Scenario: Backward compatibility with tests
**Given** existing 30 raycast tests
**When** `RaycastHit` is made generic with `value` field
**Then** all 30 tests continue to pass
**And** tests use `RaycastHit<i32>` explicitly

## ADDED Requirements

### Requirement: Octree Raycast Integration
The renderer SHALL integrate the cube crate's octree raycast implementation to enable rendering of subdivided voxel structures.

#### Scenario: Solid cube rendering still works
**Given** a `Cube::Solid(value)` with non-zero value
**When** raycast is called with a ray intersecting the cube
**Then** a hit is returned with correct position and normal
**And** the voxel value matches the solid value

#### Scenario: Subdivided octree rendering works
**Given** a `Cube::Cubes(children)` with at least one solid child
**When** raycast is called with a ray intersecting a solid octant
**Then** a hit is returned at the correct octant position
**And** the normal corresponds to the entry face
**And** the voxel coordinate identifies the hit octant

#### Scenario: Empty voxel is treated as miss
**Given** a voxel with value == 0 (empty)
**When** raycast is called
**Then** no hit is returned (raycast continues through empty space)

#### Scenario: Deep octree traversal
**Given** a `Cube::Cubes` subdivided to depth 3+
**When** raycast is called with sufficient max_depth
**Then** the deepest solid voxel is found
**And** the hit position is accurate to the voxel size

### Requirement: Coordinate Transformation
The raycast integration SHALL correctly transform coordinates between world space and normalized cube space.

#### Scenario: World to normalized transformation
**Given** a hit point in world space from bounding box intersection
**When** transforming to normalized [0,1]³ space
**Then** the position is correctly mapped relative to cube bounds
**And** positions outside [0,1]³ are handled gracefully (miss)

#### Scenario: Normalized to world transformation
**Given** a hit position in normalized [0,1]³ space from cube raycast
**When** transforming back to world space
**Then** the position correctly maps to the world coordinate system
**And** the transformation is the inverse of world-to-normalized

#### Scenario: Direction vector handling
**Given** a ray direction in world space
**When** passed to cube raycast
**Then** the direction is normalized
**And** the direction remains unchanged (directions are space-agnostic)

### Requirement: Hit Information Conversion
The raycast SHALL convert between cube and renderer RaycastHit formats accurately.

#### Scenario: Convert successful hit to HitInfo
**Given** a `Some(cube::raycast::RaycastHit<i32>)` from cube raycast
**When** converting to `HitInfo` for lighting
**Then** `hit` field is `true`
**And** `point` is transformed from normalized [0,1]³ to world space
**And** `normal` is copied directly
**And** `t` is calculated as distance from ray origin to hit point

#### Scenario: Convert miss to background color
**Given** a `None` result from cube raycast
**When** rendering the ray
**Then** background color is returned
**And** no lighting calculation is performed

#### Scenario: Voxel value available directly
**Given** a hit at a specific octree coordinate
**When** `RaycastHit<i32>` is returned
**Then** `value` field contains the voxel data (i32)
**And** no tree traversal is needed to extract it
**And** the value can be used for future material/color systems

### Requirement: Empty Voxel Filtering
The raycast SHALL use an `is_empty` predicate to filter out empty voxels (value == 0).

#### Scenario: Empty voxel predicate
**Given** a voxel with value `v`
**When** the `is_empty` predicate is evaluated
**Then** it returns `true` if `v == 0`
**And** it returns `false` if `v != 0`

#### Scenario: Ray passes through empty voxels
**Given** an octree with both empty (0) and solid (non-zero) voxels
**When** a ray passes through empty voxels before hitting solid
**Then** empty voxels are skipped (treated as transparent)
**And** the first solid voxel is returned as the hit

### Requirement: Maximum Depth Configuration
The raycast SHALL specify a maximum traversal depth to limit recursion.

#### Scenario: Default max depth
**Given** no explicit max depth configuration
**When** calling cube raycast
**Then** a reasonable default depth is used (e.g., 8-10)
**And** deeper subdivisions are treated as solid boundaries

#### Scenario: Depth limiting prevents infinite recursion
**Given** a malformed or extremely deep octree
**When** raycast is called with max_depth limit
**Then** traversal stops at the depth limit
**And** no stack overflow or infinite loop occurs

### Requirement: Backward Compatibility
The raycast integration SHALL maintain compatibility with existing renderer usage.

#### Scenario: CPU tracer integration unchanged
**Given** the existing `cpu_tracer.rs` usage of `gpu_tracer::raycast()`
**When** the new implementation is integrated
**Then** no changes to `cpu_tracer.rs` are required
**And** the function signature remains identical

#### Scenario: Existing solid cube tests pass
**Given** any existing tests rendering solid cubes
**When** the new raycast is integrated
**Then** all existing tests continue to pass
**And** visual output for solid cubes is unchanged
