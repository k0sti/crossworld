# Design: Hierarchical Octree Traversal for GL Tracer

## Overview

This document describes the architectural design for replacing 3D texture sampling with proper hierarchical octree traversal in the GL tracer.

## Current Architecture (Incorrect)

```
Octree (Rust)
     ↓ sample_cube_at_position()
8x8x8 Voxel Grid
     ↓ gl.tex_image_3d()
GPU 3D Texture (TEXTURE_3D)
     ↓ texture(u_octree_texture, pos)
Fragment Shader Lookup
```

**Problems:**
- Flattens hierarchical structure to dense grid
- Fixed resolution (8³ currently)
- Stores empty space explicitly
- `sampler3D` texture lookup bypasses octree logic

## Proposed Architecture (Correct)

```
Octree (Rust)
     ↓ serialize_to_buffer()
Linearized Octree Buffer (BCF-inspired)
     ↓ gl.tex_buffer() or SSBO
GPU Texture Buffer / SSBO
     ↓ texelFetch() or buffer[index]
Fragment Shader DDA Traversal
```

**Advantages:**
- Preserves hierarchical structure
- Arbitrary depth support
- Memory-efficient (empty nodes skipped)
- Proper octree semantics

## GPU Buffer Format

### Design Goals
1. **GPU-Friendly**: Byte-aligned, simple indexing
2. **Compact**: Minimize memory footprint
3. **Traversal-Optimized**: Fast child node lookup
4. **WebGL 2.0 Compatible**: No advanced GL features

### Node Structure (8 bytes)

Each octree node is encoded as 2 × uint32 (8 bytes total):

```
Word 0 (uint32): [is_parent: 1 bit] [data: 31 bits]
  - If is_parent = 0 (leaf):
      data = material_index (31 bits, supports up to 2^31 materials)
  - If is_parent = 1 (parent):
      data = child_base_index (31 bits, pointer to first child)

Word 1 (uint32): Child metadata (parent nodes only)
  - Bits 0-31: Reserved for optimization hints
  - Initial implementation: 0x00000000
```

### Buffer Layout

```
[Header: 4 bytes]
  uint32: root_node_index

[Node Data: Variable length]
  Node 0: Root node (8 bytes)
  Node 1-N: Children in depth-first order
```

### Example: Simple Octree

```rust
// Rust octree
Cubes([
    Solid(1),  // Octant 0
    Solid(2),  // Octant 1
    Solid(0),  // Octant 2 (empty)
    Solid(0),  // Octant 3 (empty)
    Solid(3),  // Octant 4
    Solid(4),  // Octant 5
    Solid(0),  // Octant 6 (empty)
    Solid(0),  // Octant 7 (empty)
])
```

```
Buffer layout (76 bytes):
[Header]
0x00: 00 00 00 00  // root_node_index = 0

[Node 0: Root (parent)]
0x04: 01 00 00 80  // is_parent=1, child_base_index=1
0x08: 00 00 00 00  // reserved

[Node 1: Octant 0 (leaf, material 1)]
0x0C: 01 00 00 00  // is_parent=0, material=1
0x10: 00 00 00 00  // unused (leaf)

[Node 2: Octant 1 (leaf, material 2)]
0x14: 02 00 00 00  // is_parent=0, material=2
0x18: 00 00 00 00  // unused (leaf)

... (nodes 3-8 for octants 2-7)
```

## Fragment Shader Algorithm

### High-Level Flow

```glsl
1. Ray-box intersection → entry point in [0,1]³
2. current_node = root_node
3. while (iterations < MAX_ITER):
     a. If current_node is leaf:
        - If non-empty: return hit
        - Else: DDA step to next octant boundary
     b. If current_node is parent:
        - Calculate octant index from position
        - Descend to child node
        - Transform position to child space
4. Return miss (background)
```

### Node Lookup (texelFetch)

