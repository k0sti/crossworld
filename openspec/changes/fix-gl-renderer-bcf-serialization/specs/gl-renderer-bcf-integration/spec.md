## ADDED Requirements

### Requirement: BCF Serialization for GL Renderer
The GL renderer SHALL serialize Cube octree data to Binary Cube Format (BCF) instead of sampling voxel positions.

#### Scenario: Cube<i32> to Cube<u8> conversion (Simplified - Solid and Cubes only)
- **WHEN** GL renderer receives a Cube<i32> octree
- **THEN** it converts all Solid material values to u8 range
- **AND** values are clamped to 0-255
- **AND** R2G3B2 encoded colors (128-255) are preserved correctly
- **AND** Cubes variant is recursively converted (all 8 children)
- **AND** Planes and Slices variants return Solid(0) with warning log
- **AND** conversion handles only octree structures (not quad-trees or layer stacks)

#### Scenario: BCF serialization
- **WHEN** GL renderer initializes with octree data
- **THEN** it calls `cube::io::bcf::serialize_bcf(&cube_u8)`
- **AND** serialization produces valid BCF binary data
- **AND** BCF data size is logged for debugging
- **AND** BCF data is non-empty (size > 12 bytes for header)

#### Scenario: Removal of broken voxel sampling
- **WHEN** GL renderer is refactored
- **THEN** `sample_cube_at_position()` function is completely removed
- **AND** `create_octree_texture()` method is completely removed
- **AND** no code remains that samples octree positions to create voxel grids
- **AND** no code uses incorrect [0,8) coordinate system

### Requirement: GPU Buffer Upload
The GL renderer SHALL upload BCF binary data to GPU-accessible storage.

#### Scenario: SSBO support detection
- **WHEN** GL renderer initializes
- **THEN** it detects if SSBO (Shader Storage Buffer Object) is available
- **AND** checks for OpenGL ES 3.1+ or WebGL extension support
- **AND** selects SSBO if available, otherwise falls back to texture buffer

#### Scenario: SSBO upload (preferred)
- **WHEN** SSBO is available
- **THEN** GL renderer creates buffer with `gl.create_buffer()`
- **AND** binds buffer to `SHADER_STORAGE_BUFFER` target
- **AND** uploads BCF data with `gl.buffer_data_u8_slice(SHADER_STORAGE_BUFFER, &bcf_data, STATIC_DRAW)`
- **AND** binds buffer to binding point 0 with `gl.bind_buffer_base(SHADER_STORAGE_BUFFER, 0, Some(buffer))`
- **AND** buffer handle is stored in `GlTracerGl` struct

#### Scenario: Texture buffer fallback (WebGL 2.0)
- **WHEN** SSBO is not available
- **THEN** GL renderer creates buffer object
- **AND** uploads BCF data to `TEXTURE_BUFFER` target
- **AND** creates 1D texture with `gl.create_texture()`
- **AND** binds buffer to texture with `gl.tex_buffer(TEXTURE_BUFFER, R8UI, Some(buffer))`
- **AND** texture is sampled in shader as `usamplerBuffer`

#### Scenario: Buffer cleanup
- **WHEN** GL renderer is destroyed
- **THEN** it deletes buffer object with `gl.delete_buffer()`
- **AND** if using texture buffer, also deletes texture with `gl.delete_texture()`
- **AND** no GPU memory leaks occur

###Requirement: Fragment Shader BCF Traversal
The fragment shader SHALL traverse the BCF octree structure to determine ray-voxel intersections.

#### Scenario: BCF buffer access in shader
- **WHEN** fragment shader is compiled
- **THEN** it declares SSBO: `buffer OctreeData { uint data[]; } octree_buffer;`
- **OR** declares texture buffer: `uniform usamplerBuffer u_octree_buffer;`
- **AND** declares uniform: `uniform uint u_octree_data_size;`
- **AND** removes old uniform: `uniform sampler3D u_octree_texture`

#### Scenario: BCF type byte decoding
- **WHEN** shader reads a BCF node
- **THEN** it reads type byte at current offset
- **AND** extracts MSB with `(type_byte >> 7) & 1u`
- **AND** extracts type ID with `(type_byte >> 4) & 7u`
- **AND** extracts size/value with `type_byte & 15u`
- **AND** correctly identifies inline leaf (MSB=0), extended types (MSB=1)

#### Scenario: Inline leaf decoding (0x00-0x7F)
- **WHEN** shader encounters type byte with MSB=0
- **THEN** it extracts value as `type_byte & 0x7F`
- **AND** treats this as solid leaf material value
- **AND** advances read position by 1 byte
- **AND** returns material value for rendering

#### Scenario: Extended leaf decoding (0x80-0x8F)
- **WHEN** shader encounters type byte 0x80-0x8F
- **THEN** it identifies as extended leaf (type ID 0)
- **AND** reads next byte as material value
- **AND** advances read position by 2 bytes total
- **AND** returns material value for rendering

#### Scenario: Octa-with-leaves decoding (0x90-0x9F)
- **WHEN** shader encounters type byte 0x90-0x9F
- **THEN** it identifies as octa-with-leaves (type ID 1)
- **AND** reads next 8 bytes as leaf values for octants 0-7
- **AND** determines which octant the ray enters based on ray direction
- **AND** returns corresponding octant's material value
- **AND** advances read position by 9 bytes total

