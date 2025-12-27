## ADDED Requirements

### Requirement: Fabric Quaternion Octree
The system SHALL provide a fabric module at `crates/cube/src/fabric/` that generates `Cube<Quat>` structures using quaternion field interpolation with position-encoding.

#### Scenario: Generate fabric cube with default parameters (SDF convention)
- **GIVEN** a FabricGenerator with root_magnitude=0.5, boundary_magnitude=2.0, and additive states
- **WHEN** `generate_cube(depth: 3)` is called
- **THEN** a `Cube<Quat>` is returned with depth 3 octree structure
- **AND** root quaternion has magnitude 0.5 (|Q| < 1 = inside/solid)
- **AND** boundary quaternions approach magnitude 2.0 (|Q| > 1 = outside/air)

#### Scenario: Octant index determines rotation offset
- **GIVEN** a parent node with quaternion Q_parent
- **WHEN** calculating child quaternion for octant index i (0-7)
- **THEN** a positional rotation is applied based on octant bits:
  - Bit 0 (X-axis): +90° if set, -90° if unset
  - Bit 1 (Y-axis): +90° if set, -90° if unset
  - Bit 2 (Z-axis): +90° if set, -90° if unset
- **AND** the rotation encodes the child's spatial position relative to parent

#### Scenario: Child quaternion calculation pipeline
- **GIVEN** a parent quaternion Q_parent and octant index i
- **WHEN** calculating Q_child
- **THEN** the calculation follows: Q_positioned = Q_parent * octant_rotation[i]
- **AND** Q_blended = LERP(Q_parent, Q_positioned, blend_factor)
- **AND** Q_child = Q_blended * Q_additive[depth]

### Requirement: Dual-Purpose Quaternion (Rotation + Magnitude)
The fabric module SHALL use non-normalized quaternions that encode both world position (via rotation) and field density (via magnitude).

#### Scenario: Magnitude computed from Euclidean distance
- **GIVEN** root_magnitude = 0.5, boundary_magnitude = 2.0, surface_radius = 0.8
- **AND** a voxel at world position with distance 0.4 from origin
- **WHEN** computing the voxel's quaternion magnitude
- **THEN** t = 0.4 / 0.8 = 0.5
- **AND** magnitude = lerp(0.5, 2.0, 0.5) = 1.25 (outside, since > 1.0)

#### Scenario: Rotation component encodes position
- **GIVEN** two voxels at different world positions
- **WHEN** examining their quaternion rotation components (Q.normalize())
- **THEN** the rotations differ based on their octant paths from root
- **AND** the rotation difference reflects their spatial relationship

#### Scenario: Magnitude determines surface (SDF convention)
- **GIVEN** quaternion magnitude threshold at 1.0 (following SDF sign convention)
- **WHEN** |Q| < 1.0
- **THEN** the voxel is inside (solid, negative SDF equivalent)
- **WHEN** |Q| > 1.0
- **THEN** the voxel is outside (air, positive SDF equivalent)
- **AND** surface exists at |Q| = 1.0 boundary

#### Scenario: Spherical surface from distance-based magnitude
- **GIVEN** root_magnitude = 0.5, boundary_magnitude = 2.0, surface_radius = 0.8
- **WHEN** generating a fabric cube
- **THEN** surface (|Q| = 1.0) forms at distance ≈ 0.53 from origin
- **AND** the surface is spherical (equidistant from origin in all directions)

### Requirement: Additive Quaternion State
The fabric generator SHALL apply additive states per depth level that affect both rotation and magnitude to introduce variation and detail.

#### Scenario: Additive states control rotation variation
- **GIVEN** additive_states with rotation values `[0.0, 0.1, 0.2]`
- **WHEN** generating quaternion at depth 2
- **THEN** the additive rotation magnitude is approximately 0.2 radians (scaled by noise)

#### Scenario: Additive states control magnitude variation
- **GIVEN** additive_states with magnitude values `[0.0, 0.05, 0.1]`
- **WHEN** generating quaternion at depth 2
- **THEN** the quaternion magnitude varies by approximately ±0.1 from base interpolated value

#### Scenario: Deeper depths get more variation
- **GIVEN** additive_states with increasing values per depth
- **WHEN** comparing quaternion variation at depth 1 vs depth 3
- **THEN** depth 3 nodes show more angular and magnitude variation than depth 1

### Requirement: Cube Value Retrieval
The `Cube<T>` type SHALL provide a `value()` method that returns the stored value for leaf nodes and a default/computed value for branch nodes.

#### Scenario: Solid cube returns its value
- **GIVEN** a `Cube::Solid(Q)` where Q is a quaternion
- **WHEN** `value()` is called
- **THEN** `Some(&Q)` is returned