```glsl
// Read node from texture buffer
uvec2 readNode(uint node_index) {
    // Each node = 2 uint32 values
    uint texel_index = node_index * 2u;
    uint word0 = texelFetch(u_octree_buffer, int(texel_index)).r;
    uint word1 = texelFetch(u_octree_buffer, int(texel_index + 1u)).r;
    return uvec2(word0, word1);
}

bool isParent(uint word0) {
    return (word0 & 0x80000000u) != 0u;
}

uint getMaterial(uint word0) {
    return word0 & 0x7FFFFFFFu;
}

uint getChildBaseIndex(uint word0) {
    return word0 & 0x7FFFFFFFu;
}
```

### Octant Calculation

```glsl
// Calculate which of 8 octants position is in
uint calculateOctant(vec3 pos) {
    // pos in [0,1]³, scale to [0,2)³
    vec3 scaled = pos * 2.0;
    uvec3 bit = uvec3(floor(scaled));

    // Octant index = (x_bit << 2) | (y_bit << 1) | z_bit
    return (bit.x << 2u) | (bit.y << 1u) | bit.z;
}

// Transform position to child coordinate space
vec3 transformToChild(vec3 pos, uint octant) {
    vec3 scaled = pos * 2.0;
    vec3 octant_min = vec3(
        float((octant >> 2u) & 1u),
        float((octant >> 1u) & 1u),
        float(octant & 1u)
    );
    return scaled - octant_min;
}
```

### DDA Stepping

```glsl
// Step ray to next octant boundary
vec3 ddaStep(vec3 pos, vec3 dir, uint current_depth) {
    // Scale position to current depth's grid
    float grid_size = pow(2.0, float(current_depth));
    vec3 grid_pos = pos * grid_size;

    // Calculate next integer boundary in direction
    vec3 sign_dir = sign(dir);
    vec3 next_boundary = floor(grid_pos + 0.5 + sign_dir * 0.5) + sign_dir * 0.5;

    // Calculate t values for each axis
    vec3 t = (next_boundary - grid_pos) / (dir * grid_size);

    // Step along axis with smallest positive t
    float min_t = min(min(t.x, t.y), t.z);
    vec3 next_pos = grid_pos + dir * min_t * grid_size;

    // Transform back to [0,1]³
    return next_pos / grid_size;
}
```

## Rust Implementation Changes

### File: `crates/renderer/src/gl_tracer.rs`

**Remove:**
- `create_octree_texture()` (lines 209-282)
- `sample_cube_at_position()` (lines 451-480)
- All `TEXTURE_3D` references
- `octree_texture` field from `GlTracerGl`

**Add:**
```rust
/// Serialize octree to GPU-friendly linear buffer
fn serialize_octree_to_buffer(cube: &Cube<i32>) -> Vec<u32> {
    let mut buffer = Vec::new();

    // Reserve space for header (root index)
    buffer.push(0);

    // Serialize root and get its index
    let root_index = serialize_node_recursive(cube, &mut buffer, 0);
    buffer[0] = root_index;

    buffer
}

fn serialize_node_recursive(
    cube: &Cube<i32>,
    buffer: &mut Vec<u32>,
    depth: u32,
) -> u32 {
    let node_index = (buffer.len() / 2) as u32;

    match cube {
        Cube::Solid(material) => {
            // Leaf node: is_parent=0, material in lower 31 bits
            buffer.push(*material as u32);
            buffer.push(0); // reserved
        }
        Cube::Cubes(children) => {
            // Parent node: is_parent=1, child_base in lower 31 bits
            let child_base = (buffer.len() / 2 + 1) as u32;
            buffer.push(0x80000000 | child_base);
            buffer.push(0); // reserved

            // Recursively serialize all 8 children
            for child in children.iter() {
                serialize_node_recursive(child, buffer, depth + 1);
            }
        }
        _ => {
            // Unsupported variants, treat as empty
            buffer.push(0);
            buffer.push(0);
        }
    }

    node_index
}
```

