# gl-bcf-octree-raycast Specification

## Purpose
TBD - created by archiving change implement-gl-bcf-octree-traversal. Update Purpose after archive.
## Requirements
### Requirement: BCF Octree Traversal Algorithm
The GL fragment shader SHALL perform hierarchical BCF octree traversal using iterative DDA algorithm.

#### Scenario: Basic octree traversal with inline leaves
- **WHEN** fragment shader receives ray for single voxel octree (depth 0)
- **AND** BCF root node is inline leaf (type_byte <= 0x7F)
- **THEN** SHALL read node type and extract value from type_byte & 0x7F
- **AND** SHALL return hit if value != 0
- **AND** SHALL return miss if value == 0

#### Scenario: Octa-leaves traversal
- **WHEN** fragment shader traverses octree with octa-leaves node (depth 1)
- **AND** BCF node type is 0x90-0x9F (octa-leaves)
- **THEN** SHALL read 8 child values (1 byte each)
- **AND** SHALL compute entry octant from ray origin and direction
- **AND** SHALL check corresponding child value
- **AND** SHALL return hit if child value != 0
- **AND** SHALL continue traversal if child value == 0

#### Scenario: Octa-pointers hierarchical descent
- **WHEN** fragment shader encounters octa-pointers node (depth 2+)
- **AND** BCF node type is 0xA0-0xAF
- **THEN** SHALL extract ssss from type_byte & 0x0F
- **AND** SHALL read 8 child pointers of size 2^ssss bytes each
- **AND** SHALL compute entry octant
- **AND** SHALL transform ray to child local space [-1,1]³
- **AND** SHALL push child traversal state to stack
- **AND** SHALL continue iterating

#### Scenario: Stack-based traversal with depth limit
- **WHEN** fragment shader traverses deep octree (depth > 1)
- **THEN** SHALL use fixed-size stack array (MAX_STACK_DEPTH = 16)
- **AND** SHALL push and pop TraversalState structs
- **AND** SHALL check stack overflow before push
- **AND** SHALL return error material 4 (sky blue) if stack overflows
- **AND** SHALL terminate traversal when stack is empty

#### Scenario: Iteration limit enforcement
- **WHEN** fragment shader traverses complex octree
- **THEN** SHALL limit main loop to MAX_ITERATIONS (256)
- **AND** SHALL return error material 4 if iteration limit exceeded
- **AND** SHALL break loop when hit found or stack empty

---

### Requirement: BCF Binary Data Reading
The GL fragment shader SHALL read BCF binary data from 2D texture using byte-level addressing.

#### Scenario: Read single byte from BCF texture
- **WHEN** shader needs to read byte at linear offset N
- **THEN** SHALL compute texel index = N / 4
- **AND** SHALL compute byte position within texel = N % 4
- **AND** SHALL convert linear texel index to 2D coordinates (x, y)
- **AND** SHALL fetch texel using texelFetch()
- **AND** SHALL extract byte from RGBA components based on position

#### Scenario: Read variable-width pointer
- **WHEN** shader reads pointer with ssss = 0 (1-byte)
- **THEN** SHALL read 1 byte and return as uint
- **WHEN** shader reads pointer with ssss = 1 (2-byte)
- **THEN** SHALL read 2 bytes little-endian and return as uint
- **WHEN** shader reads pointer with ssss = 2 (4-byte)
- **THEN** SHALL read 4 bytes little-endian and return as uint
- **WHEN** shader reads pointer with ssss = 3 (8-byte)
- **THEN** SHALL read first 4 bytes as uint (GLSL limitation)

#### Scenario: Parse BCF node type
- **WHEN** shader reads node at offset
- **THEN** SHALL read type byte
- **AND** SHALL determine node type from type byte value
- **AND** SHALL read additional data based on node type
- **AND** SHALL return structured BcfNode with type, value(s), or pointers

---

### Requirement: Octant and Ray-Space Transformations
The GL fragment shader SHALL compute octant and ray-space transformations matching CPU algorithm.

#### Scenario: Compute entry octant from ray position
- **WHEN** ray enters octree node at position P in local space [-1,1]³
- **THEN** SHALL compute octant = ivec3(P > 0.0)
- **AND** SHALL handle boundary case where P == 0.0 using ray direction sign
- **AND** SHALL return octant in range [0,0,0] to [1,1,1]

#### Scenario: Convert octant to linear index
- **WHEN** shader has octant coordinates (x, y, z)
- **THEN** SHALL compute index = x + y*2 + z*4
- **AND** SHALL return value in range 0-7

#### Scenario: Transform ray to child local space
- **WHEN** descending from parent octant to child
- **THEN** SHALL compute child_origin = (parent_origin - octant*2 + 1) * 2 - 1
- **AND** SHALL keep ray_dir unchanged
- **AND** SHALL result in child_origin in [-1,1]³ space

---

### Requirement: Surface Normal and Hit Point Calculation
The GL fragment shader SHALL calculate surface normals and hit points matching CPU implementation.

#### Scenario: Return normal from entry axis
- **WHEN** ray hits voxel via traversal state with entry normal
- **THEN** SHALL decode Axis integer to vec3 normal
- **AND** SHALL match Axis encoding from cube crate
- **AND** normal SHALL be one of (+/-X, +/-Y, +/-Z)

#### Scenario: Compute hit point at voxel entry
- **WHEN** hit detected in traversal
- **THEN** SHALL use local_origin from traversal state as hit point
- **AND** hit point SHALL be in [-1,1]³ local space of leaf node

---

### Requirement: Error Handling and Edge Cases
The GL fragment shader SHALL handle errors and edge cases with visual feedback.

#### Scenario: Empty voxel traversal
- **WHEN** shader encounters voxel with value == 0
- **THEN** SHALL treat as empty space
- **AND** SHALL continue DDA traversal to next octant
- **AND** SHALL NOT return hit

#### Scenario: Stack overflow error
- **WHEN** stack depth exceeds MAX_STACK_DEPTH during push
- **THEN** SHALL return error material 4 (sky blue)
- **AND** SHALL display animated checkered pattern
- **AND** SHALL terminate traversal

#### Scenario: Iteration limit exceeded
- **WHEN** main loop reaches MAX_ITERATIONS without hit or empty stack
- **THEN** SHALL return error material 4 (sky blue)
- **AND** SHALL break loop
- **AND** SHALL display error visualization

#### Scenario: Invalid BCF node type
- **WHEN** type byte has invalid value (msb_type == 3)
- **THEN** SHALL return error material 3 (orange - type validation error)
- **AND** SHALL display error color

#### Scenario: Ray miss
- **WHEN** traversal completes without hitting non-empty voxel
- **THEN** SHALL return miss (hit = false)
- **AND** SHALL display background color

