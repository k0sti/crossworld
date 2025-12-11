# BCF Raycast Traversal Specification

## ADDED Requirements

### Requirement: BCF Binary Reader
The system SHALL provide a zero-allocation binary reader for BCF format data with GPU-compatible operations.

#### Scenario: Read single byte at offset
- **WHEN** BcfReader reads byte at valid offset
- **THEN** SHALL return byte value
- **AND** SHALL perform bounds checking
- **AND** SHALL return BcfError if offset beyond data length

#### Scenario: Read multi-byte pointer (little-endian)
- **WHEN** BcfReader reads pointer with ssss=0 (1-byte)
- **THEN** SHALL read and return usize from 1 byte
- **WHEN** BcfReader reads pointer with ssss=1 (2-byte)
- **THEN** SHALL read little-endian u16 and return as usize
- **WHEN** BcfReader reads pointer with ssss=2 (4-byte)
- **THEN** SHALL read little-endian u32 and return as usize
- **WHEN** BcfReader reads pointer with ssss=3 (8-byte)
- **THEN** SHALL read little-endian u64 and return as usize

#### Scenario: Decode type byte using bit operations
- **WHEN** type byte is 0x00-0x7F (MSB=0)
- **THEN** SHALL decode as inline leaf with value = type_byte & 0x7F
- **WHEN** type byte is 0x80-0x8F (MSB=1, type=0)
- **THEN** SHALL decode as extended leaf requiring 1 additional byte
- **WHEN** type byte is 0x90-0x9F (MSB=1, type=1)
- **THEN** SHALL decode as octa-leaves requiring 8 additional bytes
- **WHEN** type byte is 0xA0-0xAF (MSB=1, type=2)
- **THEN** SHALL decode as octa-pointers with ssss = type_byte & 0x0F

#### Scenario: Read BCF header
- **WHEN** BcfReader reads header from valid BCF data
- **THEN** SHALL validate magic number (0x42434631)
- **AND** SHALL validate version (1)
- **AND** SHALL read root offset as u32 little-endian
- **AND** SHALL return BcfHeader struct

### Requirement: BCF Node Type Parsing
The system SHALL parse BCF nodes at arbitrary offsets and return structured node information.

#### Scenario: Parse inline leaf node
- **WHEN** reading node at offset where type byte <= 0x7F
- **THEN** SHALL return BcfNodeType::InlineLeaf(value)
- **AND** value SHALL equal type_byte & 0x7F
- **AND** SHALL consume 1 byte total

#### Scenario: Parse extended leaf node
- **WHEN** reading node at offset where type byte is 0x80-0x8F
- **THEN** SHALL read 1 additional value byte
- **AND** SHALL return BcfNodeType::ExtendedLeaf(value)
- **AND** SHALL consume 2 bytes total

#### Scenario: Parse octa-leaves node
- **WHEN** reading node at offset where type byte is 0x90-0x9F
- **THEN** SHALL read 8 value bytes
- **AND** SHALL return BcfNodeType::OctaLeaves([v0, v1, ..., v7])
- **AND** SHALL consume 9 bytes total

#### Scenario: Parse octa-pointers node
- **WHEN** reading node at offset where type byte is 0xA0-0xAF
- **THEN** SHALL extract ssss = type_byte & 0x0F
- **AND** SHALL read 8 pointers of size 2^ssss bytes each
- **AND** SHALL return BcfNodeType::OctaPointers { ssss, pointers: [p0, ..., p7] }
- **AND** SHALL consume 1 + (8 * 2^ssss) bytes total

### Requirement: Ray-AABB Intersection
The system SHALL compute ray-axis-aligned bounding box intersection using slab method.

#### Scenario: Ray intersects AABB
- **WHEN** ray intersects AABB from outside
- **THEN** SHALL return Some((t_near, t_far))
- **AND** t_near SHALL be entry point parameter (t >= 0)
- **AND** t_far SHALL be exit point parameter (t > t_near)

#### Scenario: Ray misses AABB
- **WHEN** ray does not intersect AABB
- **THEN** SHALL return None

#### Scenario: Ray origin inside AABB
- **WHEN** ray origin is inside AABB
- **THEN** SHALL return Some((0.0, t_far))
- **AND** t_far SHALL be exit point parameter

#### Scenario: Ray parallel to axis
- **WHEN** ray direction component is zero (parallel to axis)
- **THEN** SHALL handle edge case without division by zero
- **AND** SHALL check if ray is within slab bounds

### Requirement: Octant Selection and Addressing
The system SHALL map 3D positions to octant indices using bit operations compatible with GPU execution.

#### Scenario: Map position to octant index
- **WHEN** position is in octant with x>0, y>0, z>0
- **THEN** SHALL return octant index 7 = (1<<2 | 1<<1 | 1)
- **WHEN** position is in octant with x<0, y<0, z<0
- **THEN** SHALL return octant index 0 = (0<<2 | 0<<1 | 0)
- **WHEN** position is exactly on octant boundary (pos=0)
- **THEN** SHALL use ray direction sign to select octant

