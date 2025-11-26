//! BCF binary reader with GPU-compatible operations
//!
//! This module provides a zero-allocation reader for BCF (Binary Cube Format) data.
//! All operations are designed to map directly to GLSL fragment shader code.
//!
//! Key design principles:
//! - No heap allocations (no Vec, Box, String)
//! - Explicit bounds checking on all reads
//! - Simple, predictable control flow (maps to GLSL if/else)
//! - Bit operations documented for GPU translation

use super::constants::*;
use super::BcfError;

/// BCF file header (12 bytes)
#[derive(Debug, Clone, Copy)]
pub struct BcfHeader {
    pub magic: u32,
    pub version: u8,
    pub root_offset: usize,
}

/// BCF node types (result of parsing a node)
#[derive(Debug, Clone)]
pub enum BcfNodeType {
    /// Inline leaf: type byte 0x00-0x7F, value encoded in lower 7 bits
    InlineLeaf(u8),
    /// Extended leaf: type byte 0x80-0x8F, followed by 1 value byte
    ExtendedLeaf(u8),
    /// Octa with 8 leaf values: type byte 0x90-0x9F, followed by 8 value bytes
    OctaLeaves([u8; 8]),
    /// Octa with pointers to children: type byte 0xA0-0xAF, followed by 8 pointers
    OctaPointers { ssss: u8, pointers: [usize; 8] },
}

/// Zero-allocation BCF binary reader
///
/// Designed for GPU translation - all operations use simple arithmetic and bit operations.
pub struct BcfReader<'a> {
    data: &'a [u8],
}

impl<'a> BcfReader<'a> {
    /// Create a new reader for BCF binary data
    pub fn new(data: &'a [u8]) -> Self {
        Self { data }
    }

    /// Read a single byte at offset (with bounds checking)
    ///
    /// GLSL equivalent: `uint read_u8(uint offset) { return octree_data[offset]; }`
    #[inline]
    pub fn read_u8(&self, offset: usize) -> Result<u8, BcfError> {
        if offset >= self.data.len() {
            return Err(BcfError::InvalidOffset {
                offset,
                file_size: self.data.len(),
            });
        }
        Ok(self.data[offset])
    }

    /// Read 2-byte little-endian unsigned integer
    ///
    /// GLSL equivalent:
    /// ```glsl
    /// uint read_u16_le(uint offset) {
    ///     uint b0 = octree_data[offset];
    ///     uint b1 = octree_data[offset + 1u];
    ///     return b0 | (b1 << 8u);
    /// }
    /// ```
    #[inline]
    pub fn read_u16_le(&self, offset: usize) -> Result<u16, BcfError> {
        if offset + 2 > self.data.len() {
            return Err(BcfError::TruncatedData {
                expected_bytes: offset + 2,
                available_bytes: self.data.len(),
            });
        }
        let b0 = self.data[offset] as u16;
        let b1 = self.data[offset + 1] as u16;
        Ok(b0 | (b1 << 8))
    }

    /// Read 4-byte little-endian unsigned integer
    ///
    /// GLSL equivalent:
    /// ```glsl
    /// uint read_u32_le(uint offset) {
    ///     uint b0 = octree_data[offset];
    ///     uint b1 = octree_data[offset + 1u];
    ///     uint b2 = octree_data[offset + 2u];
    ///     uint b3 = octree_data[offset + 3u];
    ///     return b0 | (b1 << 8u) | (b2 << 16u) | (b3 << 24u);
    /// }
    /// ```
    #[inline]
    pub fn read_u32_le(&self, offset: usize) -> Result<u32, BcfError> {
        if offset + 4 > self.data.len() {
            return Err(BcfError::TruncatedData {
                expected_bytes: offset + 4,
                available_bytes: self.data.len(),
            });
        }
        let b0 = self.data[offset] as u32;
        let b1 = self.data[offset + 1] as u32;
        let b2 = self.data[offset + 2] as u32;
        let b3 = self.data[offset + 3] as u32;
        Ok(b0 | (b1 << 8) | (b2 << 16) | (b3 << 24))
    }

    /// Read 8-byte little-endian unsigned integer
    ///
    /// GLSL note: GLSL ES 3.0 does not have 64-bit integers.
    /// For GPU, limit pointer sizes to u32 (ssss <= 2)
    #[inline]
    pub fn read_u64_le(&self, offset: usize) -> Result<u64, BcfError> {
        if offset + 8 > self.data.len() {
            return Err(BcfError::TruncatedData {
                expected_bytes: offset + 8,
                available_bytes: self.data.len(),
            });
        }
        let b0 = self.data[offset] as u64;
        let b1 = self.data[offset + 1] as u64;
        let b2 = self.data[offset + 2] as u64;
        let b3 = self.data[offset + 3] as u64;
        let b4 = self.data[offset + 4] as u64;
        let b5 = self.data[offset + 5] as u64;
        let b6 = self.data[offset + 6] as u64;
        let b7 = self.data[offset + 7] as u64;
        Ok(b0
            | (b1 << 8)
            | (b2 << 16)
            | (b3 << 24)
            | (b4 << 32)
            | (b5 << 40)
            | (b6 << 48)
            | (b7 << 56))
    }

