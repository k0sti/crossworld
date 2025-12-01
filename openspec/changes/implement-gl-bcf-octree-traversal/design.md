# Design: GL Fragment Shader BCF Octree Traversal

## Overview

This change **replaces the broken AABB-based GL implementation** with a correct port of the CPU BCF traversal algorithm (`bcf_raycast.rs`). The design strictly follows the CPU implementation's [-1,1]³ normalized space approach, which is mathematically correct and proven to work.

## Why the Old Implementation is Broken

The existing GL shader (lines 531-719) uses AABB bounds tracking:
```glsl
// BROKEN APPROACH:
stack_min[i] = vec3(-1.0);    // Track world-space bounds
stack_max[i] = vec3(1.0);
vec3 child_min = box_center + offset_vec * box_size * 0.25;  // Calculate child bounds
vec3 child_max = child_min + box_size * 0.5;
```

**Problems:**
1. **Incorrect child bounds calculation** - The formula for child bounds doesn't match the octree subdivision math
2. **Accumulated floating-point error** - Repeated bounds calculations compound numerical errors
3. **Ray position tracking in world space** - Doesn't properly handle local-to-parent transformations
4. **Doesn't match CPU algorithm** - Impossible to verify correctness by comparison

**Result:** Produces rendering artifacts, incorrect voxel placement, and fails on hierarchical models.

## Architecture

### Algorithm Source: bcf_raycast.rs (Proven Correct)

The BCF raycast algorithm in `crates/renderer/src/bcf_raycast.rs` (lines 184-370) is the **reference implementation**:

```rust
// CPU implementation proven correct by tests:
fn bcf_raycast_impl(
    reader: &BcfReader,
    root_offset: usize,
    ray_origin: Vec3,  // In [-1,1]³ space
    ray_dir: Vec3,
) -> Option<Hit<u8>>

Key characteristics:
- Iterative with explicit stack (GPU-compatible)
- Works in normalized [-1,1]³ space AT EACH OCTREE LEVEL
- Child ray transformation: child_origin = ray_origin * 2.0 - offset (line 311)
- NO bounds tracking (bounds are always [-1,1]³ in local space)
- Matches cube::raycast behavior (passes all 33 raycast tests)
- All operations GPU-compatible (no heap allocation, no recursion)
```

**Critical insight:** Each octree node exists in its own [-1,1]³ normalized space. When descending to a child, we transform the ray to the child's local [-1,1]³ space. This eliminates the need to track bounds.

### GLSL Translation Strategy

#### 1. Type Mappings

Following the documented guide in `bcf_cpu_tracer.rs:14-32`:

| Rust Type | GLSL Type | Notes |
|-----------|-----------|-------|
| `Vec3` | `vec3` | Direct mapping |
| `f32` | `float` | Direct mapping |
| `u8` | `uint` | GLSL has no u8, use uint |
| `usize` | `uint` | Use uint for all offsets |
| `Option<T>` | `bool + T` | Separate flag + value |
| `[T; N]` | `T[N]` | Fixed-size arrays |
| `CubeCoord` | `uvec3` | Unsigned 3D coordinate |
| `Axis` | `int` | Encode as integer (-3 to +3) |

#### 2. Stack-Based Traversal (Matching CPU Exactly)

CPU uses `stack: [TraversalState; MAX_TRAVERSAL_DEPTH]` (line 190). GPU uses same structure:

```glsl
// Match CPU constants
const int MAX_STACK_DEPTH = 16;     // Same as CPU MAX_TRAVERSAL_DEPTH
const int MAX_ITERATIONS = 256;     // Safety limit (CPU uses while loop)

// GLSL can't have struct arrays efficiently, use separate arrays:
uint stack_offset[MAX_STACK_DEPTH];
vec3 stack_local_origin[MAX_STACK_DEPTH];
vec3 stack_ray_dir[MAX_STACK_DEPTH];
int stack_normal[MAX_STACK_DEPTH];
uvec3 stack_coord[MAX_STACK_DEPTH];
int stack_ptr = 0;  // 0 = empty, matches CPU

// Push state (inline, no function due to GLSL limitations)
stack_offset[stack_ptr] = offset;
stack_local_origin[stack_ptr] = local_origin;
stack_ray_dir[stack_ptr] = ray_dir;
stack_normal[stack_ptr] = normal;
stack_coord[stack_ptr] = coord;
stack_ptr++;

// Pop state (inline)
stack_ptr--;
uint offset = stack_offset[stack_ptr];
vec3 local_origin = stack_local_origin[stack_ptr];
// ... etc

// CRITICAL: No bounds arrays! Bounds are always [-1,1]³ in local space.
```

