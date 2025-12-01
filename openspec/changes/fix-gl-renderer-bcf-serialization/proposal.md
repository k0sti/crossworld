# Change: Fix GL Renderer with BCF-Based Octree Serialization

## Why

The GL renderer (WebGL 2.0 fragment shader raytracer) is currently broken and renders empty/black output. Investigation reveals critical issues with the current octree serialization approach:

**Current Broken Implementation:**
- Uses `sample_cube_at_position()` to create an 8×8×8 voxel grid by sampling octree positions
- Converts normalized [0,1]³ coordinates to integer grid coordinates [0,8)
- Calls `cube.get_id(depth, pos)` with these positions
- **CRITICAL BUG**: The octree uses **center-based [-1,1]³ coordinates** (e.g., [-8, -6, -4, -2, 2, 4, 6, 8] at depth 3), NOT [0,8) coordinates
- Result: All sampled positions are invalid, `get_id()` returns 0 for all voxels
- Output shows: "Solid voxels: 0 (0.0%)" confirming complete failure
- Fragment shader receives empty texture and renders nothing

**Why This Matters:**
- GL renderer is one of three validation raytracers (CPU, GL, GPU) used for correctness testing
- Cannot validate raycast implementation without working GL renderer
- Current approach fundamentally misunderstands octree coordinate system
- Voxel grid sampling loses octree structure and spatial coherence benefits

**Evidence:**
```bash
$ cargo run --bin renderer -- --single-frame 2>&1 | grep "Solid voxels"
[GL Tracer] Solid voxels: 0 (0.0%)
```

**Root Cause Analysis:**
- File: `crates/renderer/src/gl_tracer.rs:581-607`
- Function: `sample_cube_at_position()`
- Line 590-594: Converts [0,1] → [0,8) integer coordinates
- Line 607: Calls `cube.get_id(max_depth, octree_pos)` with wrong coordinate system
- The octree internals use center-based coordinates (documented in `crates/cube/src/core/cube.rs:7-8`)

**Proper Solution:**
Use the existing Binary Cube Format (BCF) which:
- Already implemented and tested (see `openspec/changes/add-binary-cube-format/`)
- Preserves exact octree structure with compact binary encoding
- Supports GPU upload as byte buffer (SSBO or texture buffer)
- Designed for GPU-friendly traversal with simple bit operations
- Proven format: 10-20x smaller than CSM text, 5x faster parsing

## What Changes

### Phase 1: Remove Broken Voxel Grid Code
- **`crates/renderer/src/gl_tracer.rs`**
  - Delete `sample_cube_at_position()` function (lines 566-618)
  - Delete `create_octree_texture()` method (lines 257-341)
  - Remove 3D texture upload logic (OpenGL `TEXTURE_3D`)
  - Remove unused texture binding in shader setup

### Phase 2: Change Renderer to Use Cube<u8> (Simplified)
- **`crates/renderer/src/gl_tracer.rs`**
  - Change `GlCubeTracer` to store `Rc<Cube<u8>>` instead of `Rc<Cube<i32>>`
  - Update constructor to accept `Rc<Cube<u8>>`
  - Remove all `Cube<i32>` references from GL tracer implementation
  - **SIMPLIFIED SCOPE**: Assumes input cubes already use u8 material indices
  - **RATIONALE**: BCF format requires Cube<u8>, so enforce this at API level

**IMPORTANT NOTE**: This change does NOT require a fully working BCF serializer. The existing BCF implementation (phases 1-4 of `add-binary-cube-format`) already supports Solid and Cubes variants for Cube<u8>, which is sufficient for the current renderer usage. Full Quad/Layers support is deferred to future work.

### Phase 3: BCF Serialization for GPU Upload
- **`crates/renderer/Cargo.toml`**
  - Ensure `cube` dependency includes BCF feature (currently `path = "../cube"`)

