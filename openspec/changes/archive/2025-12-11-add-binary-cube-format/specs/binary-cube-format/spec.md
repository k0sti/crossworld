## ADDED Requirements

### Requirement: Binary Format Structure
The binary cube format (BCF) SHALL use a byte-aligned structure with a header followed by a compact node array using single-byte type encoding.

#### Scenario: File header format
- **WHEN** a BCF file is created
- **THEN** it starts with an 8-byte header
- **AND** header contains magic bytes 'BCF1' (0x42, 0x43, 0x46, 0x31)
- **AND** header contains version byte (0x01 for initial version)
- **AND** header contains 3 reserved bytes (set to 0x00)

#### Scenario: Root node offset
- **WHEN** a BCF file is created
- **THEN** bytes 8-11 contain root node offset as 32-bit little-endian integer
- **AND** root node offset points to the first node in the node array
- **AND** offset is measured from the start of the file (byte 0)

#### Scenario: Node array layout
- **WHEN** a BCF file is created
- **THEN** node array starts at byte 12 (after header + root offset)
- **AND** nodes are stored contiguously in byte-aligned format
- **AND** each node starts with a single type byte encoding node type and size

### Requirement: Single-Byte Type Encoding
The binary format SHALL encode all node type and size information in a single byte with format [M|TTT|SSSS].

#### Scenario: Type byte format
- **WHEN** a node is encoded
- **THEN** the type byte has format `[M|TTT|SSSS]`
- **AND** M is the MSB (bit 7)
- **AND** TTT is the type ID (bits 6-4, 3 bits = 8 types)
- **AND** SSSS is the size/value field (bits 3-0, 4 bits = 16 values)

#### Scenario: Type byte decoding
- **WHEN** parsing a type byte
- **THEN** MSB is extracted as `type_byte >> 7`
- **AND** type ID is extracted as `(type_byte >> 4) & 0x07`
- **AND** size/value is extracted as `type_byte & 0x0F`

### Requirement: Inline Leaf Value Encoding
The binary format SHALL encode leaf values 0-127 directly in the type byte.

#### Scenario: Inline leaf encoding
- **WHEN** a Cube::Solid(value) where value ≤ 127 is serialized
- **THEN** it is encoded as a single byte `0VVVVVVV`
- **AND** MSB = 0, indicating inline value
- **AND** lower 7 bits contain the value directly
- **AND** total node size is 1 byte

#### Scenario: Inline leaf decoding
- **WHEN** parser encounters byte in range 0x00-0x7F
- **THEN** parser extracts value as `type_byte & 0x7F`
- **AND** parser constructs Cube::Solid(value)
- **AND** parser advances read position by 1 byte

#### Scenario: Inline leaf examples
- **WHEN** encoding specific values
- **THEN** value 0 encodes as 0x00
- **AND** value 42 encodes as 0x2A
- **AND** value 127 encodes as 0x7F

### Requirement: Extended Leaf Value Encoding
The binary format SHALL encode leaf values 128-255 using type 0 (extended leaf).

#### Scenario: Extended leaf encoding
- **WHEN** a Cube::Solid(value) where value ≥ 128 is serialized
- **THEN** it is encoded as type byte `0x80-0x8F` followed by value byte
- **AND** type byte is `0x80 | reserved_bits` (typically 0x80)
- **AND** lower 4 bits of type byte are reserved (typically 0)
- **AND** value byte contains the voxel value (128-255)
- **AND** total node size is 2 bytes

#### Scenario: Extended leaf decoding
- **WHEN** parser encounters byte in range 0x80-0x8F
- **THEN** parser identifies it as extended leaf (type 0)
- **AND** parser reads next byte as value
- **AND** parser constructs Cube::Solid(value)
- **AND** parser advances read position by 2 bytes

#### Scenario: Extended leaf examples
- **WHEN** encoding specific values
- **THEN** value 128 encodes as `[0x80, 0x80]`
- **AND** value 200 encodes as `[0x80, 0xC8]`
- **AND** value 255 encodes as `[0x80, 0xFF]`

### Requirement: Octa with Leaf Values Encoding
The binary format SHALL encode octrees with 8 solid children using type 1 (octa-with-leaves).

#### Scenario: Octa with leaves encoding
- **WHEN** a Cube::Cubes where all 8 children are Solid is serialized
- **THEN** it is encoded as type byte `0x90-0x9F` followed by 8 value bytes
- **AND** type byte is `0x90 | reserved_bits` (typically 0x90)
- **AND** lower 4 bits of type byte are reserved (typically 0)
- **AND** 8 value bytes follow in octant order (0-7)
- **AND** total node size is 9 bytes (1 type + 8 values)