#### 3. BCF Data Reading

BCF data is uploaded as 2D texture (`usampler2D u_octree_data`). Convert linear offset to 2D texture coordinates:

```glsl
// Texture is RGBA8UI, storing 4 bytes per texel
// Layout: [byte0, byte1, byte2, byte3] per pixel

uint readByte(uint offset) {
    // Each texel holds 4 bytes
    uint texel_index = offset / 4u;
    uint byte_in_texel = offset % 4u;

    // Convert linear index to 2D coordinates
    // Assuming texture width is u_octree_data_width (uniform)
    uint x = texel_index % u_octree_data_width;
    uint y = texel_index / u_octree_data_width;

    // Fetch texel (returns uvec4)
    uvec4 texel = texelFetch(u_octree_data, ivec2(x, y), 0);

    // Extract byte
    if (byte_in_texel == 0u) return texel.r;
    if (byte_in_texel == 1u) return texel.g;
    if (byte_in_texel == 2u) return texel.b;
    return texel.a;
}

uint readPointer(uint offset, uint ssss) {
    // ssss encodes pointer size: 0=1 byte, 1=2 bytes, 2=4 bytes, 3=8 bytes
    uint size = 1u << ssss;

    if (ssss == 0u) {
        return readByte(offset);
    } else if (ssss == 1u) {
        // 2-byte little-endian u16
        uint b0 = readByte(offset);
        uint b1 = readByte(offset + 1u);
        return b0 | (b1 << 8u);
    } else if (ssss == 2u) {
        // 4-byte little-endian u32
        uint b0 = readByte(offset);
        uint b1 = readByte(offset + 1u);
        uint b2 = readByte(offset + 2u);
        uint b3 = readByte(offset + 3u);
        return b0 | (b1 << 8u) | (b2 << 16u) | (b3 << 24u);
    } else {
        // ssss == 3: 8-byte u64 (truncate to u32 in GLSL)
        // Read first 4 bytes only (limitation of uint)
        return readPointer(offset, 2u);
    }
}
```

#### 4. BCF Node Parsing

Translate `BcfNodeType` enum to GLSL:

```glsl
// Node type constants (matching BCF spec)
const uint NODE_INLINE_LEAF = 0u;      // type_byte <= 0x7F
const uint NODE_EXTENDED_LEAF = 1u;    // type_byte 0x80-0x8F
const uint NODE_OCTA_LEAVES = 2u;      // type_byte 0x90-0x9F
const uint NODE_OCTA_POINTERS = 3u;    // type_byte 0xA0-0xAF

struct BcfNode {
    uint node_type;
    uint value;           // For leaf nodes
    uint values[8];       // For octa-leaves
    uint pointers[8];     // For octa-pointers
    uint ssss;            // For octa-pointers
};

BcfNode readNode(uint offset) {
    BcfNode node;
    uint type_byte = readByte(offset);

    if (type_byte <= 0x7Fu) {
        // Inline leaf: value stored in lower 7 bits
        node.node_type = NODE_INLINE_LEAF;
        node.value = type_byte & 0x7Fu;
    } else {
        uint msb_type = (type_byte >> 4u) & 0x3u;

        if (msb_type == 0u) {
            // Extended leaf: next byte is value
            node.node_type = NODE_EXTENDED_LEAF;
            node.value = readByte(offset + 1u);
        } else if (msb_type == 1u) {
            // Octa-leaves: 8 bytes follow
            node.node_type = NODE_OCTA_LEAVES;
            for (int i = 0; i < 8; i++) {
                node.values[i] = readByte(offset + 1u + uint(i));
            }
        } else if (msb_type == 2u) {
            // Octa-pointers: ssss in lower 4 bits, 8 pointers follow
            node.node_type = NODE_OCTA_POINTERS;
            node.ssss = type_byte & 0x0Fu;
            uint ptr_offset = offset + 1u;
            for (int i = 0; i < 8; i++) {
                node.pointers[i] = readPointer(ptr_offset, node.ssss);
                ptr_offset += (1u << node.ssss);
            }
        } else {
            // Invalid type (msb_type == 3)
            node.node_type = NODE_INLINE_LEAF;
            node.value = 3u;  // Error material 3 (type validation error)
        }
    }

    return node;
}
```