**Modify:**
```rust
// Replace texture with buffer texture
struct GlTracerGl {
    program: Program,
    vao: VertexArray,
    octree_buffer: Option<Texture>,  // Changed from octree_texture
    octree_buffer_location: Option<UniformLocation>,
    // ... other fields
}

unsafe fn create_octree_buffer(gl: &Context, cube: &Cube<i32>) -> Result<Texture, String> {
    let buffer_data = serialize_octree_to_buffer(cube);

    // Create buffer texture
    let buffer_texture = gl.create_texture()?;
    gl.bind_texture(TEXTURE_BUFFER, Some(buffer_texture));

    // Create buffer object
    let buffer_object = gl.create_buffer()?;
    gl.bind_buffer(TEXTURE_BUFFER, Some(buffer_object));
    gl.buffer_data_u8_slice(
        TEXTURE_BUFFER,
        bytemuck::cast_slice(&buffer_data),
        STATIC_DRAW
    );

    // Bind buffer to texture
    gl.tex_buffer(TEXTURE_BUFFER, R32UI, Some(buffer_object));

    Ok(buffer_texture)
}
```

### File: `crates/renderer/src/shaders/octree_raycast.frag`

**Replace:**
```glsl
// OLD: uniform sampler3D u_octree_texture;
uniform usamplerBuffer u_octree_buffer;  // NEW: Buffer texture

// OLD: int getVoxelValue(vec3 pos) { ... texture lookup ... }
// NEW: Hierarchical traversal (see shader algorithm above)
```

## Trade-offs and Decisions

### Decision 1: Texture Buffer vs SSBO

**Choice: Texture Buffer**

Rationale:
- ✅ WebGL 2.0 compatible (SSBO requires WebGL 2.0 compute or extension)
- ✅ Simple uniform binding (`uniform usamplerBuffer`)
- ✅ Sufficient size limit (128MB typical, 64K nodes = 512KB)
- ❌ Less flexible than SSBO for future extensions

Alternative considered: SSBO would allow dynamic updates and larger buffers, but requires compute shaders or extensions not guaranteed in WebGL 2.0.

### Decision 2: Node Size (8 bytes)

**Choice: 2 × uint32 (8 bytes)**

Rationale:
- ✅ GPU-friendly alignment (8-byte alignment common)
- ✅ 31-bit child pointers support ~2 billion nodes
- ✅ Simple indexing (`node_index * 2`)
- ❌ Some wasted space in Word 1 (reserved for future)

Alternative considered: 4-byte nodes would be more compact but limit addressable nodes to 2^16 (65K).

### Decision 3: Depth-First Serialization

**Choice: Depth-first traversal order**

Rationale:
- ✅ Matches CPU raycaster's traversal pattern
- ✅ Natural recursive serialization
- ✅ Children stored contiguously after parent
- ❌ Less cache-friendly than breadth-first

Alternative considered: Breadth-first would improve cache coherency for wide octrees but complicates pointer calculation.

## Validation Strategy

### Phase 1: Correctness (Required)
1. **Pixel-Perfect Comparison**: Render same scene with CPU and GL tracers
2. **Diff Analysis**: `diff_cpu_gl.png` should show <1% difference
3. **Visual Inspection**: Manual comparison of rendered outputs

### Phase 2: Performance (Optional)
1. **Frame Time**: Measure render time for GL tracer
2. **Memory Usage**: Compare VRAM usage (3D texture vs buffer)
3. **Acceptance**: Performance regression acceptable for correctness

### Phase 3: Edge Cases
1. **Deep Octrees**: Test with depth=8+ octrees
2. **Large Models**: Test with >10K nodes
3. **Empty Regions**: Verify DDA correctly skips empty space

## Future Enhancements (Out of Scope)

1. **Optimization**: Cache-friendly node layout, SIMD traversal hints
2. **Compression**: Pack multiple small nodes into single uint32
3. **LOD**: Level-of-detail selection based on distance
4. **Dynamic Updates**: Streaming octree updates for editing
5. **Ray Coherence**: Exploit adjacent pixel ray similarity

## References

- BCF Format Specification: `doc/architecture/bcf-format.md`
- CPU Raycast Implementation: `crates/cube/src/raycast/mod.rs`
- WebGL 2.0 Texture Buffers: https://registry.khronos.org/webgl/specs/latest/2.0/
- Efficient Octree Traversal: Revelles et al. (1996)
