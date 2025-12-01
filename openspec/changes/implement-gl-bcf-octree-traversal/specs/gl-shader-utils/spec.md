# Spec: GL Shader Utilities for BCF Traversal

## ADDED Requirements

### Requirement: GLSL Helper Functions
The GL fragment shader SHALL provide GLSL helper functions for BCF octree operations.

#### Scenario: Sign function for direction vectors
- **WHEN** shader computes ray direction signs
- **THEN** SHALL return vec3 with components in {-1.0, +1.0}
- **AND** SHALL treat zero as positive (+1.0)

#### Scenario: Octant to index conversion
- **WHEN** shader converts 3D octant coordinate to linear index
- **THEN** SHALL compute index = x + y*2 + z*4
- **AND** SHALL return uint in range 0-7

#### Scenario: Axis encoding and decoding
- **WHEN** shader stores Axis as integer
- **THEN** SHALL use values: -3=NegZ, -2=NegY, -1=NegX, 0=None, +1=PosX, +2=PosY, +3=PosZ
- **WHEN** shader decodes Axis to normal vector
- **THEN** SHALL return corresponding vec3 unit vector

---

### Requirement: TraversalState Structure
The GL fragment shader SHALL provide TraversalState struct for stack-based iteration.

#### Scenario: TraversalState structure
- **WHEN** shader defines traversal state
- **THEN** SHALL include:
  - uint offset (BCF data offset)
  - vec3 local_origin (ray origin in [-1,1]³)
  - vec3 ray_dir (ray direction, unchanged across levels)
  - int normal (entry axis encoded as integer)
  - uvec3 coord (octree coordinate)

#### Scenario: Fixed-size stack array
- **WHEN** shader declares traversal stack
- **THEN** SHALL use fixed-size array TraversalState[MAX_STACK_DEPTH]
- **AND** MAX_STACK_DEPTH SHALL be at least 16
- **AND** SHALL track stack_top index (0 = empty)

#### Scenario: Push state to stack
- **WHEN** shader pushes state to stack
- **THEN** SHALL check stack_top < MAX_STACK_DEPTH
- **AND** SHALL assign state to stack[stack_top]
- **AND** SHALL increment stack_top
- **AND** SHALL handle overflow by returning error

#### Scenario: Pop state from stack
- **WHEN** shader pops state from stack
- **THEN** SHALL check stack_top > 0
- **AND** SHALL decrement stack_top
- **AND** SHALL return stack[stack_top]

---

### Requirement: BcfNode Structure
The GL fragment shader SHALL provide BcfNode struct for parsed node data.

#### Scenario: BcfNode structure
- **WHEN** shader defines BCF node representation
- **THEN** SHALL include:
  - uint node_type (NODE_INLINE_LEAF, NODE_EXTENDED_LEAF, NODE_OCTA_LEAVES, NODE_OCTA_POINTERS)
  - uint value (for leaf nodes)
  - uint values[8] (for octa-leaves)
  - uint pointers[8] (for octa-pointers)
  - uint ssss (pointer size encoding for octa-pointers)

#### Scenario: Node type constants
- **WHEN** shader defines node type identifiers
- **THEN** SHALL use:
  - const uint NODE_INLINE_LEAF = 0u
  - const uint NODE_EXTENDED_LEAF = 1u
  - const uint NODE_OCTA_LEAVES = 2u
  - const uint NODE_OCTA_POINTERS = 3u