#### Scenario: Octa with leaves decoding
- **WHEN** parser encounters byte in range 0x90-0x9F
- **THEN** parser identifies it as octa-with-leaves (type 1)
- **AND** parser reads next 8 bytes as leaf values
- **AND** parser constructs Cube::Cubes with 8 Solid children
- **AND** children are created in octant order (0-7)
- **AND** parser advances read position by 9 bytes

#### Scenario: Leaf optimization benefit
- **WHEN** serializing an octa with 8 leaf children
- **THEN** type 1 encoding uses 9 bytes total
- **AND** this is smaller than 8 separate leaf nodes (8-16 bytes)
- **AND** provides 11-44% space savings for leaf-heavy octrees

### Requirement: Octa with Pointers Encoding
The binary format SHALL encode octrees with pointer children using type 2 (octa-with-pointers) with uniform-sized pointers.

#### Scenario: Octa with pointers type byte
- **WHEN** a Cube::Cubes with non-leaf children is serialized
- **THEN** it is encoded with type byte `0xA0-0xAF`
- **AND** type byte is `0xA0 | SSSS`
- **AND** SSSS (bits 3-0) encodes pointer size
- **AND** SSSS values map to pointer sizes:
  - SSSS=0: 1-byte pointers (2^0 = 1)
  - SSSS=1: 2-byte pointers (2^1 = 2)
  - SSSS=2: 4-byte pointers (2^2 = 4)
  - SSSS=3: 8-byte pointers (2^3 = 8)

#### Scenario: Pointer size selection
- **WHEN** serializing an octa with pointer children
- **THEN** serializer calculates the maximum offset among all 8 children
- **AND** selects minimum SSSS to represent maximum offset:
  - SSSS=0 (1 byte) for offsets 0-255
  - SSSS=1 (2 bytes) for offsets 256-65535
  - SSSS=2 (4 bytes) for offsets 65536-4294967295
  - SSSS=3 (8 bytes) for offsets > 4294967295
- **AND** all 8 pointers are encoded with this uniform size

#### Scenario: Pointer encoding as little-endian
- **WHEN** pointers are written in octa-with-pointers node
- **THEN** each pointer is encoded as a little-endian integer
- **AND** pointer size is 2^SSSS bytes
- **AND** least significant byte is written first
- **AND** example: offset 0x1234 with SSSS=1 (2 bytes) is `[0x34, 0x12]`

#### Scenario: Array indexing with uniform pointer size
- **WHEN** parsing octa-with-pointers node
- **THEN** parser extracts SSSS from type byte as `type_byte & 0x0F`
- **AND** parser calculates pointer size as `2^SSSS` bytes
- **AND** parser calculates pointer[i] position as: `node_start + 1 + (i * 2^SSSS)`
- **AND** no sequential parsing required, enabling random access to any child pointer
- **AND** all pointers can be read as a contiguous array

#### Scenario: Octa with pointers examples
- **WHEN** encoding octas with different pointer sizes
- **THEN** type `0xA0` = 1-byte pointers, total 9 bytes (1 + 8*1)
- **AND** type `0xA1` = 2-byte pointers, total 17 bytes (1 + 8*2)
- **AND** type `0xA2` = 4-byte pointers, total 33 bytes (1 + 8*4)
- **AND** type `0xA3` = 8-byte pointers, total 65 bytes (1 + 8*8)

#### Scenario: Octa with pointers decoding
- **WHEN** parser encounters byte in range 0xA0-0xAF
- **THEN** parser identifies it as octa-with-pointers (type 2)
- **AND** parser extracts SSSS as `type_byte & 0x0F`
- **AND** parser calculates pointer size as `2^SSSS`
- **AND** parser reads 8 pointers as little-endian integers of pointer_size bytes each
- **AND** parser recursively loads each child node from its offset
- **AND** parser constructs Cube::Cubes with 8 child Cubes
- **AND** parser validates all offsets are within file bounds

### Requirement: Serialization Algorithm
The serializer SHALL traverse the Cube tree and generate a valid BCF binary representation.

#### Scenario: Single solid value serialization
- **WHEN** serializing Cube::Solid(42)
- **THEN** output is 13 bytes total
- **AND** bytes 0-7: header ('BCF1', version 0x01, reserved 0x00,0x00,0x00)
- **AND** bytes 8-11: root offset 0x0C (12 in little-endian)
- **AND** byte 12: 0x2A (inline leaf value 42)

