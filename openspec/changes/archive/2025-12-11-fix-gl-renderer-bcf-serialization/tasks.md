# Implementation Tasks

## Phase 1: Cleanup Broken Code

### Task 1.1: Remove voxel grid sampling function
- **File**: `crates/renderer/src/gl_tracer.rs`
- **Action**: Delete `sample_cube_at_position()` function (lines ~566-618)
- **Verification**: Search codebase for function name, ensure no references remain
- **Test**: `cargo build --bin renderer` succeeds

### Task 1.2: Remove broken texture creation
- **File**: `crates/renderer/src/gl_tracer.rs`
- **Action**: Delete `create_octree_texture()` method (lines ~257-341)
- **Action**: Remove octree_texture field from `GlTracerGl` struct
- **Action**: Remove `u_octree_texture` uniform location
- **Action**: Remove texture binding/deletion in init/destroy
- **Verification**: Grep for `TEXTURE_3D` and `octree_texture`, ensure no GL renderer references
- **Test**: `cargo build --bin renderer` succeeds

## Phase 2: Change to Cube<u8>

### Task 2.1: Update GlCubeTracer struct to use Cube<u8>
- **File**: `crates/renderer/src/gl_tracer.rs`
- **Action**: Change `cube: Rc<Cube<i32>>` to `cube: Rc<Cube<u8>>`
- **Action**: Update constructor signature to accept `Rc<Cube<u8>>`
- **Verification**: Code compiles after struct change
- **Note**: This makes BCF serialization direct (no conversion needed)

### Task 2.2: Update GlTracerGl::new() to accept Cube<u8>
- **File**: `crates/renderer/src/gl_tracer.rs`
- **Action**: Change signature from `fn new(gl: &Context, cube: &Cube<i32>)` to `fn new(gl: &Context, cube: &Cube<u8>)`
- **Action**: Remove all Cube<i32> type annotations in function
- **Verification**: Function signature updated, code compiles

## Phase 3: BCF Serialization

### Task 3.1: Add BCF import
- **File**: `crates/renderer/src/gl_tracer.rs`
- **Action**: Add `use cube::io::bcf::serialize_bcf;` at top of file
- **Verification**: Check Cargo.toml has `cube` dependency with BCF module
- **Test**: `cargo build --bin renderer` succeeds

### Task 3.2: Serialize cube to BCF in init_gl()
- **File**: `crates/renderer/src/gl_tracer.rs`
- **Action**: In `GlTracerGl::new()`:
  1. Serialize: `let bcf_data = serialize_bcf(cube);` (cube is now &Cube<u8>)
  2. Log size: `println!("[GL Tracer] BCF data serialized: {} bytes", bcf_data.len());`
- **Verification**: Run renderer, check log shows "BCF data serialized: X bytes" where X > 12
- **Test**: Size should be reasonable (< 1KB for octa cube)

## Phase 4: GPU Buffer Upload

### Task 4.1: Create 1D-like 2D texture for BCF data ✓
- **File**: `crates/renderer/src/gl_tracer.rs`
- **Action**: Use TEXTURE_2D with width=data_size, height=1
- **Rationale**: tex_buffer not available in glow 0.14, this is more compatible
- **Implementation**:
  1. Create texture: `let texture = gl.create_texture()?;`
  2. Bind: `gl.bind_texture(TEXTURE_2D, Some(texture));`
  3. Upload: `gl.tex_image_2d(TEXTURE_2D, 0, R8UI as i32, width, 1, 0, RED_INTEGER, UNSIGNED_BYTE, Some(&bcf_data));`
  4. Set params: MIN_FILTER=NEAREST, MAG_FILTER=NEAREST, WRAP_S/T=CLAMP_TO_EDGE
- **Verification**: Texture created and uploaded successfully

### Task 4.2: Store texture handle and data size ✓
- **File**: `crates/renderer/src/gl_tracer.rs`
- **Action**: Store in `GlTracerGl` struct:
  - `octree_texture: Option<Texture>`
  - `octree_data_size: u32`
- **Verification**: Fields populated correctly

### Task 4.3: Add cleanup for texture ✓
- **File**: `crates/renderer/src/gl_tracer.rs`
- **Action**: In `destroy()`:
  - Delete texture: `gl.delete_texture(octree_texture);`
- **Verification**: No GL errors on shutdown, no memory leaks

### Task 4.4: Update uniform locations ✓
- **File**: `crates/renderer/src/gl_tracer.rs`
- **Action**: Add `octree_data_location: Option<UniformLocation>` for sampler uniform
- **Action**: Add `octree_data_size_location: Option<UniformLocation>` for size uniform
- **Action**: Get uniforms:
  - `gl.get_uniform_location(program, "u_octree_data")`
  - `gl.get_uniform_location(program, "u_octree_data_size")`
