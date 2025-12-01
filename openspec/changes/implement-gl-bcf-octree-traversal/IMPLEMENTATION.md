# Implementation Status: GL BCF Octree Traversal

## Summary

**The existing GL fragment shader implementation is BROKEN and must be replaced.**

The GL fragment shader (`octree_raycast.frag` lines 531-719) contains a flawed implementation using AABB bounds tracking that produces incorrect rendering. It must be replaced with a correct implementation based on the proven CPU algorithm (`bcf_raycast.rs`) which uses [-1,1]³ normalized space.

## What is Broken

### Broken Implementation (octree_raycast.frag lines 531-719)

The `raycastBcfOctree` function uses AABB bounds tracking which is **fundamentally flawed**:

```glsl
// BROKEN: Tracks world-space bounds for each stack entry
vec3 stack_min[MAX_STACK];
vec3 stack_max[MAX_STACK];

// BROKEN: Child bounds calculation doesn't match octree math
vec3 child_min = box_center + offset_vec * box_size * 0.25;
vec3 child_max = child_min + box_size * 0.5;
```

**Why it's broken:**
1. Child bounds formula is incorrect for octree subdivision
2. Ray position tracked in world space doesn't properly transform between levels
3. Accumulated floating-point errors as depth increases
4. Doesn't match the proven CPU algorithm, impossible to validate

**Symptoms:**
- Rendering artifacts on complex models
- Incorrect voxel placement
- Failures on depth 2+ hierarchical structures

### What Can Be Reused

#### BCF Data Reading (KEEP THESE)

**BCF Data Reading Functions (lines 196-248) - REUSABLE**
- ✅ `readU8(offset, error_material)` - Read single byte from BCF texture with bounds checking
- ✅ `readPointer(offset, ssss, error_material)` - Read variable-width pointers (1, 2, 4, 8 bytes) in little-endian
- ✅ `decodeTypeByte(type_byte, msb, type_id, size_val)` - Parse BCF type byte format
- ✅ `getOctant(pos, center)` - Compute octant index from position

**BCF Node Parsing (lines 259-321) - REUSABLE**
- ✅ `parseBcfNode(offset, octant, child_offset, error_material)` - Parse BCF node and return material or child pointer
  - ✅ Inline leaves (0x00-0x7F)
  - ✅ Extended leaves (0x80-0x8F)
  - ✅ Octa-leaves (0x90-0x9F)
  - ✅ Octa-pointers (0xA0-0xAF)
  - ✅ Error detection and material assignment

**Octree Math Helpers (lines 359-380) - REUSABLE**
- ✅ `sign3(v)` - Compute direction signs (returns -1 or +1)
- ✅ `computeOctant(pos, dir_sign)` - Compute starting octant from position and direction
- ✅ `octantToIndex(o)` - Convert 3D octant to linear index (0-7)
- ✅ `minTimeAxis(t)` - Find axis with minimum time for DDA step

#### Stack-Based Hierarchical Traversal (lines 531-719)
- ✅ Fixed-size stack arrays (MAX_STACK = 16 levels)
- ✅ Stack overflow detection (returns error material 4)
- ✅ Iterative traversal loop (MAX_ITERATIONS = 512)
- ✅ DDA octant stepping with time calculation
- ✅ Hierarchical descent (push child nodes to stack)
- ✅ Ray-space transformations for child bounds
- ✅ Entry normal tracking and propagation

#### Hit Detection & Normal Calculation (lines 644-651, 701-703)
- ✅ Non-empty voxel detection (value != 0)
- ✅ Entry normal from DDA step
- ✅ Hit point calculation
- ✅ Material value extraction

#### Error Handling (lines 636-642, 661-667, 712-716)
- ✅ BCF read errors → error materials (2, 3, 5, 6)
- ✅ Stack overflow → error material 4
- ✅ Iteration timeout → error material 4
- ✅ Animated error visualization (checkered patterns)