#### 5. DDA Traversal Algorithm

Port the main traversal loop from `bcf_raycast_impl()`:

```glsl
HitInfo raycastBcf(vec3 ray_origin, vec3 ray_dir) {
    HitInfo miss;
    miss.hit = false;

    // Initialize traversal
    vec3 dir_sign = sign(ray_dir);
    TraversalState initial_state;
    initial_state.offset = u_root_offset;  // From BCF header
    initial_state.local_origin = ray_origin;
    initial_state.ray_dir = ray_dir;
    initial_state.normal = 0;  // No entry normal for root
    initial_state.coord = uvec3(0u);

    pushState(initial_state);

    // Main traversal loop
    for (int iter = 0; iter < MAX_ITERATIONS; iter++) {
        if (stack_top <= 0) break;  // No more nodes to process

        TraversalState state = popState();

        // Read node at current offset
        BcfNode node = readNode(state.offset);

        // Handle different node types
        if (node.node_type == NODE_INLINE_LEAF || node.node_type == NODE_EXTENDED_LEAF) {
            uint value = node.value;
            if (value != 0u) {
                // Hit non-empty voxel
                HitInfo hit;
                hit.hit = true;
                hit.value = int(value);
                hit.normal = decodeAxis(state.normal);
                hit.point = state.local_origin;  // Entry point
                return hit;
            }
            // Empty voxel (value == 0), continue to next octant
            continue;

        } else if (node.node_type == NODE_OCTA_LEAVES) {
            // Determine which child octant to enter
            vec3 pos = state.local_origin;
            ivec3 octant = computeOctant(pos, dir_sign);
            int octant_idx = octantToIndex(octant);

            uint value = node.values[octant_idx];
            if (value != 0u) {
                // Hit
                HitInfo hit;
                hit.hit = true;
                hit.value = int(value);
                hit.normal = decodeAxis(state.normal);
                hit.point = pos;
                return hit;
            }
            // Empty child, continue
            continue;

        } else if (node.node_type == NODE_OCTA_POINTERS) {
            // Subdivide and push child states
            // (See bcf_raycast_impl lines 250-350 for full algorithm)

            // Compute child offset and transform ray to child space
            vec3 pos = state.local_origin;
            ivec3 octant = computeOctant(pos, dir_sign);
            int octant_idx = octantToIndex(octant);

            uint child_offset = node.pointers[octant_idx];

            // Transform to child local space [-1,1]³
            vec3 child_origin = (pos - vec3(octant) * 2.0 + vec3(1.0)) * 2.0 - vec3(1.0);

            // Push child state
            TraversalState child_state;
            child_state.offset = child_offset;
            child_state.local_origin = child_origin;
            child_state.ray_dir = ray_dir;  // Same direction
            child_state.normal = state.normal;  // Inherit or compute new normal
            child_state.coord = state.coord * 2u + uvec3(octant);

            pushState(child_state);
        }
    }

    return miss;  // No hit found
}
```

#### 6. Loop Bounds and Iteration Limits

GLSL requires compile-time loop bounds. Use conservative maximums:

- `MAX_ITERATIONS = 256` - Maximum number of nodes visited
- `MAX_STACK_DEPTH = 16` - Maximum octree depth (covers depth 0-15)

If limits exceeded, return error material:
- Material 4 (sky blue) - Stack/iteration errors

## Data Flow

```
Fragment Shader Input
    ↓
Generate Ray (camera + screen position)
    ↓
Initialize Traversal Stack (root node)
    ↓
Main Loop (up to MAX_ITERATIONS)
    ↓
Pop State from Stack
    ↓
Read BCF Node (readNode via texture)
    ↓
Node Type?
├─ Inline/Extended Leaf → Check value → Hit or Continue
├─ Octa-Leaves → Check child value → Hit or Continue
└─ Octa-Pointers → Push child state → Continue
    ↓
Hit Found? → Calculate color with lighting
    ↓
No Hit? → Background color
    ↓
Output FragColor
```

## Testing Strategy

### Phase 1: Basic Traversal
1. Test depth 0 (single voxel) - `create_single_red_voxel()`
2. Verify inline leaf reading works
3. Confirm hit detection and normal calculation

### Phase 2: Depth 1 (Octa-Leaves)
1. Test `create_octa_cube()` - 2x2x2 octree
2. Verify octa-leaves node type parsed correctly
3. Confirm all 8 octants render with correct colors
4. Compare pixel-by-pixel with CPU output