#### Scenario: Octa-with-pointers decoding (0xA0-0xAF)
- **WHEN** shader encounters type byte 0xA0-0xAF
- **THEN** it identifies as octa-with-pointers (type ID 2)
- **AND** extracts SSSS bits as `type_byte & 0x0F`
- **AND** calculates pointer size as `1 << ssss` bytes (2^SSSS)
- **AND** reads 8 pointers of uniform size (little-endian)
- **AND** determines which octant the ray enters
- **AND** follows corresponding pointer to child node
- **AND** recursively traverses child node

#### Scenario: Pointer reading (little-endian)
- **WHEN** shader reads a pointer at given offset with given size
- **THEN** for 1-byte pointer: reads single byte
- **AND** for 2-byte pointer: combines 2 bytes as `byte0 | (byte1 << 8)`
- **AND** for 4-byte pointer: combines 4 bytes as little-endian uint
- **AND** for 8-byte pointer: combines 8 bytes (or uses lower 4 bytes if 64-bit unsupported)
- **AND** returned offset points to child node in BCF data

#### Scenario: Octree traversal algorithm
- **WHEN** shader performs raycast
- **THEN** it starts at root node (fixed offset 12 in BCF)
- **AND** for each ray step, calculates which octant ray enters
- **AND** descends into corresponding child node via pointer
- **AND** continues until hitting solid leaf node
- **AND** returns material value of hit leaf
- **AND** handles miss case (ray exits cube bounds)

#### Scenario: Bounds checking
- **WHEN** shader reads from BCF buffer
- **THEN** it validates offset is within `u_octree_data_size`
- **AND** returns error value (0 or sentinel) if out of bounds
- **AND** prevents crashes from invalid pointers
- **AND** logs error in debug builds (if shader printf available)

### Requirement: Rendering Correctness
The GL renderer SHALL produce visually correct output that matches the CPU raytracer.

#### Scenario: Non-empty output
- **WHEN** GL renderer is run with octa cube scene
- **THEN** it produces non-black, non-empty rendered image
- **AND** output contains at least 6 distinct colors (red, cyan, green, blue, white, yellow)
- **AND** does NOT log "Solid voxels: 0 (0.0%)"
- **AND** logs "BCF data size: X bytes" where X > 12

#### Scenario: Visual comparison with CPU raytracer
- **WHEN** GL and CPU raytracers render same scene with same camera
- **THEN** both outputs show similar voxel structure
- **AND** colors match within tolerance (allowing for lighting differences)
- **AND** solid/empty regions match between renders
- **AND** no major visual artifacts in GL render

#### Scenario: Material value preservation
- **WHEN** octree contains R2G3B2 encoded colors (128-255 range)
- **THEN** GL renderer preserves exact color values
- **AND** material palette lookup produces correct RGB colors
- **AND** rendered colors match CPU raytracer output

#### Scenario: Performance
- **WHEN** GL renderer renders a frame
- **THEN** render time is less than 5ms per frame (60+ FPS capable)
- **AND** performance is comparable to previous (broken) implementation
- **AND** BCF traversal overhead is minimal

### Requirement: Error Handling and Debugging
The GL renderer SHALL provide clear error messages and debugging information.

#### Scenario: BCF serialization failure
- **WHEN** BCF serialization fails (edge case)
- **THEN** GL renderer logs error message with details
- **AND** initialization returns error Result
- **AND** does not crash or produce undefined behavior

#### Scenario: Buffer upload failure
- **WHEN** GPU buffer creation or upload fails
- **THEN** GL renderer logs OpenGL error code
- **AND** logs BCF data size attempted
- **AND** initialization returns error Result
- **AND** cleans up any partially created resources

#### Scenario: Shader compilation error
- **WHEN** fragment shader fails to compile
- **THEN** GL renderer logs shader source with line numbers
- **AND** logs compiler error messages
- **AND** indicates which shader stage failed (vertex vs fragment)
- **AND** initialization returns error Result

#### Scenario: Debug logging
- **WHEN** GL renderer initializes successfully
- **THEN** it logs "[GL Tracer] BCF data serialized: X bytes"
- **AND** logs buffer/texture creation method (SSBO vs texture buffer)
- **AND** logs shader uniform locations
- **AND** provides visibility into initialization process

### Requirement: Code Quality
The GL renderer implementation SHALL meet project quality standards.

#### Scenario: No broken code remains
- **WHEN** refactoring is complete
- **THEN** all voxel grid sampling code is removed
- **AND** no dead code or commented-out blocks remain
- **AND** no references to `sample_cube_at_position()` exist
- **AND** no references to incorrect [0,8) coordinate system exist

#### Scenario: Type safety
- **WHEN** converting Cube<i32> to Cube<u8>
- **THEN** conversion function has proper error handling
- **AND** out-of-range values are clamped with logging
- **AND** no unsafe transmutes or unchecked casts

#### Scenario: Clippy compliance
- **WHEN** running `cargo clippy --workspace`
- **THEN** all GL renderer code passes with no warnings
- **AND** no unnecessary allocations or copies
- **AND** proper use of Result types for error handling

#### Scenario: Testing
- **WHEN** implementation is complete
- **THEN** automated test verifies BCF serialization produces non-empty data
- **AND** test verifies buffer upload succeeds
- **AND** test verifies rendered output is not black
- **AND** test can run in CI without GPU (using headless GL context)
