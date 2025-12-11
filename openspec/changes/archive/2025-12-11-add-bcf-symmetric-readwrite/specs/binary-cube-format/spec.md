# Binary Cube Format (BCF) Specification - Delta

## ADDED Requirements

### Requirement: BCF Round-Trip Serialization
The system SHALL provide deterministic serialization and deserialization for Cube<u8> structures such that multiple serialize-deserialize cycles produce identical binary output.

#### Scenario: Round-trip preserves structure
- **WHEN** a Cube<u8> is serialized to BCF binary format
- **AND** the binary data is deserialized back to Cube<u8>
- **AND** the result is serialized again
- **THEN** the second binary output SHALL be identical to the first binary output (byte-for-byte)

#### Scenario: Deterministic serialization
- **WHEN** the same Cube<u8> is serialized multiple times
- **THEN** all serialization attempts SHALL produce identical binary output
- **AND** the output SHALL be deterministic (same input always produces same bytes)

#### Scenario: Deep tree round-trip
- **WHEN** a Cube<u8> with depth ≥ 3 (512+ voxels) is serialized and deserialized
- **THEN** the resulting structure SHALL match the original exactly
- **AND** all leaf values SHALL be preserved
- **AND** all octree branching SHALL be preserved

#### Scenario: Pointer size optimization preserved
- **WHEN** a Cube<u8> with octa-pointers is serialized
- **THEN** the serializer SHALL choose the minimal pointer size (1, 2, 4, or 8 bytes)
- **AND** deserialization SHALL correctly handle all pointer sizes
- **AND** re-serialization SHALL produce identical pointer size choice

### Requirement: BCF Structural Equality
The system SHALL ensure that logically equivalent Cube<u8> structures produce identical BCF binary output.

#### Scenario: Equivalent cubes produce identical binary
- **WHEN** two Cube<u8> structures are structurally equal (cube1 == cube2)
- **THEN** serialize(cube1) SHALL equal serialize(cube2) byte-for-byte

#### Scenario: Different cubes produce different binary
- **WHEN** two Cube<u8> structures are not equal (cube1 != cube2)
- **THEN** serialize(cube1) SHALL NOT equal serialize(cube2)
- **AND** the binary difference SHALL be detectable

#### Scenario: Optimization correctness
- **WHEN** a Cube::Cubes octree has all-solid children
- **THEN** serializer MAY optimize to octa-leaves encoding (9 bytes)
- **AND** deserialization SHALL reconstruct equivalent Cube::Cubes structure
- **AND** logical equality SHALL be preserved (original == deserialized)

### Requirement: BCF Error Handling Validation
The system SHALL reject invalid BCF binary data with descriptive error messages.

#### Scenario: Invalid magic number rejection
- **WHEN** BCF data has incorrect magic number (not 0x31464342 = "BCF1")
- **THEN** parse_bcf SHALL return BcfError::InvalidMagic
- **AND** error message SHALL include expected and found values

#### Scenario: Unsupported version rejection
- **WHEN** BCF data has unsupported version number
- **THEN** parse_bcf SHALL return BcfError::UnsupportedVersion
- **AND** error message SHALL include the unsupported version number

#### Scenario: Truncated data rejection
- **WHEN** BCF data is incomplete (less bytes than header or nodes require)
- **THEN** parse_bcf SHALL return BcfError::TruncatedData
- **AND** error SHALL specify expected vs available bytes

#### Scenario: Invalid offset rejection
- **WHEN** root offset or child pointer points beyond file size
- **THEN** parse_bcf SHALL return BcfError::InvalidOffset
- **AND** error SHALL specify the invalid offset and file size

#### Scenario: Recursion limit protection
- **WHEN** BCF data contains excessively deep tree (>64 levels)
- **THEN** parse_bcf SHALL return BcfError::RecursionLimit
- **AND** prevent stack overflow from malicious data

#### Scenario: Zero-length buffer rejection
- **WHEN** parse_bcf receives empty buffer
- **THEN** SHALL return BcfError::TruncatedData
- **AND** error message SHALL be clear about minimum size requirement

### Requirement: BCF Comprehensive Test Coverage
The system SHALL include test suites validating all BCF serialization edge cases and error conditions.