### Phase 3: Depth 2+ (Octa-Pointers)
1. Test `create_extended_octa_cube()` - depth 2
2. Verify pointer reading and child state pushing
3. Test `create_depth_3_cube()` - complex structure
4. Validate stack doesn't overflow

### Phase 4: Edge Cases
1. Test empty octrees (all value == 0)
2. Test rays that miss entirely
3. Test boundary conditions (ray exactly on voxel edge)
4. Test error materials for invalid data

### Phase 5: Performance
1. Benchmark 512x512 render time vs CPU
2. Verify GPU is 10x+ faster for complex scenes
3. Profile iteration count per pixel

## Critical Algorithm Details

### Child Ray Transformation (Most Important!)

When descending from parent to child octant, we must transform the ray to the child's local [-1,1]³ space:

```glsl
// Parent octant in {0,1}³
ivec3 octant = computeOctant(local_origin, dir_sign);

// Offset for this octant in parent's [-1,1]³ space
vec3 offset = vec3(octant) * 2.0 - 1.0;  // Maps {0,1}³ → {-1,1}³

// Transform ray to child's [-1,1]³ space (THIS IS THE KEY LINE!)
vec3 child_origin = local_origin * 2.0 - offset;

// Verify child_origin is in [-1,1]³:
// - If local_origin = (0.5, 0.5, 0.5) and octant = (1, 1, 1), offset = (1, 1, 1)
// - child_origin = (0.5, 0.5, 0.5) * 2 - (1, 1, 1) = (1, 1, 1) - (1, 1, 1) = (0, 0, 0) ✓
```

This formula (from `bcf_raycast.rs:311`) is **mathematically exact** and preserves ray direction while scaling the origin to the child's coordinate system.

## Trade-offs and Decisions

### Decision 1: Normalized Space vs AABB Tracking
**Choice:** [-1,1]³ normalized space (matching CPU exactly)

**Rationale:**
- Mathematically provably correct (CPU passes all tests)
- No bounds tracking needed (always [-1,1]³ in local space)
- Exact 1:1 correspondence with CPU for validation
- Simpler mental model and easier to debug

**Alternative rejected:** AABB bounds tracking (the broken existing approach)
- More complex (6 floats per stack entry vs 0)
- Prone to floating-point error accumulation
- Hard to verify correctness
- **Already proven to be broken in existing shader**

### Decision 2: Texture Format for BCF Data
**Choice:** `RGBA8UI` 2D texture (4 bytes per texel)

**Rationale:**
- Existing GL tracer already uses this format
- Efficient: single texture fetch gets 4 bytes
- Linear offset → 2D coord conversion is simple

**Alternative considered:** 1D texture (limited max size on some GPUs)

### Decision 3: Error Handling Strategy
**Choice:** Return error materials (1-7) for failures

**Rationale:**
- Matches existing error coloring system
- Visual feedback for debugging
- No need for exception handling in GLSL

**Alternative considered:** Silent failures (harder to debug)

### Decision 4: Iteration Limit
**Choice:** `MAX_ITERATIONS = 256`

**Rationale:**
- Conservative upper bound (typical scenes use < 50 iterations)
- Prevents infinite loops from malformed BCF data
- Allows compiler to optimize loop

**Alternative considered:** Unbounded loop (risky on GPU)

## Dependencies

### Required Completions
1. `add-cpu-bcf-traversal-raytracer` - CPU reference (84/85 tasks done)
2. `implement-gpu-raytracer` - GL infrastructure (52/57 tasks done)

### Leverages Existing Work
- BCF format specification (`cube::io::bcf`)
- BCF texture upload in `gl_tracer.rs`
- Shader compilation in `shader_utils.rs`
- Material palette system
- Error coloring system (materials 1-7)

## Open Questions

1. **Texture width for BCF data** - Currently not a uniform. Need to add `u_octree_data_width`?
   - **Resolution:** Add uniform when implementing readByte()

2. **Axis encoding** - How to represent `Axis` enum in GLSL?
   - **Resolution:** Use int with values -3 to +3 (matching cube::Axis::as_i8)

3. **Coordinate overflow** - What if `coord * 2 + octant` exceeds uint range?
   - **Resolution:** Use uvec3, depth 16 max means coord < 2^16 (well within uint range)

## Future Enhancements (Out of Scope)

- Compute shader version (Phase 2 of GPU tracer)
- Sparse voxel cone tracing (advanced lighting)
- Mipmapping for distance rendering
- Instancing for repeated structures
