# Tasks: Replace Broken GL Implementation with Correct BCF Traversal

## 0. Remove Broken Implementation
- [ ] 0.1 Delete broken `raycastBcfOctree` function (lines 531-719 in octree_raycast.frag)
- [ ] 0.2 Remove broken AABB stack arrays (stack_min, stack_max)
- [ ] 0.3 Keep BCF reading functions (readU8, readPointer, parseBcfNode)
- [ ] 0.4 Keep math helpers (sign3, computeOctant, octantToIndex, minTimeAxis)
- [ ] 0.5 Create empty function skeleton for new implementation

## 1. GLSL Infrastructure Setup (Based on bcf_raycast.rs)
- [ ] 1.1 Add constants matching CPU: MAX_STACK_DEPTH=16, MAX_ITERATIONS=256
- [ ] 1.2 Add BCF node type constants (NODE_INLINE_LEAF, etc.) if not present
- [ ] 1.3 Remove old TraversalState struct if it exists
- [ ] 1.4 Declare separate stack arrays (NO bounds arrays):
  - [ ] uint stack_offset[MAX_STACK_DEPTH]
  - [ ] vec3 stack_local_origin[MAX_STACK_DEPTH]
  - [ ] vec3 stack_ray_dir[MAX_STACK_DEPTH]
  - [ ] int stack_normal[MAX_STACK_DEPTH]
  - [ ] uvec3 stack_coord[MAX_STACK_DEPTH]
- [ ] 1.5 Declare stack_ptr variable (int, 0 = empty)

## 2. Implement Main Raycast Function from bcf_raycast_impl (lines 184-370)
- [ ] 2.1 Function signature: `HitInfo raycastBcfOctree(vec3 pos, vec3 dir)`
- [ ] 2.2 Initialize result (miss by default)
- [ ] 2.3 Compute dir_sign = sign3(dir)
- [ ] 2.4 Handle ray entry to [-1,1]³ cube (if outside, compute entry point)
- [ ] 2.5 Initialize stack with root state:
  - [ ] offset = BCF_HEADER_SIZE (or from uniform if available)
  - [ ] local_origin = ray entry point in [-1,1]³
  - [ ] ray_dir = dir (unchanged)
  - [ ] normal = entry normal axis
  - [ ] coord = uvec3(0, 0, 0)
  - [ ] stack_ptr = 1

## 3. Main Iteration Loop (Matching bcf_raycast_impl lines 215-370)
- [ ] 3.1 While loop: `while (stack_ptr > 0 && iter < MAX_ITERATIONS)`
- [ ] 3.2 Pop state from stack (decrement stack_ptr, read values)
- [ ] 3.3 Read BCF node at current offset using parseBcfNode (if available) or readU8
- [ ] 3.4 Branch on node type: InlineLeaf, ExtendedLeaf, OctaLeaves, OctaPointers

## 4. Handle Inline Leaf / Extended Leaf (Matching lines 227-238)
- [ ] 4.1 Check if node type is inline leaf (type_byte <= 0x7F) or extended leaf
- [ ] 4.2 Extract value from node
- [ ] 4.3 If value != 0: return hit with value, normal, position, coord
- [ ] 4.4 If value == 0: continue to next stack item (empty voxel)

## 5. Handle Octa-Leaves (Matching lines 240-290)
- [ ] 5.1 Detect octa-leaves node type (0x90-0x9F)
- [ ] 5.2 Read 8 child values (1 byte each)
- [ ] 5.3 Compute starting octant: `octant = computeOctant(local_origin, dir_sign)`
- [ ] 5.4 DDA loop through octants:
  - [ ] 5.4.1 Get octant index: `oct_idx = octantToIndex(octant)`
  - [ ] 5.4.2 Get value: `value = values[oct_idx]`
  - [ ] 5.4.3 If value != 0: return hit
  - [ ] 5.4.4 DDA step to next octant (lines 264-289):
    - [ ] Compute far_side, adjusted, dist, time
    - [ ] Find exit_axis = minTimeAxis(time)
    - [ ] Advance ray: `local_origin += ray_dir * time[exit_axis]`
    - [ ] Step octant in exit direction
    - [ ] Snap to boundary
    - [ ] Update entry normal
    - [ ] Check if exited parent (octant out of [0,1] range), break if so

## 6. Handle Octa-Pointers (Matching lines 293-360)
- [ ] 6.1 Detect octa-pointers node type (0xA0-0xAF)
- [ ] 6.2 Extract ssss from type byte
- [ ] 6.3 Read 8 child pointers
- [ ] 6.4 DDA loop to collect children in traversal order:
  - [ ] 6.4.1 Compute current octant
  - [ ] 6.4.2 Get child_offset = pointers[oct_idx]
  - [ ] 6.4.3 If child_offset > 0 (non-empty child):
    - [ ] Transform ray to child space: `child_origin = local_origin * 2.0 - offset`
    - [ ] Where `offset = vec3(octant) * 2.0 - 1.0`
    - [ ] Compute child coord: `child_coord = coord * 2u + uvec3(octant)`
    - [ ] Store child state for pushing to stack
  - [ ] 6.4.4 DDA step to next octant (same as octa-leaves)
  - [ ] 6.4.5 Check exit condition