- **`crates/renderer/src/gl_tracer.rs`**
  - Import `cube::io::bcf::serialize_bcf`
  - Serialize cube to BCF binary format: `let bcf_data = serialize_bcf(&cube)` (cube is now Cube<u8>)
  - Log BCF data size for debugging
  - Replace 3D texture with 1D byte buffer storage

### Phase 4: GPU Buffer Upload (SSBO or Texture Buffer)
- **`crates/renderer/src/gl_tracer.rs`**
  - **Option A (SSBO - preferred if available)**:
    - Create SSBO (Shader Storage Buffer Object) with `gl.create_buffer()`
    - Upload BCF data: `gl.buffer_data_u8_slice(SHADER_STORAGE_BUFFER, &bcf_data, STATIC_DRAW)`
    - Bind to shader: `gl.bind_buffer_base(SHADER_STORAGE_BUFFER, 0, Some(buffer))`

  - **Option B (Texture Buffer - fallback for WebGL 2)**:
    - Create buffer object: `gl.create_buffer()`
    - Upload BCF data: `gl.buffer_data_u8_slice(TEXTURE_BUFFER, &bcf_data, STATIC_DRAW)`
    - Create texture: `gl.create_texture()`
    - Bind buffer to texture: `gl.tex_buffer(TEXTURE_BUFFER, R8UI, Some(buffer))`
    - Sample in shader as `usamplerBuffer`

  - Detect SSBO support at init time and choose appropriate method
  - Store buffer/texture handles in `GlTracerGl` struct
  - Update uniform locations (remove `u_octree_texture`, add `u_octree_data_size`)

### Phase 5: Fragment Shader BCF Traversal
- **`crates/renderer/src/shaders/octree_raycast.frag`**
  - Replace 3D texture sampling with BCF byte buffer access
  - Implement BCF node parsing:
    - Read type byte at offset
    - Decode MSB, type ID, SSSS fields via bit operations
    - Handle inline leaf (0x00-0x7F), extended leaf (0x80-0x8F), octa-leaves (0x90-0x9F), octa-pointers (0xA0-0xAF)

  - Implement octree traversal using BCF structure:
    - Start at root offset (fixed at byte 12)
    - For each ray step, determine which octant to descend
    - Follow pointer chain through BCF nodes
    - Return material value when hitting solid leaf

  - Add BCF decoding helper functions:
    - `uint readU8(uint offset)` - read byte from buffer
    - `uint readPointer(uint offset, uint ssss)` - read 1/2/4/8 byte pointer
    - `uint decodeTypeByte(uint type_byte)` - extract MSB, type, size
    - `uint traverseBcf(vec3 ray_origin, vec3 ray_dir)` - main traversal

- **Shader Input Changes**:
  - Remove: `uniform sampler3D u_octree_texture`
  - Add: `buffer OctreeData { uint data[]; } octree_buffer;` (SSBO)
  - OR: `uniform usamplerBuffer u_octree_buffer;` (Texture Buffer)
  - Add: `uniform uint u_octree_data_size;` (for bounds checking)

### Phase 6: Testing and Validation
- **`crates/renderer/tests/gl_rendering_test.rs`** (if exists, else create)
  - Test BCF serialization produces non-empty data
  - Test buffer upload succeeds
  - Test rendered output is not black/empty
  - Compare GL output to CPU raytracer output (diff analysis)

- **Manual Testing**:
  - Run `just dev` or `cargo run --bin renderer`
  - Verify GL renderer shows colored voxels (6 colors: red, cyan, green, blue, white, yellow)
  - Verify no "Solid voxels: 0" message
  - Verify render matches CPU and GPU tracers

### Not Changed
- CPU raytracer (already working correctly)
- GPU compute shader raytracer (separate implementation)
- Material palette system (RGB color lookup)
- Camera and lighting systems
- Egui UI and dual-renderer app structure
- BCF format itself (already specified and tested)

