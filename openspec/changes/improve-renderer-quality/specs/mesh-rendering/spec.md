# mesh-rendering Specification Delta

## ADDED Requirements

### Requirement: Camera Synchronization
The mesh renderer SHALL use identical camera configuration to raytracing renderers for consistent viewpoint comparison.

#### Scenario: Camera matrix consistency
- **WHEN** mesh renderer renders a frame with a given `CameraConfig`
- **THEN** the view matrix is computed using `CameraConfig::view_matrix()`
- **AND** the projection matrix is computed using `CameraConfig::projection_matrix(aspect)`
- **AND** the model-view-projection (MVP) matrix passed to shaders matches raytracer calculations
- **AND** rendered output shows the same viewpoint as CPU/GL/GPU raytracers

#### Scenario: Camera sync validation test
- **WHEN** a test compares camera matrices across all five renderers
- **THEN** view matrices are identical for all renderers
- **AND** projection matrices are identical (for same aspect ratio)
- **AND** mesh renderer output viewpoint matches raytracer output (subject to rasterization vs raytracing differences)

### Requirement: Correct Face Culling
The mesh renderer SHALL render voxel faces with correct orientation and culling configuration.

#### Scenario: Front face winding order
- **WHEN** mesh is uploaded from `generate_face_mesh`
- **THEN** triangle vertices follow counter-clockwise (CCW) winding for front faces
- **AND** OpenGL front face is configured as `GL_CCW`
- **AND** backface culling is enabled with `GL_CULL_FACE` and `GL_BACK`

#### Scenario: Normal orientation
- **WHEN** mesh normals are uploaded
- **THEN** normals point outward from voxel surfaces
- **AND** normals match triangle winding order (right-hand rule)
- **AND** lighting calculations use outward-facing normals

#### Scenario: Visual validation
- **WHEN** mesh renderer displays a voxel cube
- **THEN** all visible faces render correctly without holes
- **AND** no inside-out faces are visible
- **AND** culled backfaces do not appear

### Requirement: Mesh Caching
The mesh renderer SHALL support caching uploaded meshes to avoid unnecessary regeneration and GPU uploads.

#### Scenario: Cache enable/disable
- **WHEN** user enables mesh caching option in UI
- **THEN** mesh is uploaded once and reused across frames
- **AND** mesh is not re-uploaded while cache is valid
- **AND** user can disable caching to force regeneration every frame

#### Scenario: Cache invalidation on model change
- **WHEN** user changes the voxel model selection
- **THEN** mesh cache is marked as invalid
- **AND** next render will regenerate and upload mesh
- **AND** subsequent renders reuse the new cached mesh

#### Scenario: Cache invalidation on material change
- **WHEN** user changes single voxel material (for SingleRedVoxel model)
- **THEN** mesh cache is marked as invalid
- **AND** mesh is regenerated with new material colors
- **AND** cache is revalidated after upload

#### Scenario: Manual regeneration
- **WHEN** user clicks "Regenerate Mesh" button
- **THEN** mesh cache is immediately invalidated
- **AND** mesh is regenerated on next render regardless of cache setting
- **AND** cache is revalidated if caching is enabled

#### Scenario: Cache does not invalidate on camera change
- **WHEN** user moves camera (rotation, zoom, position)
- **THEN** mesh cache remains valid
- **AND** mesh is not regenerated
- **AND** only view/projection matrices are updated

#### Scenario: Cache status display
- **WHEN** mesh caching UI is visible
- **THEN** UI shows current cache status ("Cached", "Needs Regen", "Uncached")
- **AND** UI shows mesh statistics (vertex count, face count)
- **AND** UI shows last upload time in milliseconds

### Requirement: Mesh Statistics
The mesh renderer SHALL track and display mesh generation statistics for performance analysis.

#### Scenario: Vertex and face counting
- **WHEN** mesh is uploaded from octree
- **THEN** total vertex count is tracked
- **AND** total face (triangle) count is tracked
- **AND** counts are displayed in UI

#### Scenario: Upload timing
- **WHEN** mesh is uploaded to GPU
- **THEN** upload duration is measured in milliseconds
- **AND** upload time is displayed in UI
- **AND** upload time updates only when mesh is regenerated (not on cache hits)

#### Scenario: Render timing separation
- **WHEN** mesh renderer renders a frame
- **THEN** mesh upload time is tracked separately from render time
- **AND** render time excludes mesh generation/upload when cache is hit
- **AND** both times are displayed for performance comparison