#### Scenario: Compute child AABB from parent and octant
- **WHEN** computing child bounds for octant
- **THEN** SHALL compute center = (parent.min + parent.max) / 2
- **AND** SHALL compute half_size = (parent.max - parent.min) / 2
- **AND** SHALL offset center by octant signs * half_size / 2
- **AND** SHALL return AABB with size half of parent

### Requirement: Iterative Octree Traversal
The system SHALL traverse BCF octree iteratively (no recursion) to find ray-voxel intersection.

#### Scenario: Traverse depth-1 octree (8 leaf nodes)
- **WHEN** ray intersects octree with 8 solid leaves
- **THEN** SHALL check root AABB intersection
- **AND** SHALL read root node (octa-leaves or octa-pointers)
- **AND** SHALL select octant based on ray entry point
- **AND** SHALL check leaf value (non-zero = solid)
- **AND** SHALL return BcfHit with value, normal, position

#### Scenario: Traverse depth-2 octree (recursive structure)
- **WHEN** ray intersects octree with nested children
- **THEN** SHALL maintain traversal stack
- **AND** SHALL push child nodes to stack as needed
- **AND** SHALL pop and process nodes until hit or stack empty
- **AND** SHALL track current bounds and depth

#### Scenario: Ray misses all solid voxels
- **WHEN** ray traverses octree but hits only empty voxels (value=0)
- **THEN** SHALL continue traversal to next node
- **AND** SHALL return None if all nodes exhausted

#### Scenario: Ray hits solid voxel
- **WHEN** ray intersects non-zero leaf value
- **THEN** SHALL return BcfHit with:
  - value: material index (u8)
  - normal: face normal (unit vector)
  - pos: intersection point (Vec3)
  - distance: ray parameter t

#### Scenario: Compute hit normal from entry face
- **WHEN** ray enters AABB through face
- **THEN** SHALL determine which axis (X, Y, or Z) was crossed
- **AND** SHALL determine direction (positive or negative)
- **AND** SHALL return unit normal vector (e.g., Vec3::X, -Vec3::Y)

### Requirement: BcfCpuTracer Rendering
The system SHALL render images using BCF traversal with same interface as existing CPU tracer.

#### Scenario: Render image from cube
- **WHEN** BcfCpuTracer renders image from Cube<u8>
- **THEN** SHALL serialize cube to BCF format
- **AND** SHALL trace ray for each pixel
- **AND** SHALL convert BcfHit to RGB color
- **AND** SHALL return ImageBuffer

#### Scenario: Apply lighting to hit
- **WHEN** ray hits solid voxel
- **THEN** SHALL look up material RGB from palette
- **AND** SHALL compute diffuse lighting from normal
- **AND** SHALL return final pixel color

#### Scenario: Handle ray miss (background)
- **WHEN** ray misses all geometry
- **THEN** SHALL return background color
- **AND** background color SHALL be consistent with existing CPU tracer

### Requirement: Validation Against Reference Implementation
The system SHALL produce identical output to existing CPU raytracer for same inputs.

#### Scenario: Pixel-perfect match for simple scenes
- **WHEN** rendering simple scene (single solid cube, octa-leaves)
- **THEN** BCF tracer output SHALL match CPU tracer output exactly
- **AND** every pixel RGB value SHALL be identical

#### Scenario: Complex scene visual equivalence
- **WHEN** rendering complex scene (depth-2+ octree)
- **THEN** BCF tracer output SHALL be visually equivalent to CPU tracer
- **AND** any differences SHALL be within 1 RGB value per channel (rounding)

#### Scenario: Performance comparison
- **WHEN** measuring render time for same scene
- **THEN** BCF tracer SHALL be within 2x of CPU tracer performance
- **AND** preferably faster due to better cache locality

### Requirement: GPU Translation Compatibility
The system SHALL use only operations that map directly to GLSL fragment shader code.

#### Scenario: No heap allocations during traversal
- **WHEN** tracing ray through BCF octree
- **THEN** SHALL use stack-allocated traversal state
- **AND** SHALL NOT allocate Vec, String, or Box
- **AND** SHALL use fixed-size arrays only

#### Scenario: Explicit control flow (no pattern matching)
- **WHEN** processing BCF nodes
- **THEN** SHALL use if/else chains (not match expressions)
- **AND** SHALL make branching explicit and predictable

#### Scenario: Bit operations documented for GLSL
- **WHEN** performing bit manipulations (shifts, masks, OR)
- **THEN** SHALL document equivalent GLSL operations
- **AND** SHALL use only operations available in GLSL ES 3.0

#### Scenario: Bounded iteration (no unbounded loops)
- **WHEN** traversing octree
- **THEN** SHALL limit maximum traversal depth
- **AND** SHALL limit maximum iteration count
- **AND** SHALL prevent infinite loops in GLSL translation