### Explicitly Out of Scope (Future Work)
- **Quad support**: Cube::Quad variant not implemented in this change
- **Layers support**: Cube::Layers variant not implemented in this change
- **Current limitation**: Only Solid and Cubes (octree) variants supported
- **Workaround**: Quad/Layers convert to Solid(0) with warning log

## Impact

### Affected Specs
- **NEW**: `gl-renderer-bcf-integration` - GL renderer BCF serialization and GPU upload
- **DEPENDS ON**: `binary-cube-format` (phases 1-4 complete for Solid/Cubes variants - sufficient for this change)

### Dependencies
- **MINIMAL DEPENDENCIES**: Only requires BCF format for Solid/Cubes variants
  - `add-binary-cube-format` phases 1-4 already complete and tested
  - Provides all necessary serialization for current renderer usage
- **Does NOT require**:
  - Full BCF serializer with Quad/Layers support (phases 5-6 of `add-binary-cube-format`)
  - Completion of `reimplement-raycast` (CPU tracer already working)
  - Completion of `refactor-gl-hierarchical-traversal` (different approach)
- **Current renderer scope**: Only uses `Cube<i32>` with Cubes variant (via `create_octa_cube()` helper)

### Affected Code
- `crates/renderer/src/gl_tracer.rs` - Remove broken code, add BCF serialization
- `crates/renderer/src/shaders/octree_raycast.frag` - Rewrite to traverse BCF format
- `crates/renderer/Cargo.toml` - Verify BCF feature dependency
- `crates/renderer/tests/` - Add GL renderer validation tests

### Benefits
- **Fixes broken GL renderer** - primary goal
- **Matches other implementations** - CPU raytracer already uses octree directly
- **Smaller GPU memory** - BCF is 10-20x more compact than voxel grid
- **Accurate rendering** - preserves exact octree structure
- **GPU-friendly format** - simple bit operations, no complex sampling
- **Future-proof** - BCF supports arbitrary octree depths, not limited to 8×8×8

### Compatibility
- No API changes to renderer interface (`render()`, `render_with_camera()`)
- No changes to material system or color palette
- Fragment shader version remains OpenGL ES 3.0 / WebGL 2.0
- SSBO requires OpenGL ES 3.1+ or WebGL extension; fallback to texture buffer

### Breaking Changes
None - this is a bug fix that changes internal implementation only

### Success Criteria
- GL renderer displays non-empty output with correct colors
- BCF serialization produces valid binary data (logged size > 0)
- GPU buffer upload succeeds without errors
- Fragment shader traverses BCF without crashes
- Rendered output matches CPU raytracer (visual validation)
- All existing tests pass
- Code passes `cargo clippy` with no warnings
- Render times comparable to previous implementation (<5ms per frame)

### Migration Path
N/A - internal change only, no user-facing migration

## Implementation Status

### Current State (2025-11-26)
- ❌ GL renderer broken (renders empty/black)
- ❌ Voxel grid sampling uses wrong coordinate system
- ❌ All sampled positions invalid (0 solid voxels detected)
- ✅ BCF format implemented and tested
- ✅ CPU raytracer working correctly
- ✅ Material palette system functional

### Blocked By
- None - all dependencies exist

### Blocks
- Raycast validation testing (needs working GL renderer)
- Performance comparison between tracers
- Visual debugging of octree structure

### Timeline Estimate
- Phase 1-2: 30 minutes (cleanup + conversion helper)
- Phase 3-4: 1 hour (BCF serialization + GPU upload)
- Phase 5: 2-3 hours (shader rewrite and testing)
- Phase 6: 30 minutes (validation)
- **Total**: 4-5 hours

### Risk Assessment
**Medium Risk:**
- Fragment shader BCF traversal is complex (bit operations, pointer arithmetic)
- Need to handle both SSBO and texture buffer fallback
- Shader debugging is difficult (no stack traces)

**Mitigation:**
- Start with simple test cases (single solid cube)
- Add extensive logging/printf debugging in shader
- Reference BCF spec document for exact format
- Test incrementally (leaf nodes → octa-leaves → octa-pointers)