- **Action**: Set uniforms in render:
  - `gl.uniform_1_i32(octree_data_location, 0)` (bind to texture unit 0)
  - `gl.uniform_1_u32(octree_data_size_location, bcf_data.len() as u32)`
- **Verification**: Uniforms bound correctly

## Phase 5: Fragment Shader Rewrite

### Task 5.1: Update shader uniform declarations ✓
- **File**: `crates/renderer/src/shaders/octree_raycast.frag`
- **Action**: Remove: `uniform sampler3D u_octree_texture;`
- **Action**: Add: `uniform usampler2D u_octree_data;` (1D-like 2D texture)
- **Action**: Add: `uniform uint u_octree_data_size;`
- **Verification**: Shader compiles without errors

### Task 5.2: Implement BCF byte reading functions ✓
- **File**: `crates/renderer/src/shaders/octree_raycast.frag`
- **Action**: Add helper functions:
  ```glsl
  // Read byte at offset (bounds checked)
  uint readU8(uint offset) {
      if (offset >= u_octree_data_size) return 0u;
      // 1D-like 2D texture: texelFetch(u_octree_data, ivec2(int(offset), 0), 0).r
      return texelFetch(u_octree_data, ivec2(int(offset), 0), 0).r;
  }

  // Read multi-byte pointer (little-endian)
  uint readPointer(uint offset, uint ssss) {
      uint size = 1u << ssss; // 2^ssss bytes
      if (ssss == 0u) return readU8(offset);
      if (ssss == 1u) return readU8(offset) | (readU8(offset + 1u) << 8);
      if (ssss == 2u) return readU8(offset) | (readU8(offset + 1u) << 8)
                            | (readU8(offset + 2u) << 16) | (readU8(offset + 3u) << 24);
      return 0u; // 8-byte pointers not supported
  }

  // Decode BCF type byte
  void decodeTypeByte(uint type_byte, out uint msb, out uint type_id, out uint size_val) {
      msb = (type_byte >> 7) & 1u;
      type_id = (type_byte >> 4) & 7u;
      size_val = type_byte & 15u;
  }
  ```
- **Verification**: Functions implemented and shader compiles

### Task 5.3: Implement BCF node parsing ✓
- **File**: `crates/renderer/src/shaders/octree_raycast.frag`
- **Action**: Add function:
  ```glsl
  // Parse BCF node at offset, return material value or 0 if branch
  // For branches, returns 0 and sets child_offset
  uint parseBcfNode(uint offset, uint octant, out uint child_offset) {
      uint type_byte = readU8(offset);
      uint msb, type_id, size_val;
      decodeTypeByte(type_byte, msb, type_id, size_val);

      if (msb == 0u) {
          // Inline leaf (0x00-0x7F)
          return type_byte & 0x7Fu;
      }

      if (type_id == 0u) {
          // Extended leaf (0x80-0x8F)
          return readU8(offset + 1u);
      }

      if (type_id == 1u) {
          // Octa-with-leaves (0x90-0x9F)
          return readU8(offset + 1u + octant);
      }

      if (type_id == 2u) {
          // Octa-with-pointers (0xA0-0xAF)
          uint ssss = size_val;
          uint ptr_offset = offset + 1u + (octant * (1u << ssss));
          child_offset = readPointer(ptr_offset, ssss);
          return 0u; // Not a leaf, follow pointer
      }

      return 0u; // Unknown type
  }
  ```
- **Verification**: Shader compiles, parseBcfNode implemented

### Task 5.4: Implement octree traversal ✓
- **File**: `crates/renderer/src/shaders/octree_raycast.frag`
- **Action**: Implement `raycastBcfOctree()` function:
  1. Use stack-based traversal (no recursion)
  2. Start at root (offset 12 = BCF header size)
  3. For each stack item:
     - Check ray-box intersection
     - Calculate entry point and octant
     - Parse node at current offset
     - If leaf: return hit with material value
     - If branch: push child octant onto stack
  4. Handle ray miss (stack empty)
- **Algorithm**: Stack-based iterative traversal with ray-box intersection
- **Implementation**: Max stack depth of 8, max 256 iterations
- **Verification**: Shader compiles, raycastBcfOctree implemented

