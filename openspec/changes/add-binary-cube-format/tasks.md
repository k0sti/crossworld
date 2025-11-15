## 1. Format Specification
- [x] 1.1 Document binary format layout (header, node types, single-byte encoding)
- [x] 1.2 Define magic number ('BCF1' = 0x42434631)
- [x] 1.3 Specify version byte (initial version = 0x01)
- [x] 1.4 Document type byte format [M|TTT|SSSS]
- [x] 1.5 Define type IDs: 0=extended leaf, 1=octa-with-leaves, 2=octa-with-pointers
- [x] 1.6 Define pointer size encoding: SSSS → 2^SSSS bytes (0→1, 1→2, 2→4, 3→8)
- [x] 1.7 Document endianness (little-endian for pointers)

## 2. Module Structure
- [x] 2.1 Create `crates/cube/src/io/bcf/` directory
- [x] 2.2 Create `crates/cube/src/io/bcf/mod.rs` with module exports
- [x] 2.3 Create `crates/cube/src/io/bcf/serializer.rs` skeleton
- [x] 2.4 Create `crates/cube/src/io/bcf/parser.rs` skeleton
- [x] 2.5 Define `BcfError` error type
- [x] 2.6 Add `pub mod bcf;` to `crates/cube/src/io/mod.rs`

## 3. Serialization Implementation
- [x] 3.1 Implement header writing (magic + version + root offset)
- [x] 3.2 Implement depth-first octree traversal
- [x] 3.3 Detect leaf octas (all 8 children are Solid values)
- [x] 3.4 Implement inline leaf encoding (values 0-127 as single byte)
- [x] 3.5 Implement extended leaf encoding (values 128-255 as type 0x80 + value byte)
- [x] 3.6 Implement octa-with-leaves encoding (type 0x90 + 8 value bytes)
- [x] 3.7 Calculate maximum offset for octa node to determine SSSS
- [x] 3.8 Implement octa-with-pointers encoding (type 0xA0|SSSS + pointers)
- [x] 3.9 Write pointers as little-endian integers of 2^SSSS bytes each
- [x] 3.10 Ensure all 8 pointers in a node use the same size
- [x] 3.11 Calculate and write node offsets correctly
- [x] 3.12 Add `serialize_bcf(cube: &Cube<u8>) -> Vec<u8>` public function

## 4. Deserialization Implementation
- [x] 4.1 Implement header validation (magic number check)
- [x] 4.2 Validate version byte
- [x] 4.3 Read root node offset (little-endian)
- [x] 4.4 Implement type byte decoding (extract MSB, type ID, size/value)
- [x] 4.5 Parse inline leaf values (0x00-0x7F) using `type_byte & 0x7F`
- [x] 4.6 Parse extended leaf values (0x80-0x8F) by reading next byte
- [x] 4.7 Parse octa-with-leaf-values (0x90-0x9F) by reading 8 bytes
- [x] 4.8 Parse octa-with-pointers (0xA0-0xAF) by extracting SSSS
- [x] 4.9 Calculate pointer size as `2^SSSS`
- [x] 4.10 Read 8 pointers as little-endian integers of pointer_size bytes each
- [x] 4.11 Parse octa with pointers (recursive node loading)
- [x] 4.12 Validate pointer offsets (bounds checking)
- [x] 4.13 Handle malformed data with errors
- [x] 4.14 Add `parse_bcf(data: &[u8]) -> Result<Cube<u8>, BcfError>` public function

## 5. Error Handling
- [x] 5.1 Define error variants (InvalidMagic, UnsupportedVersion, etc.)
- [x] 5.2 Add InvalidTypeId error variant
- [x] 5.3 Add InvalidPointerSize error variant (SSSS > 3)
- [x] 5.4 Add offset bounds checking errors
- [x] 5.5 Add truncated data errors
- [x] 5.6 Implement Display trait for BcfError
- [x] 5.7 Add context to errors (offset, expected vs actual)

## 6. Testing
- [x] 6.1 Create `crates/cube/tests/bcf_tests.rs`
- [x] 6.2 Test inline leaf encoding/decoding (values 0-127)
- [x] 6.3 Test extended leaf encoding/decoding (values 128-255)
- [x] 6.4 Test octa-with-leaves encoding/decoding
- [ ] 6.5 Test octa-with-pointers encoding/decoding (SSSS=0, 1-byte pointers)
- [ ] 6.6 Test octa-with-pointers encoding/decoding (SSSS=1, 2-byte pointers)
- [ ] 6.7 Test octa-with-pointers encoding/decoding (SSSS=2, 4-byte pointers)
- [ ] 6.8 Test octa-with-pointers encoding/decoding (SSSS=3, 8-byte pointers)
- [x] 6.9 Test round-trip: single solid value (0-127)
- [x] 6.10 Test round-trip: single solid value (128-255)
- [x] 6.11 Test round-trip: octa with 8 leaf values
- [x] 6.12 Test round-trip: octa with mixed children (pointers)
- [ ] 6.13 Test round-trip: deep octree (depth 3+)
- [ ] 6.14 Test pointer size selection (verify correct SSSS for various offset ranges)
- [x] 6.15 Test error: invalid magic number
- [x] 6.16 Test error: invalid version
- [ ] 6.17 Test error: invalid type ID (types 3-7)
- [ ] 6.18 Test error: invalid SSSS value (> 3)
- [x] 6.19 Test error: truncated data
- [ ] 6.20 Test error: invalid pointer offset
- [ ] 6.21 Test bit operations (type byte encoding/decoding)
- [ ] 6.22 Compare file sizes: BCF vs CSM for same model
- [ ] 6.23 Benchmark parsing speed: BCF vs CSM

## 7. Documentation
- [x] 7.1 Add format documentation to `docs/bcf-format.md`
- [x] 7.2 Document binary layout with diagrams
- [x] 7.3 Document type byte format [M|TTT|SSSS] with examples
- [x] 7.4 Add usage examples (serialize/deserialize)
- [x] 7.5 Document format version history
- [x] 7.6 Add code comments to serializer and parser
- [x] 7.7 Document type IDs and reserved types

## 8. Code Quality
- [x] 8.1 Run `cargo clippy --workspace` and fix warnings
- [x] 8.2 Run `cargo fmt --workspace`
- [x] 8.3 Run `cargo test --workspace` and ensure all tests pass
- [x] 8.4 Add inline documentation for public functions
- [ ] 8.5 Optimize hot paths (pointer encoding/decoding, bit operations)
