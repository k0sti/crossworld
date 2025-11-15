# Binary Cube Format (BCF) Specification

## Overview

BCF is a compact binary serialization format for `Cube<u8>` octree structures. It provides efficient storage and fast parsing compared to text-based formats like CSM.

## Design Goals

- **Compact**: Minimize file size through efficient encoding
- **Fast Parsing**: Simple binary format with minimal overhead
- **GPU-Friendly**: Byte-aligned for easy upload to GPU
- **Extensible**: Reserved type codes for future features

## File Structure

```
[Header: 12 bytes]
  - Magic number: 4 bytes ('BCF1' = 0x42 0x43 0x46 0x31)
  - Version: 1 byte (0x01 for initial version)
  - Reserved: 3 bytes (0x00 0x00 0x00)
  - Root offset: 4 bytes (little-endian uint32)

[Node Data: Variable length]
  - Root node at specified offset
  - Child nodes follow in depth-first order
```

## Type Byte Format

All node information is encoded in a single byte with format: `[M|TTT|SSSS]`

```
Bit 7 (M): Mode bit
  0 = Inline leaf value (bits 0-6 = value 0-127)
  1 = Extended node type (bits 0-6 = type and size)

Bits 4-6 (TTT): Type ID (when M=1)
  000 (0) = Extended leaf value (128-255)
  001 (1) = Octa with 8 leaf values
  010 (2) = Octa with pointers
  011-111 (3-7) = Reserved for future use

Bits 0-3 (SSSS): Size/value field
  When M=0: Not used (part of 7-bit value)
  When M=1, Type=0: Reserved
  When M=1, Type=1: Reserved
  When M=1, Type=2: Pointer size (0→1 byte, 1→2 bytes, 2→4 bytes, 3→8 bytes)
```

## Node Types

### Type 0: Inline Leaf Value (0x00-0x7F)

Encodes leaf values 0-127 in a single byte.

**Format:**
```
[0VVVVVVV]
```

**Example:**
- `0x2A` = leaf value 42
- `0x00` = leaf value 0 (empty voxel)
- `0x7F` = leaf value 127

**Size:** 1 byte

### Type 1: Extended Leaf Value (0x80-0x8F)

Encodes leaf values 128-255 in two bytes.

**Format:**
```
[1000xxxx] [value]
```

Where:
- First byte: `0x80-0x8F` (xxxx reserved, currently ignored)
- Second byte: Voxel value (128-255)

**Example:**
- `0x80 0x80` = leaf value 128
- `0x80 0xFF` = leaf value 255

**Size:** 2 bytes

### Type 2: Octa with 8 Leaf Values (0x90-0x9F)

Encodes an octree node where all 8 children are leaf values.

**Format:**
```
[1001xxxx] [v0] [v1] [v2] [v3] [v4] [v5] [v6] [v7]
```

Where:
- First byte: `0x90-0x9F` (xxxx reserved, currently ignored)
- Next 8 bytes: Values for octants 0-7

**Octant Ordering:**
```
Octant 0 (000): (-x, -y, -z)   Octant 4 (100): (+x, -y, -z)
Octant 1 (001): (-x, -y, +z)   Octant 5 (101): (+x, -y, +z)
Octant 2 (010): (-x, +y, -z)   Octant 6 (110): (+x, +y, -z)
Octant 3 (011): (-x, +y, +z)   Octant 7 (111): (+x, +y, +z)
```

**Example:**
```
0x90 0x01 0x02 0x03 0x04 0x05 0x06 0x07 0x08
  └─ Type byte
       └─────────────────────────────────┘
              8 leaf values (1-8)
```

**Size:** 9 bytes (1 type + 8 values)

### Type 3: Octa with Pointers (0xA0-0xAF)

Encodes an octree node where children are referenced by byte offsets.

**Format:**
```
[1010SSSS] [ptr0] [ptr1] [ptr2] [ptr3] [ptr4] [ptr5] [ptr6] [ptr7]
```

Where:
- First byte: `0xA0-0xAF` (SSSS encodes pointer size)
- Next 8×(2^SSSS) bytes: Pointers to child nodes

**Pointer Size Encoding (SSSS):**
- `0000` (0xA0): 1-byte pointers → 9 bytes total
- `0001` (0xA1): 2-byte pointers → 17 bytes total
- `0010` (0xA2): 4-byte pointers → 33 bytes total
- `0011` (0xA3): 8-byte pointers → 65 bytes total
- `0100-1111`: Reserved (invalid)

**Pointer Format:**
- All pointers are little-endian unsigned integers
- All 8 pointers in a node use the same size
- Pointers are byte offsets from start of file
- Zero pointer indicates empty/null child (future extension)

**Example (1-byte pointers):**
```
0xA0 0x10 0x20 0x30 0x40 0x50 0x60 0x70 0x80
  └─ Type (1-byte pointers)
       └──────────────────────────────────┘
            8 pointers (0x10, 0x20, ...)
```

**Size:** 1 + (8 × 2^SSSS) bytes

## Pointer Size Selection

The serializer automatically selects the smallest pointer size that can represent all child offsets in a node:

- **1-byte (SSSS=0):** Max offset ≤ 255 (0xFF)
- **2-byte (SSSS=1):** Max offset ≤ 65,535 (0xFFFF)
- **4-byte (SSSS=2):** Max offset ≤ 4,294,967,295 (0xFFFFFFFF)
- **8-byte (SSSS=3):** Max offset ≤ 18,446,744,073,709,551,615

