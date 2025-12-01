# Change: Add Binary Serialization Format for Cube Structures

## Why

The existing CSM (Cube Script Model) format is text-based, which is human-readable and editable but inefficient for storage and network transmission. For GPU rendering, network synchronization, and file storage, we need a compact binary format that minimizes overhead.

**Current situation:**
- CSM text format exists for `Cube<u8>` structures
- Text parsing has overhead (CPU time, memory allocations)
- Large models result in large text files
- No efficient binary representation for GPU upload or network sync

**Impact:** Without a binary format, loading large voxel models is slow, file sizes are large, and GPU data upload requires conversion overhead.

**Scope:** Add a compact binary serialization format for `Cube<u8>` focusing on octree (Cubes variant) and leaf (Solid variant) structures. Format will be byte-aligned for easy parsing and GPU compatibility.

## What Changes

### Phase 1: Binary Format Specification
- **Single-byte type encoding** - All node information encoded in one byte:
  - **Format**: `[M|TTT|SSSS]` where M=MSB, TTT=type (3 bits), SSSS=size/value (4 bits)

  - **MSB = 0**: Inline leaf value (values 0-127)
    - `0VVVVVVV` - Value encoded directly in lower 7 bits
    - Example: `0x2A` = leaf value 42
    - Total size: 1 byte

  - **MSB = 1**: Extended node types
    - **Type 0** (`0x80-0x8F`): Extended leaf value (128-255)
      - `1000xxxx` followed by 1 value byte
      - xxxx ignored (reserved for future use)
      - Total size: 2 bytes

    - **Type 1** (`0x90-0x9F`): Octa with 8 leaf values
      - `1001xxxx` followed by 8 value bytes
      - xxxx ignored (reserved for future use)
      - Total size: 9 bytes (1 type + 8 values)

    - **Type 2** (`0xA0-0xAF`): Octa with pointers
      - `1010SSSS` followed by 8 pointers
      - SSSS encodes pointer size: 0→1 byte, 1→2 bytes, 2→4 bytes, 3→8 bytes
      - All 8 pointers use uniform size (2^SSSS bytes each)
      - Total size: 1 + (8 * 2^SSSS) bytes
      - Examples:
        - `0xA0` = 1-byte pointers (9 bytes total)
        - `0xA1` = 2-byte pointers (17 bytes total)
        - `0xA2` = 4-byte pointers (33 bytes total)
        - `0xA3` = 8-byte pointers (65 bytes total)

    - **Type 3** (`0xB0-0xBF`): Quad with 4 Cube children (+ axis)
      - `1011AAAA` where AAAA encodes axis (0=X, 1=Y, 2=Z, rest reserved)
      - Followed by 4 pointers to Cube<T> children
      - Pointer size determined by SSSS calculation (uniform for all 4)
      - Total size: 1 + axis encoding + 4 * pointer_size

    - **Type 4** (`0xC0-0xCF`): Layers with 2 Cube children (+ axis)
      - `1100AAAA` where AAAA encodes axis (0=X, 1=Y, 2=Z, rest reserved)
      - Followed by 2 pointers to Cube<T> children
      - Pointer size determined by SSSS calculation (uniform for both)
      - Total size: 1 + axis encoding + 2 * pointer_size

    - **Types 5-7** (`0xD0-0xFF`): Reserved for future use

- **Pointer encoding**:
  - All pointers are little-endian unsigned integers
  - Size determined by SSSS bits in type byte
  - All 8 pointers in a node use the same size for array indexing

- **File structure**:
  - Header: `[magic:4bytes]['B''C''F''1'][version:1byte][reserved:3bytes]` (8 bytes)
  - Root node offset: `[root_offset:4bytes]` (4 bytes)
  - Node array: byte-aligned sequence of nodes

### Phase 2: Serialization Implementation
- **`crates/cube/src/io/bcf/serializer.rs`** - Binary serialization
  - Implement `serialize_bcf(cube: &Cube<u8>) -> Vec<u8>`
  - Traverse octree depth-first or breadth-first
  - Encode leaf values:
    - Values 0-127: Single byte `0VVVVVVV`
    - Values 128-255: Type byte `0x80` + value byte
  - Detect leaf octas (all 8 children are Solid):
    - Encode as `[0x9X, val0, val1, ..., val7]` (9 bytes)
  - For octa-with-pointers:
    - Calculate maximum offset among all 8 children
    - Determine SSSS: 0 (1-byte), 1 (2-byte), 2 (4-byte), or 3 (8-byte) pointers
    - Encode type byte as `0xA0 | SSSS`
    - Write all 8 pointers as little-endian integers of 2^SSSS bytes each
  - Generate optimized binary output

### Phase 3: Deserialization Implementation
- **`crates/cube/src/io/bcf/parser.rs`** - Binary parsing
  - Implement `parse_bcf(data: &[u8]) -> Result<Cube<u8>, BcfError>`
  - Validate magic header and version
  - Parse node array starting from root offset
  - Decode type byte using bit operations:
    - Extract MSB: `type_byte >> 7`
    - Extract type: `(type_byte >> 4) & 0x07`
    - Extract size/value: `type_byte & 0x0F`
  - Parse based on type byte:
    - `0x00-0x7F`: Inline leaf value (value = type_byte & 0x7F)
    - `0x80-0x8F`: Extended leaf (read 1 value byte)
    - `0x90-0x9F`: Octa with 8 leaves (read 8 value bytes)
    - `0xA0-0xAF`: Octa with pointers (SSSS = type_byte & 0x0F, read 8*2^SSSS pointer bytes)
  - Decode pointers as little-endian integers
  - Reconstruct `Cube<u8>` tree from binary data
  - Handle malformed data gracefully with error types