#### Scenario: Extended leaf serialization
- **WHEN** serializing Cube::Solid(200)
- **THEN** output is 14 bytes total
- **AND** bytes 0-11: header + root offset
- **AND** bytes 12-13: `[0x80, 0xC8]` (extended leaf, value 200)

#### Scenario: Octa with leaf values serialization
- **WHEN** serializing Cube::Cubes with 8 Solid children [1,2,3,4,5,6,7,8]
- **THEN** output is 21 bytes total
- **AND** bytes 0-11: header + root offset
- **AND** bytes 12-20: `[0x90, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08]`
- **AND** serializer detects all children are leaves and uses type 1

#### Scenario: Nested octree serialization
- **WHEN** serializing a Cube::Cubes with mixed children (some Solid, some Cubes)
- **THEN** parent uses type 2 (octa-with-pointers)
- **AND** type byte includes SSSS indicating pointer size
- **AND** all 8 pointers are encoded with uniform size (little-endian)
- **AND** pointers reference child nodes stored later in the array
- **AND** child nodes are serialized recursively
- **AND** all pointer offsets are valid and within file bounds

#### Scenario: Depth-first traversal
- **WHEN** serializing a deep octree
- **THEN** serializer traverses depth-first or breadth-first (implementation choice)
- **AND** child nodes are written with correct offset calculation
- **AND** traversal order is deterministic and consistent

### Requirement: Deserialization Algorithm
The parser SHALL read BCF binary data and reconstruct the Cube tree correctly.

#### Scenario: Header validation
- **WHEN** parsing BCF data
- **THEN** parser validates magic bytes are 'BCF1'
- **AND** parser validates version byte is supported (0x01)
- **AND** parser returns InvalidMagic error if magic bytes don't match
- **AND** parser returns UnsupportedVersion error if version is unknown

#### Scenario: Root node loading
- **WHEN** parsing valid BCF data
- **THEN** parser reads root offset from bytes 8-11 (little-endian)
- **AND** parser jumps to root offset in the node array
- **AND** parser reads type byte at root offset
- **AND** parser decodes type byte to determine node type
- **AND** parser constructs root Cube based on node type

#### Scenario: Type byte parsing
- **WHEN** parser reads a type byte
- **THEN** parser checks MSB to distinguish inline leaf (MSB=0) from extended types (MSB=1)
- **AND** if MSB=1, parser extracts type ID from bits 6-4
- **AND** parser extracts size/value from bits 3-0
- **AND** parser uses decoded information to parse node data

### Requirement: Error Handling
The parser SHALL detect and report malformed BCF data with descriptive errors.

#### Scenario: Invalid magic number error
- **WHEN** parsing data that doesn't start with 'BCF1'
- **THEN** parser returns BcfError::InvalidMagic
- **AND** error message includes actual bytes found
- **AND** parsing stops immediately

#### Scenario: Unsupported version error
- **WHEN** parsing data with version byte > 0x01
- **THEN** parser returns BcfError::UnsupportedVersion(version)
- **AND** error message includes the unsupported version number

#### Scenario: Invalid type ID error
- **WHEN** parser encounters type byte with unknown type ID (types 3-7)
- **THEN** parser returns BcfError::InvalidTypeId(type_id, offset)
- **AND** error message includes the invalid type ID and file offset

#### Scenario: Invalid pointer size error
- **WHEN** parser reads SSSS value > 3 from octa-with-pointers type byte
- **THEN** parser returns BcfError::InvalidPointerSize(ssss, offset)
- **AND** error message includes the invalid SSSS value and file offset

#### Scenario: Truncated data error
- **WHEN** parser attempts to read beyond end of data
- **THEN** parser returns BcfError::TruncatedData(expected, actual)
- **AND** error message includes expected vs actual data length

#### Scenario: Invalid pointer offset error
- **WHEN** parser encounters pointer offset beyond file size
- **THEN** parser returns BcfError::InvalidOffset(offset, file_size)
- **AND** error message includes the invalid offset and file size

#### Scenario: Circular reference detection (optional)
- **WHEN** parser encounters pointer that creates a cycle
- **THEN** parser detects infinite recursion (via depth limit or visited set)
- **AND** parser returns BcfError::CircularReference
- **AND** parsing stops to prevent stack overflow