## Serialization Algorithm

1. **Header Generation:**
   - Write magic number: `0x42 0x43 0x46 0x31` ('BCF1')
   - Write version: `0x01`
   - Write reserved bytes: `0x00 0x00 0x00`
   - Reserve space for root offset (to be filled later)

2. **Depth-First Traversal:**
   - Start at root node
   - For each node, determine optimal encoding:
     - `Cube::Solid(v)` where v ≤ 127 → Inline leaf (1 byte)
     - `Cube::Solid(v)` where v > 127 → Extended leaf (2 bytes)
     - `Cube::Cubes([children])` where all children are `Solid` → Octa-with-leaves (9 bytes)
     - `Cube::Cubes([children])` with mixed types → Octa-with-pointers
   - Write current node
   - Recursively process children (for octa-with-pointers)

3. **Pointer Size Calculation:**
   - Before writing octa-with-pointers node
   - Calculate or estimate maximum offset of all 8 children
   - Select smallest SSSS value that fits

4. **Offset Tracking:**
   - Track current write position
   - Record child offsets for pointer writing
   - Update root offset in header after writing

## Deserialization Algorithm

1. **Header Validation:**
   - Read and validate magic number
   - Read and validate version byte
   - Read root offset

2. **Recursive Parsing:**
   - Seek to root offset
   - Parse node at current position:
     - Read type byte
     - Extract M, TTT, SSSS bits
     - Based on type, read additional data
   - For octa-with-pointers, recursively parse children

3. **Type Byte Decoding:**
   ```rust
   let type_byte = read_u8()?;
   let msb = (type_byte & 0x80) != 0;

   if !msb {
       // Inline leaf: value = type_byte & 0x7F
       return Cube::Solid(type_byte & 0x7F);
   }

   let type_id = (type_byte >> 4) & 0x07;
   let size_field = type_byte & 0x0F;

   match type_id {
       0 => parse_extended_leaf(),
       1 => parse_octa_with_leaves(),
       2 => parse_octa_with_pointers(size_field),
       _ => Err(InvalidTypeId),
   }
   ```

## Error Handling

- **InvalidMagic**: Magic number doesn't match 'BCF1'
- **UnsupportedVersion**: Version byte is not 0x01
- **InvalidTypeId**: Type ID is 3-7 (reserved)
- **InvalidPointerSize**: SSSS > 3
- **TruncatedData**: Unexpected end of file
- **InvalidOffset**: Pointer points outside file bounds
- **RecursionLimit**: Octree too deep (prevent stack overflow)

## Examples

### Example 1: Single Solid Voxel (value 42)

```
File contents (13 bytes):
[Header: 12 bytes]
42 43 46 31  Magic 'BCF1'
01           Version 1
00 00 00     Reserved
0C 00 00 00  Root offset = 12 (little-endian)

[Root node: 1 byte]
2A           Inline leaf (value 42)
```

### Example 2: Octa with 8 Leaf Values

```
File contents (21 bytes):
[Header: 12 bytes]
42 43 46 31  Magic
01           Version
00 00 00     Reserved
0C 00 00 00  Root offset = 12

[Root node: 9 bytes]
90           Type: octa-with-leaves
01 02 03 04  Octants 0-3 (values 1-4)
05 06 07 08  Octants 4-7 (values 5-8)
```

### Example 3: Depth-2 Octree with Pointers

```
File contents (variable):
[Header: 12 bytes]
42 43 46 31  Magic
01           Version
00 00 00     Reserved
0C 00 00 00  Root offset = 12

[Root node at 0x0C: 9 bytes with 1-byte pointers]
A0           Type: octa-with-pointers (SSSS=0, 1-byte)
15 00 00 00  Octants 0-3 pointers (0x15, 0x00, 0x00, 0x00)
00 00 00 00  Octants 4-7 pointers (all 0x00 = empty)

[Child at 0x15: 9 bytes]
90           Type: octa-with-leaves
0A 0B 0C 0D  Values for octants 0-3
0E 0F 10 11  Values for octants 4-7
```

## Version History

### Version 1 (0x01)
- Initial release
- Support for inline leaves, extended leaves, octa-with-leaves, octa-with-pointers
- Little-endian pointer encoding
- Reserved types 3-7 for future extensions

## Future Extensions

Potential uses for reserved type IDs (3-7):

- **Type 3**: Run-length encoding for solid regions
- **Type 4**: Compressed node (zlib, lz4)
- **Type 5**: Material/color palette reference
- **Type 6**: Metadata (names, properties)
- **Type 7**: Sparse octree (bitmask for present children)

## Advantages over CSM

1. **File Size:** 60-80% smaller for typical models
2. **Parse Speed:** 5-10x faster (binary vs text)
3. **Memory Efficiency:** Direct deserialization without string allocations
4. **GPU Upload:** Can be mapped directly to GPU buffers
5. **Validation:** Built-in format validation via magic number

## Limitations

1. **Not Human-Readable:** Requires tools to inspect/edit
2. **u8 Only:** Currently only supports `Cube<u8>` (not i32 or other types)
3. **No Compression:** Raw binary (compression can be added externally)
4. **Limited Metadata:** No support for names, comments, or properties (yet)