### GL Tracer Integration (`gl_tracer.rs`)
- ✅ BCF serialization (line 236: `serialize_bcf(cube)`)
- ✅ Texture upload (line 246-270: 1D-like 2D texture, RGBA8UI format)
- ✅ Shader uniform binding (lines 392-465)
- ✅ Material palette system
- ✅ Lighting calculations (matches CPU implementation)

## Why AABB Tracking is Wrong

The existing implementation claims both approaches are equivalent, but **this is FALSE**. The AABB approach has fundamental errors:

### The Fatal Flaw

```glsl
// Current broken code (line 656-658):
vec3 offset_vec = vec3(octant) * 2.0 - 1.0;
vec3 child_min = box_center + offset_vec * box_size * 0.25;
vec3 child_max = child_min + box_size * 0.5;
```

**Problem:** This formula assumes the child occupies a quarter of the parent's volume centered at `offset_vec * box_size * 0.25`. But octree children are NOT positioned this way!

**Correct formula (from bcf_raycast.rs:311):**
```rust
let offset = octant.as_vec3() * 2.0 - 1.0;
let child_origin = ray_origin * 2.0 - offset;
```

This transforms the ray to the child's **own [-1,1]³ space**, not to a sub-box within the parent's space.

### Why Normalized Space is Correct

Each octree node has its own coordinate system where it occupies [-1,1]³. When descending:
1. We're entering a child that occupies 1/8th of the parent
2. In the child's view, IT is still [-1,1]³ (full cube)
3. We transform the ray from parent's [-1,1]³ to child's [-1,1]³
4. **No absolute bounds needed** - always [-1,1]³ relative to current node

The AABB approach tries to maintain absolute world-space bounds, which leads to incorrect child bound calculations.

## Implementation Plan

### Phase 1: Remove Broken Code
1. Delete `raycastBcfOctree` function (lines 531-719)
2. Keep helper functions and BCF reading infrastructure
3. Create new empty `raycastBcfOctree` skeleton

### Phase 2: Implement Correct Algorithm
1. Follow `bcf_raycast_impl` from CPU (lines 184-370)
2. Use [-1,1]³ normalized space approach
3. Implement stack with NO bounds arrays
4. Child transformation: `child_origin = local_origin * 2.0 - offset`
5. DDA octant stepping in normalized space

### Phase 3: Validation
1. Test with single voxel (depth 0)
2. Test with octa cube (depth 1)
3. Test with extended octa cube (depth 2)
4. Compare pixel-by-pixel with CPU tracer
5. Verify <0.1% difference (floating-point tolerance)

## Remaining Work

**Critical:** Replace broken implementation with correct one based on CPU algorithm

### High Priority
1. **Visual validation** - Compare GL render vs CPU render for various models
   - Single voxel (depth 0)
   - Octa cube (depth 1)
   - Extended octa cube (depth 2)
   - Depth 3 complex models

2. **Pixel diff testing** - Use existing diff view in egui app
   - Should show <0.1% difference (floating-point tolerance)
   - Verify error materials display correctly

3. **Performance benchmarking** - Measure GL vs CPU render times
   - Target: 10x+ speedup on GPU
   - Test various resolutions (256x256, 512x512, 1024x1024)

### Optional Enhancements (Future Work)
1. **Algorithm alignment** - Consider migrating to [-1,1]³ normalized space approach for closer CPU alignment
2. **Code comments** - Add more detailed GLSL comments explaining algorithm steps
3. **Shader optimization** - Profile and optimize hot paths

## Conclusion

The change `implement-gl-bcf-octree-traversal` is **NOT complete**. The existing GL implementation is broken and must be replaced.

**Status:**
- ❌ Current GL traversal produces incorrect results (AABB bounds approach is flawed)
- ✅ CPU traversal is correct and proven (bcf_raycast.rs passes all tests)
- ✅ BCF reading infrastructure can be reused
- ❌ Main traversal algorithm must be rewritten

**Next action:** Implement correct traversal algorithm following bcf_raycast.rs exactly, using [-1,1]³ normalized space.

**Success criteria:** Pixel-perfect match with CPU tracer on all test models (depths 0-3).