#### Scenario: Inline leaf values (0-127)
- **WHEN** Cube::Solid(n) where n ≤ 127 is serialized
- **THEN** SHALL produce 1-byte inline encoding (0x00-0x7F)
- **AND** round-trip SHALL preserve exact value

#### Scenario: Extended leaf values (128-255)
- **WHEN** Cube::Solid(n) where n > 127 is serialized
- **THEN** SHALL produce 2-byte extended encoding (0x80 + value byte)
- **AND** round-trip SHALL preserve exact value

#### Scenario: Octa-leaves encoding
- **WHEN** Cube::Cubes with 8 Solid children is serialized
- **THEN** SHALL produce 9-byte octa-leaves encoding (0x90 + 8 value bytes)
- **AND** round-trip SHALL reconstruct Cube::Cubes structure
- **AND** all 8 leaf values SHALL be preserved

#### Scenario: Octa-pointers with 1-byte offsets
- **WHEN** Cube::Cubes with complex children is serialized
- **AND** all child offsets fit in 1 byte (max offset < 256)
- **THEN** SHALL use 1-byte pointers (type byte 0xA0)
- **AND** round-trip SHALL correctly follow all pointers

#### Scenario: Octa-pointers with 2-byte offsets
- **WHEN** Cube::Cubes with complex children is serialized
- **AND** some child offsets require 2 bytes (256 ≤ offset < 65536)
- **THEN** SHALL use 2-byte pointers (type byte 0xA1)
- **AND** round-trip SHALL preserve structure exactly

#### Scenario: Deep octree (depth 3+)
- **WHEN** Cube::Cubes tree with depth ≥ 3 is serialized
- **THEN** SHALL recursively encode all levels
- **AND** round-trip SHALL preserve exact tree structure
- **AND** all leaf values SHALL match original

#### Scenario: Checkerboard pattern
- **WHEN** Cube::Cubes with alternating 0/1 pattern is serialized
- **THEN** SHALL produce compact binary representation
- **AND** round-trip SHALL preserve exact voxel pattern

#### Scenario: All-same value octree
- **WHEN** Cube::Cubes where all leaves have same value is serialized
- **THEN** serializer MAY optimize structure
- **AND** round-trip SHALL preserve logical equivalence

#### Scenario: Empty cube (all zeros)
- **WHEN** Cube with all voxels = 0 is serialized
- **THEN** SHALL produce valid BCF binary
- **AND** round-trip SHALL preserve structure

#### Scenario: Max value cube (all 255)
- **WHEN** Cube with all voxels = 255 is serialized
- **THEN** SHALL use extended leaf encoding for all leaves
- **AND** round-trip SHALL preserve all 255 values

### Requirement: BCF API Documentation
The system SHALL provide comprehensive documentation for BCF serialization API with guarantees and limitations.

#### Scenario: Round-trip guarantee documented
- **WHEN** user reads `serialize_bcf` documentation
- **THEN** SHALL state: "deserialize(serialize(cube)) preserves structure exactly"
- **AND** SHALL provide example demonstrating round-trip

#### Scenario: Determinism guarantee documented
- **WHEN** user reads `serialize_bcf` documentation
- **THEN** SHALL state: "Multiple calls with same input produce identical bytes"
- **AND** SHALL explain why this matters (caching, comparison)

#### Scenario: Limitations documented
- **WHEN** user reads BCF module documentation
- **THEN** SHALL clearly state: "Quad and Layers variants serialize as Solid(0)"
- **AND** SHALL explain this is temporary limitation
- **AND** SHALL indicate when full support is planned

#### Scenario: Error handling documented
- **WHEN** user reads `parse_bcf` documentation
- **THEN** SHALL list all BcfError variants with descriptions
- **AND** SHALL provide example of error handling
- **AND** SHALL explain how to recover from errors

#### Scenario: Performance characteristics documented
- **WHEN** user reads BCF documentation
- **THEN** SHALL state time complexity: O(n) where n = node count
- **AND** SHALL state space complexity: O(n) in worst case
- **AND** SHALL mention typical compression ratio vs CSM format