### Phase 4: Integration
- **`crates/cube/src/io/bcf/mod.rs`** - Module exports
  - Re-export `serialize_bcf` and `parse_bcf`
  - Define `BcfError` error type
  - Add format documentation

- **`crates/cube/src/io/mod.rs`** - Expose BCF format
  - Add `pub mod bcf;` to io module
  - Make BCF format available alongside CSM

### Phase 5: Quad Node Type (NEW)
- **`crates/cube/src/io/bcf/serializer.rs`** - Add Quad support
  - Handle `Cube::Quad { axis, quads }` variant
  - Encode axis as 2 bits in type byte (X=0, Y=1, Z=2)
  - Serialize 4 Cube<T> children via pointers
  - Calculate pointer size for 4 children (using max offset)
  - Write type byte `0xB0 | (axis as u8)`
  - Write 4 pointers (uniform size)

- **`crates/cube/src/io/bcf/parser.rs`** - Parse Quad nodes
  - Detect type byte `0xB0-0xBF`
  - Extract axis from lower 4 bits
  - Read 4 pointers to child Cube nodes
  - Recursively parse 4 child Cubes
  - Reconstruct `Cube::Quad { axis, quads: [Rc<Cube<T>>; 4] }`

### Phase 6: Layers Node Type (NEW)
- **`crates/cube/src/io/bcf/serializer.rs`** - Add Layers support
  - Handle `Cube::Layers { axis, layers }` variant
  - Encode axis as 2 bits in type byte
  - Serialize 2 Cube<T> children via pointers (fixed count)
  - Calculate pointer size for 2 children (using max offset)
  - Write type byte `0xC0 | (axis as u8)`
  - Write 2 pointers (uniform size)

- **`crates/cube/src/io/bcf/parser.rs`** - Parse Layers nodes
  - Detect type byte `0xC0-0xCF`
  - Extract axis from lower 4 bits
  - Read 2 pointers to child Cube nodes (fixed count)
  - Recursively parse 2 child Cubes
  - Reconstruct `Cube::Layers { axis, layers: [Rc<Cube<T>>; 2] }`

### Not Changed
- CSM text format (remains available for human editing)
- `Cube<T>` enum structure (no API changes)
- Existing serialization/deserialization APIs

## Impact

### Affected Specs
- **NEW**: `binary-cube-format` - Binary serialization format specification

### Affected Code
- `crates/cube/src/io/bcf/` - New module for binary format
- `crates/cube/src/io/mod.rs` - Add bcf module export
- `crates/cube/tests/` - Add BCF round-trip tests
- No external dependencies required (custom compact encoding)

### Benefits
- **10-20x smaller file sizes** for typical models compared to CSM text
- **Faster parsing** - no text parsing overhead, direct memory mapping
- **GPU-friendly** - byte-aligned format suitable for GPU upload
- **Network efficiency** - smaller payloads for multiplayer sync
- **Zero-copy potential** - mmap-able format for large files

### Compatibility
- Coexists with CSM format (users choose based on use case)
- No breaking changes to existing APIs
- Can convert between BCF ↔ CSM via `Cube<u8>` in-memory representation

### Breaking Changes
None - binary format is additive

### Success Criteria
- BCF serialization produces valid binary output
- BCF deserialization reconstructs identical `Cube<u8>` tree
- Round-trip test: `cube -> BCF -> cube'` where `cube == cube'`
- File size reduction of 10x or more vs CSM for typical models
- Parsing speed improvement of 5x or more vs CSM
- All tests pass (`cargo test --workspace`)
- Code passes clippy with no warnings

## Implementation Status (2025-11-18)

### Completed (Phases 1-4)
- ✅ Binary format specification documented
- ✅ Module structure created (`crates/cube/src/io/bcf/`)
- ✅ Serialization for Solid and Cubes (octree) variants
- ✅ Deserialization for Solid and Cubes variants
- ✅ Error handling (BcfError with variants)
- ✅ Basic tests (9 tests passing):
  - Inline/extended leaf encoding
  - Octa-with-leaves encoding
  - Octa-with-pointers encoding
  - Round-trip tests
  - Error validation (invalid magic, version, truncated data)
- ✅ Documentation in `docs/bcf-format.md`
- ✅ Code quality (clippy, fmt, inline docs)

### Renderer Integration Status
- ✅ Renderer uses `Cube<i32>` successfully (octa cube scene)
- ✅ All three tracers (CPU, GL, GPU stub) render correctly
- ✅ Renderer tests pass (1 test in gl_tracer)
- ⚠️ BCF format NOT yet used in renderer (only CSM format)
- 📋 BCF integration is a future enhancement

### Pending (Phases 5-6)
- ⏳ Quad node type support (Type 3, 0xB0-0xBF)
  - Needs serialization of `Cube::Quad { axis, quads }`
  - Needs encoding of 4 Cube children with pointers
  - Requires tests for all axes and mixed children types
- ⏳ Layers node type support (Type 4, 0xC0-0xCF)
  - Needs serialization of `Cube::Layers { axis, layers }`
  - Needs fixed-length encoding of 2 Cube children
  - Requires tests for various children types
- ⏳ Extended testing for mixed node types
- ⏳ Performance benchmarks (BCF vs CSM comparison)

### Current Limitation
The serializer currently defaults Quad and Layers variants to `write_leaf(0)`
(see `serializer.rs:62`), meaning these node types are serialized as empty leaves
and lose their structure. This must be fixed before BCF can fully support the
complete Cube<T> enum.
