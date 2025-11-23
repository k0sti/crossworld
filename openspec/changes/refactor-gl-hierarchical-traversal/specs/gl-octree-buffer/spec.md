# Spec: GL Octree Buffer Rendering

## MODIFIED Requirements

### Requirement: GL tracer MUST use hierarchical octree traversal

The GL tracer SHALL traverse the octree structure hierarchically on the GPU instead of sampling a pre-flattened 3D texture.

#### Scenario: Render simple octree with hierarchical traversal
**Given** a `Cube::Cubes` with 8 solid children of different materials
**When** the GL tracer renders the octree
**Then** the output SHALL show distinct colors for each octant
**And** the rendering SHALL result from hierarchical node traversal, not texture sampling

#### Scenario: Skip empty octants efficiently
**Given** a sparse octree with only 2 of 8 octants populated
**When** the GL tracer traverses the octree
**Then** empty octants SHALL be skipped without full recursive descent
**And** DDA stepping SHALL move directly to the next non-empty region

#### Scenario: Support arbitrary octree depths
**Given** an octree of depth 6 (64³ resolution)
**When** the GL tracer renders the octree
**Then** the output SHALL correctly represent all leaf voxels
**And** depth SHALL NOT be limited by texture resolution

### Requirement: GL tracer MUST NOT use 3D textures

The GL tracer SHALL NOT use `TEXTURE_3D`, `sampler3D`, or any 3D texture operations for octree data storage or lookup.

#### Scenario: Shader compilation without 3D texture declarations
**Given** the fragment shader source code
**When** the shader is compiled
**Then** it SHALL NOT contain `sampler3D` uniform declarations
**And** it SHALL NOT contain `texture()` calls with 3D samplers
**And** it SHALL NOT contain `TEXTURE_3D` binding operations in Rust code

#### Scenario: GPU memory usage without 3D textures
**Given** the GL tracer initialized with an octree
**When** GPU resources are allocated
**Then** no `TEXTURE_3D` resources SHALL be created
**And** only `TEXTURE_BUFFER` or SSBO resources SHALL exist for octree data

### Requirement: Octree data MUST be serialized to linear GPU buffer

The octree SHALL be serialized to a linearized buffer format suitable for GPU upload via texture buffer or SSBO.

#### Scenario: Serialize simple octree to buffer
**Given** a `Cube::Solid(42)` octree
**When** `serialize_octree_to_buffer()` is called
**Then** the buffer SHALL contain:
- Header: `[0x00000000]` (root at index 0)
- Node 0: `[0x0000002A, 0x00000000]` (leaf, material 42)
**And** the buffer size SHALL be 12 bytes (3 uint32 values)

#### Scenario: Serialize parent node with 8 children
**Given** a `Cube::Cubes` with 8 `Solid` children (materials 1-8)
**When** `serialize_octree_to_buffer()` is called
**Then** the buffer SHALL contain:
- Header: `[0x00000000]` (root at index 0)
- Node 0: `[0x80000001, 0x00000000]` (parent, children at index 1)
- Nodes 1-8: Eight leaf nodes with materials 1-8
**And** child nodes SHALL be stored consecutively after the parent

#### Scenario: Node structure with correct bit packing
**Given** a parent node with child_base_index = 100
**When** the node is serialized
**Then** word 0 SHALL equal `0x80000064` (bit 31 set, lower 31 bits = 100)
**And** word 1 SHALL equal `0x00000000` (reserved)

### Requirement: Fragment shader MUST implement DDA octree traversal

The fragment shader SHALL implement Digital Differential Analyzer (DDA) algorithm for hierarchical octree traversal.

#### Scenario: Calculate octant from position
**Given** position `vec3(0.75, 0.25, 0.75)` in [0,1]³ space
**When** `calculateOctant(pos)` is called
**Then** the result SHALL be octant 5 (binary 101: +x, -y, +z)

#### Scenario: Transform position to child coordinate space
**Given** position `vec3(0.75, 0.25, 0.75)` and octant 5
**When** `transformToChild(pos, 5)` is called
**Then** the result SHALL be `vec3(0.5, 0.5, 0.5)` (center of child cube)