    /// Read pointer of variable size (1, 2, 4, or 8 bytes) based on ssss bits
    ///
    /// GLSL equivalent:
    /// ```glsl
    /// uint read_pointer(uint offset, uint ssss) {
    ///     if (ssss == 0u) return read_u8(offset);
    ///     if (ssss == 1u) return read_u16_le(offset);
    ///     if (ssss == 2u) return read_u32_le(offset);
    ///     // ssss == 3u not supported in GLSL (no 64-bit)
    ///     return 0u;
    /// }
    /// ```
    #[inline]
    pub fn read_pointer(&self, offset: usize, ssss: u8) -> Result<usize, BcfError> {
        match ssss {
            0 => self.read_u8(offset).map(|v| v as usize),
            1 => self.read_u16_le(offset).map(|v| v as usize),
            2 => self.read_u32_le(offset).map(|v| v as usize),
            3 => self.read_u64_le(offset).map(|v| v as usize),
            _ => Err(BcfError::InvalidOffset {
                offset,
                file_size: self.data.len(),
            }),
        }
    }

    /// Decode BCF type byte into components using bit operations
    ///
    /// Returns: (is_extended, type_id, size_bits)
    /// - is_extended: true if MSB = 1 (extended node types)
    /// - type_id: bits 4-6 (3 bits) for extended types
    /// - size_bits: bits 0-3 (4 bits, SSSS field)
    ///
    /// GLSL equivalent:
    /// ```glsl
    /// void decode_type_byte(uint type_byte, out bool is_extended, out uint type_id, out uint size_bits) {
    ///     is_extended = (type_byte & 0x80u) != 0u;
    ///     type_id = (type_byte >> 4u) & 0x07u;
    ///     size_bits = type_byte & 0x0Fu;
    /// }
    /// ```
    #[inline]
    pub fn decode_type_byte(type_byte: u8) -> (bool, u8, u8) {
        let is_extended = (type_byte & 0x80) != 0;
        let type_id = (type_byte >> 4) & 0x07;
        let size_bits = type_byte & 0x0F;
        (is_extended, type_id, size_bits)
    }

    /// Read and parse BCF header (12 bytes)
    pub fn read_header(&self) -> Result<BcfHeader, BcfError> {
        if self.data.len() < HEADER_SIZE {
            return Err(BcfError::TruncatedData {
                expected_bytes: HEADER_SIZE,
                available_bytes: self.data.len(),
            });
        }

        // Read magic number (4 bytes, little-endian)
        let magic = self.read_u32_le(0)?;
        if magic != MAGIC {
            return Err(BcfError::InvalidMagic {
                expected: MAGIC,
                found: magic,
            });
        }

        // Read version (1 byte)
        let version = self.read_u8(4)?;
        if version != VERSION {
            return Err(BcfError::UnsupportedVersion { found: version });
        }

        // Reserved bytes at 5-7 (ignored)

        // Read root offset (4 bytes at offset 8, little-endian)
        let root_offset = self.read_u32_le(8)? as usize;

        // Validate root offset
        if root_offset >= self.data.len() {
            return Err(BcfError::InvalidOffset {
                offset: root_offset,
                file_size: self.data.len(),
            });
        }

        Ok(BcfHeader {
            magic,
            version,
            root_offset,
        })
    }