### Task 5.5: Add octant calculation helper ✓
- **File**: `crates/renderer/src/shaders/octree_raycast.frag`
- **Action**: Add function to calculate octant index from ray position:
  ```glsl
  uint getOctant(vec3 pos, vec3 center) {
      uint octant = 0u;
      if (pos.x >= center.x) octant |= 1u;
      if (pos.y >= center.y) octant |= 2u;
      if (pos.z >= center.z) octant |= 4u;
      return octant;
  }
  ```
- **Verification**: Function implemented, octant calculation working

## Phase 6: Testing and Validation

### Task 6.1: Basic functionality test
- **Action**: Run `cargo run --bin renderer -- --single-frame`
- **Expected**: No crash, GL renderer panel shows colored voxels
- **Verification**:
  - Log shows "BCF data serialized: X bytes" where X > 12
  - Log does NOT show "Solid voxels: 0"
  - Rendered image is not black/empty
  - See at least 6 distinct colors

### Task 6.2: Visual comparison test
- **Action**: Run renderer in GUI mode with all three tracers
- **Expected**: GL output visually matches CPU output
- **Verification**:
  - Same voxel positions are solid
  - Colors match (allowing for lighting differences)
  - No major visual artifacts in GL render
  - Octree structure is preserved

### Task 6.3: Performance validation
- **Action**: Measure render time for GL tracer
- **Expected**: < 5ms per frame (60+ FPS)
- **Verification**: Check egui performance overlay or add timing logs
- **Comparison**: Similar to previous (broken) implementation

### Task 6.4: Automated test
- **File**: Create `crates/renderer/tests/bcf_gl_rendering_test.rs`
- **Action**: Write test that:
  1. Creates test cube
  2. Initializes GL renderer (headless context)
  3. Verifies BCF serialization produces data
  4. Verifies buffer upload succeeds
  5. Renders frame and checks output is not empty
- **Run**: `cargo test --package renderer --test bcf_gl_rendering_test`

### Task 6.5: Clippy and format check
- **Action**: Run `cargo clippy --package renderer -- -D warnings`
- **Action**: Run `cargo fmt --check`
- **Expected**: No warnings or format issues
- **Fix**: Address any clippy suggestions

## Phase 7: Documentation

### Task 7.1: Update GL tracer module docs
- **File**: `crates/renderer/src/gl_tracer.rs`
- **Action**: Update module-level doc comment:
  - Explain BCF serialization approach
  - Document that it uses SSBO (or texture buffer fallback)
  - Note coordinate system matches octree (center-based)
- **Verification**: `cargo doc --open`, review GL tracer docs

### Task 7.2: Add code comments
- **File**: `crates/renderer/src/gl_tracer.rs` and `.frag`
- **Action**: Add comments explaining:
  - BCF node type byte format
  - Pointer size calculation (2^SSSS)
  - Octant indexing (0-7 mapping)
  - Traversal algorithm overview
- **Verification**: Code review for clarity

### Task 7.3: Update CLAUDE.md if needed
- **File**: `CLAUDE.md`
- **Action**: Update renderer section to mention BCF usage
- **Action**: Remove any references to voxel grid sampling
- **Verification**: Grep for outdated info about GL renderer

## Dependencies

- Phase 1 must complete before Phase 2
- Phase 2 must complete before Phase 3
- Phase 3 must complete before Phase 4
- Phase 4 must complete before Phase 5
- Phase 5 must complete before Phase 6
- Phase 6 can partially overlap with Phase 7
- All phases must complete before marking change as complete

## Parallelizable Work

- Task 2.1 (conversion) and Task 3.1 (BCF import) can be done in parallel
- Task 5.2-5.5 (shader functions) can be developed incrementally
- Task 7.1-7.3 (documentation) can be done while testing

## Success Validation

Implementation completed (Phases 1-5):
- [x] `cargo check --package renderer` succeeds
- [x] All renderer types use Cube<u8> directly
- [x] Broken voxel grid code removed
- [x] BCF serialization implemented
- [x] 1D-like 2D texture upload working
- [x] Fragment shader rewritten for BCF
- [x] All BCF functions implemented (readU8, parseBcfNode, raycastBcfOctree)
- [x] Shader compiles without errors
- [x] Code compiles with only 2 warnings (unused variables)

Validation completed (Phase 6):
- [x] Runtime testing: GL renderer displays non-empty, colored output
- [x] Visual comparison: GL matches CPU raytracer
- [x] Performance: < 5ms per frame
- [x] No OpenGL errors in logs
- [x] BCF data size logged correctly (21 bytes for test cube)

Code quality completed (Phase 7):
- [x] `cargo clippy --package renderer -- -D warnings` passes
- [x] `cargo fmt --check` passes
- [x] Documentation updated (module docs, safety docs, BCF comments)