### Requirement: Round-Trip Correctness
The BCF format SHALL preserve exact Cube structure through serialize-deserialize cycles.

#### Scenario: Simple solid round-trip (inline)
- **WHEN** serializing then deserializing Cube::Solid(42)
- **THEN** deserialized cube equals original: Cube::Solid(42)
- **AND** no data is lost or corrupted

#### Scenario: Simple solid round-trip (extended)
- **WHEN** serializing then deserializing Cube::Solid(200)
- **THEN** deserialized cube equals original: Cube::Solid(200)
- **AND** no data is lost or corrupted

#### Scenario: Octa leaf round-trip
- **WHEN** serializing then deserializing Cube::Cubes with 8 Solid children
- **THEN** deserialized cube has identical structure
- **AND** all 8 child values match original values
- **AND** children are in correct octant order

#### Scenario: Deep octree round-trip
- **WHEN** serializing then deserializing a depth-3 octree
- **THEN** deserialized cube has identical tree structure
- **AND** all leaf values match original values
- **AND** all subdivision points match original structure

#### Scenario: Mixed octree round-trip
- **WHEN** serializing then deserializing octree with mixed node types
- **THEN** inline leaf nodes are preserved exactly
- **AND** extended leaf nodes are preserved exactly
- **AND** octa-with-leaves nodes are preserved exactly
- **AND** octa-with-pointers nodes are preserved with correct child references

### Requirement: Format Efficiency
The BCF format SHALL achieve significant size reduction compared to text-based CSM format through compact single-byte type encoding.

#### Scenario: File size comparison
- **WHEN** the same Cube is serialized to both BCF and CSM
- **THEN** BCF file is at least 10x smaller for typical models
- **AND** BCF overhead is minimal (12-byte header + compact node encoding)
- **AND** BCF has no redundant whitespace or syntax characters

#### Scenario: Inline leaf efficiency
- **WHEN** serializing leaf values 0-127
- **THEN** each value uses only 1 byte (inline encoding)
- **AND** this is the most compact possible encoding

#### Scenario: Leaf optimization benefit
- **WHEN** serializing octree with many leaf octas
- **THEN** type 1 encoding (9 bytes per leaf octa) reduces size
- **AND** vs 8-16 bytes for 8 separate leaf nodes
- **AND** provides 11-44% space savings for leaf-heavy octrees

#### Scenario: Pointer size optimization
- **WHEN** serializing small octrees with offsets < 256
- **THEN** uses 1-byte pointers (9 bytes per octa node)
- **AND** larger octrees automatically scale to 2, 4, or 8-byte pointers
- **AND** format adapts to file size without wasted space

### Requirement: Parsing Performance
The BCF parser SHALL be significantly faster than CSM text parsing through simple bit operations and direct memory access.

#### Scenario: Parsing speed comparison
- **WHEN** parsing the same model from BCF vs CSM
- **THEN** BCF parsing is at least 5x faster
- **AND** BCF parsing requires no text tokenization
- **AND** BCF parsing is mostly memcpy and pointer arithmetic

#### Scenario: Type decoding efficiency
- **WHEN** BCF data is parsed
- **THEN** type identification is single bit check (MSB)
- **AND** type extraction is single shift-and-mask operation
- **AND** size extraction is single bit-mask operation
- **AND** no complex state machines required

#### Scenario: Pointer decoding efficiency
- **WHEN** parsing octa-with-pointers nodes
- **THEN** pointer size is calculated as simple power-of-2 (2^SSSS)
- **AND** pointers are decoded as direct memory reads (little-endian)
- **AND** array indexing enables random access to any child pointer

### Requirement: Format Simplicity
The BCF format SHALL use a custom compact encoding without external dependencies.

#### Scenario: No external libraries required
- **WHEN** implementing BCF serializer/parser
- **THEN** no MessagePack library is required
- **AND** no CBOR library is required
- **AND** all encoding/decoding uses simple bit operations
- **AND** implementation is self-contained

#### Scenario: Simple bit operations
- **WHEN** encoding or decoding nodes
- **THEN** only basic bit operations are used: shift (>>), OR (|), AND (&)
- **AND** pointer size calculation uses power-of-2: `1 << SSSS`
- **AND** no complex algorithms required

#### Scenario: GPU-friendly format
- **WHEN** BCF data is used on GPU
- **THEN** byte-aligned format is GPU-compatible
- **AND** little-endian pointers match GPU architecture
- **AND** uniform pointer sizes enable parallel array access
- **AND** simple bit operations map to GPU instructions
