//! BCF deserialization - Parse binary format to Cube<u8>

use super::constants::*;
use super::BcfError;
use crate::Cube;
use std::rc::Rc;

/// Parse BCF binary data into a Cube<u8>
///
/// # Example
///
/// ```
/// use cube::{Cube, io::bcf::{serialize_bcf, parse_bcf}};
///
/// let cube = Cube::Solid(42u8);
/// let binary = serialize_bcf(&cube);
/// let parsed = parse_bcf(&binary).unwrap();
/// assert_eq!(cube, parsed);
/// ```
pub fn parse_bcf(data: &[u8]) -> Result<Cube<u8>, BcfError> {
    let mut parser = BcfParser::new(data)?;
    parser.parse_cube()
}

/// BCF parser state
struct BcfParser<'a> {
    data: &'a [u8],
    root_offset: usize,
    recursion_depth: usize,
}

impl<'a> BcfParser<'a> {
    fn new(data: &'a [u8]) -> Result<Self, BcfError> {
        // Validate minimum size
        if data.len() < HEADER_SIZE {
            return Err(BcfError::TruncatedData {
                expected_bytes: HEADER_SIZE,
                available_bytes: data.len(),
            });
        }

        // Validate magic number
        let magic = u32::from_le_bytes([data[0], data[1], data[2], data[3]]);
        if magic != MAGIC {
            return Err(BcfError::InvalidMagic {
                expected: MAGIC,
                found: magic,
            });
        }

        // Validate version
        let version = data[4];
        if version != VERSION {
            return Err(BcfError::UnsupportedVersion { found: version });
        }

        // Read root offset
        let root_offset = u32::from_le_bytes([data[8], data[9], data[10], data[11]]) as usize;

        // Validate root offset
        if root_offset >= data.len() {
            return Err(BcfError::InvalidOffset {
                offset: root_offset,
                file_size: data.len(),
            });
        }

        Ok(Self {
            data,
            root_offset,
            recursion_depth: 0,
        })
    }

    fn parse_cube(&mut self) -> Result<Cube<u8>, BcfError> {
        self.parse_node_at(self.root_offset)
    }

    fn parse_node_at(&mut self, offset: usize) -> Result<Cube<u8>, BcfError> {
        // Check recursion depth
        if self.recursion_depth >= MAX_RECURSION_DEPTH {
            return Err(BcfError::RecursionLimit {
                max_depth: MAX_RECURSION_DEPTH,
            });
        }

        self.recursion_depth += 1;
        let result = self.parse_node_at_impl(offset);
        self.recursion_depth -= 1;

        result
    }

    fn parse_node_at_impl(&mut self, offset: usize) -> Result<Cube<u8>, BcfError> {
        // Validate offset
        if offset >= self.data.len() {
            return Err(BcfError::InvalidOffset {
                offset,
                file_size: self.data.len(),
            });
        }

        // Read type byte
        let type_byte = self.data[offset];

        // Check MSB
        if (type_byte & MSB_MASK) == 0 {
            // Inline leaf (MSB=0): value is in lower 7 bits
            let value = type_byte & VALUE_MASK;
            return Ok(Cube::Solid(value));
        }

        // Extract type ID and size field
        let type_id = (type_byte & TYPE_MASK) >> 4;
        let size_field = type_byte & SIZE_MASK;

        match type_id {
            TYPE_EXTENDED_LEAF => self.parse_extended_leaf(offset),
            TYPE_OCTA_LEAVES => self.parse_octa_leaves(offset),
            TYPE_OCTA_POINTERS => self.parse_octa_pointers(offset, size_field),
            _ => Err(BcfError::InvalidTypeId { type_id }),
        }
    }

    fn parse_extended_leaf(&self, offset: usize) -> Result<Cube<u8>, BcfError> {
        // Extended leaf: type byte + value byte
        if offset + 2 > self.data.len() {
            return Err(BcfError::TruncatedData {
                expected_bytes: 2,
                available_bytes: self.data.len() - offset,
            });
        }

        let value = self.data[offset + 1];
        Ok(Cube::Solid(value))
    }

    fn parse_octa_leaves(&self, offset: usize) -> Result<Cube<u8>, BcfError> {
        // Octa with 8 leaf values: type byte + 8 value bytes
        if offset + 9 > self.data.len() {
            return Err(BcfError::TruncatedData {
                expected_bytes: 9,
                available_bytes: self.data.len() - offset,
            });
        }

        // Read 8 values
        let mut children: [Rc<Cube<u8>>; 8] = std::array::from_fn(|_| Rc::new(Cube::Solid(0)));

        for (i, child) in children.iter_mut().enumerate() {
            let value = self.data[offset + 1 + i];
            *child = Rc::new(Cube::Solid(value));
        }

        Ok(Cube::Cubes(Box::new(children)))
    }

    fn parse_octa_pointers(&mut self, offset: usize, ssss: u8) -> Result<Cube<u8>, BcfError> {
        // Validate SSSS value
        if ssss > 3 {
            return Err(BcfError::InvalidPointerSize { ssss });
        }

        let pointer_size = 1 << ssss; // 2^ssss
        let node_size = 1 + 8 * pointer_size;

        // Validate we have enough data for the node
        if offset + node_size > self.data.len() {
            return Err(BcfError::TruncatedData {
                expected_bytes: node_size,
                available_bytes: self.data.len() - offset,
            });
        }

        // Read 8 pointers
        let mut child_offsets = Vec::with_capacity(8);
        for i in 0..8 {
            let ptr_offset = offset + 1 + i * pointer_size;
            let child_offset = self.read_pointer(ptr_offset, pointer_size)?;
            child_offsets.push(child_offset);
        }

        // Parse children
        let mut children: [Rc<Cube<u8>>; 8] = std::array::from_fn(|_| Rc::new(Cube::Solid(0)));

        for (i, &child_offset) in child_offsets.iter().enumerate() {
            let child = self.parse_node_at(child_offset)?;
            children[i] = Rc::new(child);
        }

        Ok(Cube::Cubes(Box::new(children)))
    }

    fn read_pointer(&self, offset: usize, size: usize) -> Result<usize, BcfError> {
        if offset + size > self.data.len() {
            return Err(BcfError::TruncatedData {
                expected_bytes: size,
                available_bytes: self.data.len() - offset,
            });
        }

        let value = match size {
            1 => self.data[offset] as usize,
            2 => u16::from_le_bytes([self.data[offset], self.data[offset + 1]]) as usize,
            4 => u32::from_le_bytes([
                self.data[offset],
                self.data[offset + 1],
                self.data[offset + 2],
                self.data[offset + 3],
            ]) as usize,
            8 => u64::from_le_bytes([
                self.data[offset],
                self.data[offset + 1],
                self.data[offset + 2],
                self.data[offset + 3],
                self.data[offset + 4],
                self.data[offset + 5],
                self.data[offset + 6],
                self.data[offset + 7],
            ]) as usize,
            _ => unreachable!(),
        };

        Ok(value)
    }
}
