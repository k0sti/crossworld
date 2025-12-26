## Status (2025-11-22)

⚠️ **BROKEN**: GPU raytracer is currently non-functional. See `standardize-material-system` change for fix plan.

---

## ADDED Requirements

### Requirement: GPU Shader Octree Traversal
The GPU raytracer SHALL implement octree traversal using a shader-based DDA algorithm that matches the behavior of the CPU raytracer's `cube::Cube::raycast()` implementation.

#### Scenario: Ray hits voxel in simple cube
- **WHEN** a ray intersects a solid voxel cube
- **THEN** the shader returns hit=true with correct position, normal, and voxel value
- **AND** the result matches CPU raytracer output exactly

#### Scenario: Ray traverses subdivided octree
- **WHEN** a ray traverses a subdivided octree with multiple depth levels
- **THEN** the shader correctly steps through octants using DDA
- **AND** stops at the first non-empty voxel encountered
- **AND** returns the correct hit position and normal

#### Scenario: Ray passes through empty space
- **WHEN** a ray encounters voxels with value 0 (empty)
- **THEN** the shader continues traversal without stopping
- **AND** only reports hits on non-empty voxels (value != 0)

#### Scenario: Ray misses entire octree
- **WHEN** a ray does not intersect the octree bounding box
- **THEN** the shader returns hit=false
- **AND** no voxel lookup is performed

### Requirement: Coordinate Space Transformations
The GPU raytracer SHALL transform between world space and normalized [0,1]³ octree space, matching the CPU implementation.

#### Scenario: World space to octree space
- **WHEN** a world space ray is provided to the shader
- **THEN** the shader transforms ray origin and direction to normalized [0,1]³ space
- **AND** performs octree traversal in normalized space

#### Scenario: Hit position in world space
- **WHEN** a hit is detected in normalized space
- **THEN** the shader transforms the hit position back to world space
- **AND** the world space position matches the CPU raytracer output

### Requirement: Surface Normal Calculation
The GPU raytracer SHALL calculate surface normals based on which face of the voxel cube the ray enters.

#### Scenario: Ray enters from X-axis face
- **WHEN** a ray enters a voxel through the min-X or max-X face
- **THEN** the shader returns normal (-1,0,0) or (1,0,0) respectively

#### Scenario: Ray enters from Y-axis face
- **WHEN** a ray enters a voxel through the min-Y or max-Y face
- **THEN** the shader returns normal (0,-1,0) or (0,1,0) respectively

#### Scenario: Ray enters from Z-axis face
- **WHEN** a ray enters a voxel through the min-Z or max-Z face
- **THEN** the shader returns normal (0,0,-1) or (0,0,1) respectively

### Requirement: Octree Depth Limiting
The GPU raytracer SHALL support configurable maximum traversal depth to prevent infinite recursion and control detail level.

#### Scenario: Depth limit enforced
- **WHEN** traversal reaches the configured max depth
- **THEN** the shader treats the current octant as a solid voxel
- **AND** does not attempt to subdivide further

#### Scenario: Different depth limits produce different results
- **WHEN** rendering with max_depth=1 vs max_depth=3
- **THEN** higher depth shows more detail in subdivided regions
- **AND** lower depth shows coarser voxel representation

### Requirement: GPU-CPU Output Equivalence
The GPU raytracer SHALL produce pixel-perfect identical output to the CPU raytracer for the same scene configuration.

#### Scenario: Identical rendering output
- **WHEN** the same scene is rendered with both GPU and CPU tracers
- **THEN** a pixel-by-pixel diff shows zero differences
- **AND** all hit positions, normals, and colors match exactly

#### Scenario: Identical lighting calculations
- **WHEN** lighting is applied to raycast results
- **THEN** GPU and CPU implementations produce the same shaded colors
- **AND** shadow rays (if implemented) produce identical results

### Requirement: Octree Data Upload
The GPU raytracer SHALL efficiently upload octree data to the GPU using textures or buffers.

#### Scenario: Octree uploaded as texture
- **WHEN** an octree is prepared for GPU rendering
- **THEN** the octree structure is encoded into a GPU-compatible format
- **AND** uploaded to GPU memory as a 3D texture or buffer
- **AND** the shader can efficiently query voxel values

#### Scenario: Octree updates
- **WHEN** the octree is modified (voxels added/removed)
- **THEN** the GPU data is updated accordingly
- **AND** subsequent renders reflect the changes

### Requirement: Performance Optimization
The GPU raytracer SHALL achieve at least 10x performance improvement over the CPU raytracer for typical scenes.

#### Scenario: Performance benchmark
- **WHEN** rendering a 512x512 frame with octree depth 3
- **THEN** GPU raytracer completes in <16ms (60 FPS capable)
- **AND** CPU raytracer takes >160ms for the same frame
- **AND** frame time is measured and logged

#### Scenario: Resolution scaling
- **WHEN** rendering at higher resolutions (1024x1024, 2048x2048)
- **THEN** GPU raytracer maintains real-time performance (>30 FPS)
- **AND** performance scales better than linear with pixel count