#### Scenario: DDA step to next octant boundary
**Given** ray at position `vec3(0.9, 0.5, 0.5)` with direction `vec3(1, 0, 0)`
**When** `ddaStep(pos, dir, 0)` is called (depth 0 = full cube)
**Then** the next position SHALL be approximately `vec3(1.0, 0.5, 0.5)` (cube boundary)

#### Scenario: Hierarchical descent through parent nodes
**Given** a depth-2 octree with root → octant 5 → octant 3 → leaf(material=7)
**When** the shader traverses from root at position `vec3(0.8, 0.8, 0.8)`
**Then** traversal SHALL:
1. Start at root (node 0), identify as parent
2. Calculate octant 7 (binary 111: +x, +y, +z)
3. Read child node, identify as parent
4. Transform position to child space
5. Calculate sub-octant, descend to leaf
6. Return material 7
**And** traversal SHALL terminate upon hitting the leaf node

### Requirement: GL tracer output MUST match CPU tracer output

The GL tracer SHALL produce pixel-perfect or near-identical output compared to the CPU tracer for the same octree and camera configuration.

#### Scenario: Pixel-perfect match for simple solid cube
**Given** a `Cube::Solid(1)` octree
**When** both CPU and GL tracers render with identical camera
**Then** `diff_cpu_gl.png` SHALL show zero pixel differences
**Or** differences SHALL be ≤0.1% of pixels (rounding/precision only)

#### Scenario: Visual equivalence for complex octree
**Given** a depth-3 octree with mixed solid/empty octants
**When** both tracers render the same scene
**Then** visual inspection SHALL show equivalent geometry
**And** pixel difference count SHALL be <1% of total pixels
**And** max pixel color difference SHALL be ≤5 (out of 255)

#### Scenario: Consistent normals and lighting
**Given** any valid octree
**When** both tracers calculate surface normals
**Then** normal vectors SHALL match within 0.01 tolerance
**And** diffuse lighting SHALL produce equivalent brightness
**And** specular highlights SHALL appear in the same locations

## REMOVED Requirements

### ~~Requirement: GL tracer SHALL use 3D texture for octree data~~
**Reason**: Replaced with hierarchical buffer traversal for architectural correctness

### ~~Requirement: Octree SHALL be sampled into fixed-resolution grid~~
**Reason**: Hierarchical structure must be preserved, not flattened

### ~~Requirement: `sample_cube_at_position()` SHALL convert octree to voxel grid~~
**Reason**: Function removed; serialization replaces sampling

## Implementation Notes

### WebGL 2.0 Compatibility
- Use `uniform usamplerBuffer` for buffer textures (WebGL 2.0 feature)
- Texture buffers support up to `GL_MAX_TEXTURE_BUFFER_SIZE` (typically 128MB)
- Fall back to smaller depth limits if buffer size exceeds device limits

### Buffer Format Reference
See `design.md` for detailed buffer layout specification:
- Node structure: 2 × uint32 (8 bytes)
- Bit 31 of word 0: is_parent flag
- Bits 0-30 of word 0: material (leaf) or child_base_index (parent)
- Word 1: Reserved for future optimization hints

### Traversal Algorithm Reference
See CPU implementation in `crates/cube/src/raycast/mod.rs` for reference DDA algorithm. Fragment shader SHALL implement equivalent logic adapted for GPU constraints (no recursion, iteration limit).

### Performance Expectations
- Initial implementation prioritizes correctness over performance
- Performance regression vs 3D texture approach is acceptable
- Future optimization (cache-friendly layout, SIMD hints) in separate change

## Validation Criteria

1. **Code Review**: No `TEXTURE_3D`, `sampler3D`, or `tex_image_3d` references
2. **Compilation**: Shader and Rust code compile without errors
3. **Rendering**: GL tracer produces visible output without crashes
4. **Correctness**: Diff against CPU tracer shows <1% pixel difference
5. **Tests**: All renderer tests pass with new implementation

## Related Specifications

- BCF Format: `doc/architecture/bcf-format.md` (buffer inspiration)
- Raycast Algorithm: `crates/cube/src/raycast/mod.rs` (CPU reference)
- GPU Compute Tracer: `crates/renderer/src/gpu_tracer.rs` (separate implementation)