#### Scenario: Cubes variant returns None
- **GIVEN** a `Cube::Cubes([...])` with 8 children
- **WHEN** `value()` is called
- **THEN** `None` is returned (branch nodes have no single value)

### Requirement: Surface Detection via Magnitude Threshold
The fabric system SHALL detect surfaces where quaternion magnitude crosses 1.0, with |Q| > 1 representing air/outside and |Q| < 1 representing solid/inside.

#### Scenario: Surface at magnitude crossing from inside to outside (SDF convention)
- **GIVEN** current voxel quaternion with |Q| = 0.8 (inside, |Q| < 1)
- **AND** neighbor voxel quaternion with |Q| = 1.2 (outside, |Q| > 1)
- **WHEN** `is_surface(current, neighbor)` is called
- **THEN** `true` is returned (surface exists at |Q| = 1.0 boundary)

#### Scenario: No surface when both outside
- **GIVEN** current voxel quaternion with |Q| = 1.5 (outside)
- **AND** neighbor voxel quaternion with |Q| = 1.1 (outside)
- **WHEN** `is_surface(current, neighbor)` is called
- **THEN** `false` is returned (both outside, no surface)

#### Scenario: No surface when both inside
- **GIVEN** current voxel quaternion with |Q| = 0.5 (inside)
- **AND** neighbor voxel quaternion with |Q| = 0.9 (inside)
- **WHEN** `is_surface(current, neighbor)` is called
- **THEN** `false` is returned (both inside solid, no surface)

### Requirement: Normal Calculation from Magnitude Gradient
The fabric system SHALL calculate surface normals from the gradient of the quaternion magnitude field.

#### Scenario: Normal points toward outside region (SDF convention)
- **GIVEN** a surface point where |Q| crosses 1.0
- **WHEN** `calculate_normal(position, fabric_cube, depth)` is called
- **THEN** the returned normal vector points toward the region with |Q| > 1 (outside/air)
- **AND** this follows standard SDF convention where normals point outward from solid

#### Scenario: Gradient uses central differences
- **GIVEN** position P in the quaternion field
- **WHEN** calculating the gradient
- **THEN** the gradient is computed as `(|Q(P+h)| - |Q(P-h)|) / (2*h)` for each axis
- **AND** h equals the voxel half-size at the current depth

### Requirement: Color from Quaternion Rotation
The fabric system SHALL derive voxel colors from quaternion rotation properties using HSV color space mapping.

#### Scenario: Identity quaternion produces neutral color
- **GIVEN** quaternion Q = Quat::IDENTITY (no rotation)
- **WHEN** `quaternion_to_color(Q)` is called
- **THEN** a neutral/gray color is returned (saturation near 0)

#### Scenario: Rotation axis affects hue
- **GIVEN** two quaternions with same angle but different rotation axes
- **WHEN** converting both to colors
- **THEN** they produce different hue values
- **AND** rotation around +X vs -X produces hues on opposite sides of the color wheel

### Requirement: Origin-Centered Spherical Field Generation
The fabric generator SHALL support origin-centered spherical model generation where field magnitude is computed from Euclidean distance, producing spherical surfaces.

#### Scenario: Root magnitude defines center as solid (SDF convention)
- **GIVEN** a FabricGenerator with root_magnitude = 0.5, boundary_magnitude = 2.0, surface_radius = 0.8
- **WHEN** generating the root quaternion (at distance 0)
- **THEN** the root quaternion has magnitude 0.5 (|Q| < 1 = inside/solid at origin)

#### Scenario: Distance-based magnitude creates spherical surface
- **GIVEN** root_magnitude = 0.5, boundary_magnitude = 2.0, surface_radius = 0.8
- **WHEN** generating voxels at various positions
- **THEN** magnitude = lerp(root_magnitude, boundary_magnitude, distance / surface_radius)
- **AND** voxels equidistant from origin have equal magnitude
- **AND** the surface (|Q| = 1.0) forms a sphere

#### Scenario: Surface radius controls sphere size
- **GIVEN** root_magnitude = 0.5, boundary_magnitude = 2.0
- **WHEN** surface_radius = 0.5
- **THEN** the sphere surface forms at distance ≈ 0.33 from origin (smaller sphere)
- **WHEN** surface_radius = 1.0
- **THEN** the sphere surface forms at distance ≈ 0.67 from origin (larger sphere)

#### Scenario: Additive magnitude creates surface variation
- **GIVEN** additive_states with magnitude variation `[0.0, 0.0, 0.1, 0.2]`
- **WHEN** generating at depth 3+
- **THEN** the surface position varies based on noise-scaled magnitude adjustments
- **AND** this creates organic surface detail on the sphere