- [ ] 6.5 Push collected children to stack in REVERSE order (depth-first, front-to-back)
- [ ] 6.6 Check stack overflow before each push (return error material 4 if overflow)

## 7. Error Handling
- [ ] 7.1 Iteration timeout: if iter >= MAX_ITERATIONS, return error material 4
- [ ] 7.2 Stack overflow: return error material 4
- [ ] 7.3 BCF read errors: use error_material from parseBcfNode
- [ ] 7.4 No hit found: return miss (hit = false)

## 8. Axis Encoding/Decoding
- [ ] 8.1 Implement Axis encoding: int (0=none, -3..+3 for axes)
- [ ] 8.2 Implement Axis to vec3 normal conversion
- [ ] 8.3 Implement Axis flip (for entry normal from exit direction)

## 9. Integration with Existing Shader
- [ ] 9.1 Verify BCF_HEADER_SIZE constant is defined (should be 12)
- [ ] 9.2 Ensure parseBcfNode is compatible with new approach (or rewrite node parsing inline)
- [ ] 9.3 Update main() to call new raycastBcfOctree
- [ ] 9.4 Verify lighting and material systems still work

## 10. Testing - Depth 0 (Single Voxel)
- [ ] 10.1 Load single_red_voxel model in renderer
- [ ] 10.2 Render with GL tracer
- [ ] 10.3 Render with CPU BCF tracer
- [ ] 10.4 Visual comparison (should show single red voxel)
- [ ] 10.5 Pixel diff (should be <0.1% difference)

## 11. Testing - Depth 1 (Octa Cube)
- [ ] 11.1 Load octa_cube model (2x2x2 with 6 colored voxels, 2 empty)
- [ ] 11.2 Render with GL tracer
- [ ] 11.3 Render with CPU tracer
- [ ] 11.4 Verify all 6 voxels visible with correct colors
- [ ] 11.5 Pixel diff comparison

## 12. Testing - Depth 2 (Extended Octa Cube)
- [ ] 12.1 Load extended_octa_cube model
- [ ] 12.2 Verify octa-pointers traversal works (subdivisions visible)
- [ ] 12.3 Check sparse subdivision (1 cyan voxel) renders correctly
- [ ] 12.4 Check packed subdivision (8 rainbow voxels) renders correctly
- [ ] 12.5 Compare with CPU output

## 13. Testing - Depth 3 (Complex Model)
- [ ] 13.1 Load depth_3_cube model (random scattered cubes)
- [ ] 13.2 Verify deep hierarchical descent works
- [ ] 13.3 Check no stack overflow errors
- [ ] 13.4 Verify iteration count stays under MAX_ITERATIONS
- [ ] 13.5 Compare with CPU output (should match exactly)

## 14. Edge Case Testing
- [ ] 14.1 Test completely empty octree (all value == 0)
- [ ] 14.2 Test ray that misses octree entirely
- [ ] 14.3 Test ray on exact voxel boundary
- [ ] 14.4 Test camera inside octree
- [ ] 14.5 Test very long rays (should hit or miss, not hang)

## 15. Performance Validation
- [ ] 15.1 Benchmark GL render time (512x512, depth 3 model)
- [ ] 15.2 Benchmark CPU render time (same scene)
- [ ] 15.3 Verify GL is at least 10x faster
- [ ] 15.4 Profile iteration count per pixel (add debug mode if needed)
- [ ] 15.5 Verify no performance regression vs broken implementation

## 16. Code Quality
- [ ] 16.1 Add GLSL comments explaining algorithm steps
- [ ] 16.2 Reference bcf_raycast.rs line numbers in comments
- [ ] 16.3 Run cargo clippy on gl_tracer.rs (if modified)
- [ ] 16.4 Run cargo fmt on modified Rust files
- [ ] 16.5 Verify shader compiles without errors

## 17. Documentation
- [ ] 17.1 Update IMPLEMENTATION.md with final status
- [ ] 17.2 Document any deviations from CPU algorithm (if any)
- [ ] 17.3 Add usage notes to CLAUDE.md if needed
- [ ] 17.4 Create git commit with descriptive message
- [ ] 17.5 Update this tasks.md with completion status

## Status Notes

**Critical Insight:** The old AABB-based implementation (lines 531-719) is mathematically incorrect. We must replace it with the proven [-1,1]³ normalized space algorithm from bcf_raycast.rs.

**Key Formula:** When descending to child, transform ray: `child_origin = local_origin * 2.0 - offset` where `offset = vec3(octant) * 2.0 - 1.0`. This is the ONLY correct transformation.

**Reusable Components:**
- BCF reading functions (readU8, readPointer)
- BCF node parsing (parseBcfNode) - may need minor tweaks
- Math helpers (sign3, computeOctant, octantToIndex, minTimeAxis)

**Must Replace:**
- Entire traversal loop (lines 531-719)
- Stack arrays (remove bounds arrays)
- Child descent logic (use normalized space transform)

**Success Criteria:**
1. Pixel-perfect match with CPU tracer (< 0.1% diff)
2. All depth levels (0-3) render correctly
3. No stack overflow or iteration timeout on valid models
4. 10x+ performance improvement over CPU