    /// Read and parse BCF node at given offset
    ///
    /// This is the core decoding logic that will map to GLSL.
    pub fn read_node_at(&self, offset: usize) -> Result<BcfNodeType, BcfError> {
        // Read type byte
        let type_byte = self.read_u8(offset)?;

        // Decode using bit operations
        let (is_extended, type_id, size_bits) = Self::decode_type_byte(type_byte);

        if !is_extended {
            // Inline leaf: type byte 0x00-0x7F
            // Value is encoded directly in lower 7 bits
            let value = type_byte & 0x7F;
            return Ok(BcfNodeType::InlineLeaf(value));
        }

        // Extended node types (MSB = 1)
        match type_id {
            0 => {
                // Extended leaf: 0x80-0x8F
                // Read 1 value byte
                let value = self.read_u8(offset + 1)?;
                Ok(BcfNodeType::ExtendedLeaf(value))
            }
            1 => {
                // Octa-leaves: 0x90-0x9F
                // Read 8 value bytes
                let mut values = [0u8; 8];
                for (i, value) in values.iter_mut().enumerate() {
                    *value = self.read_u8(offset + 1 + i)?;
                }
                Ok(BcfNodeType::OctaLeaves(values))
            }
            2 => {
                // Octa-pointers: 0xA0-0xAF
                // SSSS bits determine pointer size (2^size_bits bytes per pointer)
                let ssss = size_bits;
                let pointer_size = 1usize << ssss; // 2^ssss
                let mut pointers = [0usize; 8];
                for (i, pointer) in pointers.iter_mut().enumerate() {
                    let ptr_offset = offset + 1 + (i * pointer_size);
                    *pointer = self.read_pointer(ptr_offset, ssss)?;
                }
                Ok(BcfNodeType::OctaPointers { ssss, pointers })
            }
            _ => {
                // Unknown node type (3-7 reserved for future use)
                Err(BcfError::InvalidOffset {
                    offset,
                    file_size: self.data.len(),
                })
            }
        }
    }

    /// Get raw data slice (for accessing entire buffer)
    #[inline]
    pub fn data(&self) -> &[u8] {
        self.data
    }

    /// Get data length
    #[inline]
    pub fn len(&self) -> usize {
        self.data.len()
    }

    /// Check if data is empty
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decode_type_byte_inline_leaf() {
        let type_byte = 0x2A; // 0b00101010 = value 42
        let (is_extended, type_id, size_bits) = BcfReader::decode_type_byte(type_byte);
        assert!(!is_extended);
        assert_eq!(type_id, 2); // Upper 3 bits of lower nibble
        assert_eq!(size_bits, 10); // Lower 4 bits
        assert_eq!(type_byte & 0x7F, 42); // Actual value
    }

    #[test]
    fn test_decode_type_byte_extended_leaf() {
        let type_byte = 0x80; // Extended leaf base
        let (is_extended, type_id, size_bits) = BcfReader::decode_type_byte(type_byte);
        assert!(is_extended);
        assert_eq!(type_id, 0); // Type 0 = extended leaf
        assert_eq!(size_bits, 0);
    }

    #[test]
    fn test_decode_type_byte_octa_leaves() {
        let type_byte = 0x90; // Octa-leaves base
        let (is_extended, type_id, size_bits) = BcfReader::decode_type_byte(type_byte);
        assert!(is_extended);
        assert_eq!(type_id, 1); // Type 1 = octa-leaves
        assert_eq!(size_bits, 0);
    }

    #[test]
    fn test_decode_type_byte_octa_pointers() {
        let type_byte = 0xA2; // Octa-pointers with ssss=2 (4-byte pointers)
        let (is_extended, type_id, size_bits) = BcfReader::decode_type_byte(type_byte);
        assert!(is_extended);
        assert_eq!(type_id, 2); // Type 2 = octa-pointers
        assert_eq!(size_bits, 2); // SSSS = 2 (4-byte pointers)
    }

    #[test]
    fn test_read_u8() {
        let data = vec![0x42, 0x43, 0x46];
        let reader = BcfReader::new(&data);
        assert_eq!(reader.read_u8(0).unwrap(), 0x42);
        assert_eq!(reader.read_u8(1).unwrap(), 0x43);
        assert_eq!(reader.read_u8(2).unwrap(), 0x46);
        assert!(reader.read_u8(3).is_err()); // Out of bounds
    }

    #[test]
    fn test_read_u16_le() {
        let data = vec![0x34, 0x12]; // Little-endian 0x1234
        let reader = BcfReader::new(&data);
        assert_eq!(reader.read_u16_le(0).unwrap(), 0x1234);
    }

    #[test]
    fn test_read_u32_le() {
        let data = vec![0x78, 0x56, 0x34, 0x12]; // Little-endian 0x12345678
        let reader = BcfReader::new(&data);
        assert_eq!(reader.read_u32_le(0).unwrap(), 0x12345678);
    }

    #[test]
    fn test_read_pointer_1byte() {
        let data = vec![0x42];
        let reader = BcfReader::new(&data);
        assert_eq!(reader.read_pointer(0, 0).unwrap(), 0x42);
    }

    #[test]
    fn test_read_pointer_2byte() {
        let data = vec![0x34, 0x12];
        let reader = BcfReader::new(&data);
        assert_eq!(reader.read_pointer(0, 1).unwrap(), 0x1234);
    }

    #[test]
    fn test_read_pointer_4byte() {
        let data = vec![0x78, 0x56, 0x34, 0x12];
        let reader = BcfReader::new(&data);
        assert_eq!(reader.read_pointer(0, 2).unwrap(), 0x12345678);
    }
}
