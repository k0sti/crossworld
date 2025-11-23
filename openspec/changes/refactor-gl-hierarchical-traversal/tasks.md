# Tasks: Refactor GL Hierarchical Traversal

## Phase 1: Buffer Serialization (Rust)

### Task 1.1: Implement octree buffer serialization
- [ ] Create `serialize_octree_to_buffer()` function in `gl_tracer.rs`
- [ ] Implement recursive node serialization with depth-first traversal
- [ ] Handle `Cube::Solid` and `Cube::Cubes` variants
- [ ] Add unit tests for serialization correctness
- **Validation**: Test outputs match expected buffer layout for simple octrees
- **Est**: 2-3 hours

### Task 1.2: Add buffer texture creation
- [ ] Replace `create_octree_texture()` with `create_octree_buffer()`
- [ ] Use `TEXTURE_BUFFER` with `R32UI` format
- [ ] Create and bind buffer object (VBO)
- [ ] Upload buffer data with `buffer_data_u8_slice()`
- **Validation**: GL calls succeed without errors, buffer uploads correctly
- **Est**: 1-2 hours

### Task 1.3: Update struct and uniforms
- [ ] Change `octree_texture` field to `octree_buffer` in `GlTracerGl`
- [ ] Update uniform location to `octree_buffer_location`
- [ ] Remove all `TEXTURE_3D` bind calls
- [ ] Add `TEXTURE_BUFFER` bind calls in render functions
- **Validation**: Code compiles, no 3D texture references remain
- **Est**: 1 hour

## Phase 2: Shader Rewrite (GLSL)

### Task 2.1: Replace texture uniform with buffer
- [ ] Change `uniform sampler3D u_octree_texture` to `uniform usamplerBuffer u_octree_buffer`
- [ ] Remove `precision highp sampler3D` declaration
- [ ] Add `precision highp usamplerBuffer` if needed
- **Validation**: Shader compiles without errors
- **Est**: 15 minutes

### Task 2.2: Implement node reading functions
- [ ] Add `readNode(uint node_index)` to fetch 2 uint32 values
- [ ] Add `isParent(uint word0)` to check node type
- [ ] Add `getMaterial(uint word0)` for leaf nodes
- [ ] Add `getChildBaseIndex(uint word0)` for parent nodes
- **Validation**: Test with hardcoded node indices, verify bit masking
- **Est**: 1 hour

### Task 2.3: Implement octant calculation
- [ ] Add `calculateOctant(vec3 pos)` for 8-way subdivision
- [ ] Add `transformToChild(vec3 pos, uint octant)` for coordinate transform
- [ ] Add helper functions for octant bit manipulation
- **Validation**: Test octant calculation for all 8 octants
- **Est**: 1-2 hours

### Task 2.4: Implement DDA stepping
- [ ] Add `ddaStep(vec3 pos, vec3 dir, uint depth)` for boundary stepping
- [ ] Calculate next integer boundary per axis
- [ ] Select minimum positive t for step direction
- [ ] Transform position back to normalized space
- **Validation**: Test DDA steps match expected next positions
- **Est**: 2-3 hours

### Task 2.5: Rewrite main traversal loop
- [ ] Replace `getVoxelValue()` texture lookup with node traversal
- [ ] Implement hierarchical descent for parent nodes
- [ ] Implement DDA stepping for empty leaf nodes
- [ ] Add iteration limit and bounds checking
- **Validation**: Shader compiles and runs without crashes
- **Est**: 2-3 hours

## Phase 3: Integration and Validation

### Task 3.1: Wire up buffer uniform binding
- [ ] Get buffer texture location in GL setup code
- [ ] Bind texture buffer to uniform in `render_to_gl()` and `render_to_gl_with_camera()`
- [ ] Set uniform value with correct texture unit
- **Validation**: No GL errors, uniform binding succeeds
- **Est**: 1 hour

### Task 3.2: Test basic rendering
- [ ] Run `cargo run --release --single --headless`
- [ ] Verify GL tracer produces output image
- [ ] Check for GL errors or shader compilation failures
- **Validation**: GL tracer renders without crashes
- **Est**: 30 minutes

### Task 3.3: Compare against CPU tracer
- [ ] Run sync mode to generate `diff_cpu_gl.png`
- [ ] Analyze pixel differences (max, average, count)
- [ ] Investigate any large discrepancies
- **Validation**: Diff shows <1% pixel difference or explainable variance
- **Est**: 1-2 hours

### Task 3.4: Fix rendering issues
- [ ] Debug incorrect pixels (normals, colors, positions)
- [ ] Fix octant indexing errors
- [ ] Fix DDA stepping bugs
- [ ] Fix coordinate transform errors
- **Validation**: Visual output matches CPU tracer
- **Est**: 2-4 hours (depends on issues found)

### Task 3.5: Test with complex octrees
- [ ] Test with deeper octrees (depth 4-6)
- [ ] Test with mixed solid/empty regions
- [ ] Test with all octants populated
- **Validation**: Renders correctly for diverse octree structures
- **Est**: 1 hour

## Phase 4: Cleanup and Documentation

### Task 4.1: Remove deprecated code
- [ ] Delete `sample_cube_at_position()` function
- [ ] Delete `create_octree_texture()` function
- [ ] Remove unused imports (TEXTURE_3D, sampler3D)
- [ ] Clean up commented-out code
- **Validation**: Code compiles, no dead code warnings
- **Est**: 30 minutes

### Task 4.2: Add code comments
- [ ] Document buffer format in `gl_tracer.rs`
- [ ] Explain node structure (8-byte layout)
- [ ] Document traversal algorithm in shader
- [ ] Add examples of octant indexing
- **Validation**: Comments clear and accurate
- **Est**: 1 hour

### Task 4.3: Update documentation
- [ ] Update `crates/renderer/src/shaders/README.md`
- [ ] Remove 3D texture references
- [ ] Add buffer texture format description
- [ ] Document hierarchical traversal approach
- **Validation**: README matches new implementation
- **Est**: 30 minutes

### Task 4.4: Run full test suite
- [ ] Run `cargo test --release` in renderer crate
- [ ] Run `just check` for workspace-wide checks
- [ ] Fix any failing tests
- **Validation**: All tests pass
- **Est**: 1 hour

## Task Dependencies

```
Phase 1 (Rust):    1.1 → 1.2 → 1.3
                     ↓
Phase 2 (Shader):  2.1 → 2.2 → 2.3 → 2.4 → 2.5
                     ↓
Phase 3 (Test):    3.1 → 3.2 → 3.3 → 3.4 → 3.5
                     ↓
Phase 4 (Clean):   4.1 → 4.2 → 4.3 → 4.4
```

**Critical Path**: 1.1 → 1.2 → 1.3 → 2.1 → 2.5 → 3.1 → 3.3 → 3.4

**Parallelizable**:
- 2.2, 2.3, 2.4 can be developed simultaneously (different shader functions)
- 4.1, 4.2 can be done in parallel

## Total Estimated Time

- **Optimistic**: 15-18 hours (everything works first try)
- **Realistic**: 20-25 hours (some debugging needed)
- **Pessimistic**: 30-35 hours (significant troubleshooting)

## Progress Tracking

Track progress with:
```bash
openspec show refactor-gl-hierarchical-traversal
```

Update completed tasks by editing this file and checking boxes with `[x]`.
