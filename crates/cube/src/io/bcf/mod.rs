//! Binary Cube Format (BCF) - Compact binary serialization for octrees
//!
//! BCF provides efficient binary encoding for `Cube<u8>` structures with
//! optimized size and fast parsing compared to text-based formats.
//!
//! # Format Overview
//!
//! - **Single-byte type encoding**: All node info in one byte `[M|TTT|SSSS]`
//! - **Inline leaf values**: Values 0-127 encoded directly (1 byte)
//! - **Extended leaves**: Values 128-255 encoded with 2 bytes
//! - **Octa-with-leaves**: 8 leaf values packed together (9 bytes)
//! - **Octa-with-pointers**: Variable-size pointers (1/2/4/8 bytes each)
//!
//! # Example
//!
//! ```
//! use cube::{Cube, io::bcf::{serialize_bcf, parse_bcf}};
//!
//! let cube = Cube::Solid(42u8);
//! let binary_data = serialize_bcf(&cube);
//! let parsed_cube = parse_bcf(&binary_data).unwrap();
//! assert_eq!(cube, parsed_cube);
//! ```
//!
//! # File Format
//!
//! ```text
//! [Header: 12 bytes]
//!   Magic: 'BCF1' (0x42434631)
//!   Version: 0x01
//!   Reserved: 3 bytes
//!   Root offset: 4 bytes (little-endian)
//!
//! [Node data: variable]
//!   Depth-first encoding of octree nodes
//! ```
//!
//! See `docs/bcf-format.md` for complete specification.

pub mod parser;
pub mod serializer;

use std::fmt;

/// BCF format errors
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BcfError {
    /// Magic number doesn't match 'BCF1' (0x42434631)
    InvalidMagic { expected: u32, found: u32 },

    /// Version byte is not supported
    UnsupportedVersion { found: u8 },

    /// Invalid type ID (types 3-7 are reserved)
    InvalidTypeId { type_id: u8 },

    /// Invalid pointer size (SSSS > 3)
    InvalidPointerSize { ssss: u8 },

    /// Unexpected end of file / truncated data
    TruncatedData {
        expected_bytes: usize,
        available_bytes: usize,
    },

    /// Pointer offset is outside file bounds
    InvalidOffset { offset: usize, file_size: usize },

    /// Octree recursion too deep (prevent stack overflow)
    RecursionLimit { max_depth: usize },

    /// Generic I/O error
    Io(String),
}

impl fmt::Display for BcfError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BcfError::InvalidMagic { expected, found } => {
                write!(
                    f,
                    "Invalid BCF magic number: expected 0x{:08X}, found 0x{:08X}",
                    expected, found
                )
            }
            BcfError::UnsupportedVersion { found } => {
                write!(f, "Unsupported BCF version: 0x{:02X}", found)
            }
            BcfError::InvalidTypeId { type_id } => {
                write!(f, "Invalid type ID: {} (reserved types 3-7)", type_id)
            }
            BcfError::InvalidPointerSize { ssss } => {
                write!(f, "Invalid pointer size: SSSS={} (must be 0-3)", ssss)
            }
            BcfError::TruncatedData {
                expected_bytes,
                available_bytes,
            } => {
                write!(
                    f,
                    "Truncated data: expected {} bytes, only {} available",
                    expected_bytes, available_bytes
                )
            }
            BcfError::InvalidOffset { offset, file_size } => {
                write!(
                    f,
                    "Invalid offset: {} is outside file bounds (size: {})",
                    offset, file_size
                )
            }
            BcfError::RecursionLimit { max_depth } => {
                write!(f, "Recursion limit exceeded: max depth is {}", max_depth)
            }
            BcfError::Io(msg) => write!(f, "I/O error: {}", msg),
        }
    }
}

impl std::error::Error for BcfError {}

/// BCF file format constants
pub mod constants {
    /// Magic number: 'BCF1' in ASCII
    pub const MAGIC: u32 = 0x42434631;

    /// Current format version
    pub const VERSION: u8 = 0x01;

    /// Header size in bytes
    pub const HEADER_SIZE: usize = 12;

    /// Maximum recursion depth to prevent stack overflow
    pub const MAX_RECURSION_DEPTH: usize = 64;

    // Type byte bit masks
    pub const MSB_MASK: u8 = 0x80; // Bit 7
    pub const TYPE_MASK: u8 = 0x70; // Bits 4-6
    pub const SIZE_MASK: u8 = 0x0F; // Bits 0-3
    pub const VALUE_MASK: u8 = 0x7F; // Bits 0-6 (for inline leaves)

    // Type IDs (when MSB=1)
    pub const TYPE_EXTENDED_LEAF: u8 = 0; // 0x80-0x8F
    pub const TYPE_OCTA_LEAVES: u8 = 1; // 0x90-0x9F
    pub const TYPE_OCTA_POINTERS: u8 = 2; // 0xA0-0xAF

    // Type byte patterns
    pub const EXTENDED_LEAF_BASE: u8 = 0x80; // Type 0: 10000000
    pub const OCTA_LEAVES_BASE: u8 = 0x90; // Type 1: 10010000
    pub const OCTA_POINTERS_BASE: u8 = 0xA0; // Type 2: 10100000
}

// Re-export public API
pub use parser::parse_bcf;
pub use serializer::serialize_bcf;
