//! BCF serialization - Convert Cube<u8> to binary format

use super::constants::*;
use crate::Cube;
use std::rc::Rc;

/// Serialize a Cube<u8> to BCF binary format
///
/// # Example
///
/// ```
/// use cube::{Cube, io::bcf::serialize_bcf};
///
/// let cube = Cube::Solid(42u8);
/// let binary = serialize_bcf(&cube);
/// assert!(binary.len() >= 12); // At least header size
/// ```
pub fn serialize_bcf(cube: &Cube<u8>) -> Vec<u8> {
    BcfWriterV2::new().serialize(cube)
}

/// BCF writer implementation
struct BcfWriterV2 {
    buffer: Vec<u8>,
    base_offset: usize, // Offset where this writer's buffer will be placed in the final file
}

impl BcfWriterV2 {
    fn new() -> Self {
        Self {
            buffer: Vec::new(),
            base_offset: 0,
        }
    }

    fn with_base_offset(base_offset: usize) -> Self {
        Self {
            buffer: Vec::new(),
            base_offset,
        }
    }

    /// Serialize cube to BCF format
    fn serialize(mut self, cube: &Cube<u8>) -> Vec<u8> {
        // Write header placeholder
        self.buffer.resize(HEADER_SIZE, 0);

        // Write root node
        let root_offset = self.write_node(cube);

        // Fill in header
        self.write_header(root_offset);

        self.buffer
    }

    fn write_header(&mut self, root_offset: usize) {
        self.buffer[0..4].copy_from_slice(&MAGIC.to_le_bytes());
        self.buffer[4] = VERSION;
        self.buffer[5..8].copy_from_slice(&[0, 0, 0]);
        self.buffer[8..12].copy_from_slice(&(root_offset as u32).to_le_bytes());
    }

    fn write_node(&mut self, cube: &Cube<u8>) -> usize {
        match cube {
            Cube::Solid(value) => self.write_leaf(*value),
            Cube::Cubes(children) => {
                if Self::all_solid(children) {
                    self.write_octa_leaves(children)
                } else {
                    self.write_octa_pointers(children)
                }
            }
            _ => self.write_leaf(0),
        }
    }

    fn write_leaf(&mut self, value: u8) -> usize {
        let offset = self.buffer.len();
        if value <= 127 {
            self.buffer.push(value);
        } else {
            self.buffer.push(EXTENDED_LEAF_BASE);
            self.buffer.push(value);
        }
        offset
    }

    fn write_octa_leaves(&mut self, children: &[Rc<Cube<u8>>; 8]) -> usize {
        let offset = self.buffer.len();
        self.buffer.push(OCTA_LEAVES_BASE);

        for child in children {
            if let Cube::Solid(v) = child.as_ref() {
                self.buffer.push(*v);
            } else {
                self.buffer.push(0);
            }
        }

        offset
    }

    fn write_octa_pointers(&mut self, children: &[Rc<Cube<u8>>; 8]) -> usize {
        // Strategy: Write children to a temporary buffer first to calculate sizes,
        // then write the node header with pointers, then append children

        let node_offset = self.buffer.len();

        // Calculate pointer size needed (conservative estimate)
        let max_possible_offset = node_offset + 1 + 8 * 8 + children.len() * 100; // Rough estimate
        let ssss = Self::calc_ssss(max_possible_offset);
        let pointer_size = 1 << ssss;

        // Calculate where children will start in the final buffer
        let children_start = node_offset + 1 + 8 * pointer_size;

        // Write children to temporary buffer WITH correct base offset
        let mut temp_children = Vec::new();
        let mut child_sizes = Vec::with_capacity(8);

        for child in children.iter() {
            // Calculate where THIS child will be in the final buffer
            // Must include self.base_offset for nested nodes!
            let child_base_offset = self.base_offset + children_start + temp_children.len();

            // Create child writer with correct base offset
            let mut child_writer = BcfWriterV2::with_base_offset(child_base_offset);
            child_writer.write_node(child.as_ref());
            child_sizes.push(child_writer.buffer.len());
            temp_children.extend_from_slice(&child_writer.buffer);
        }

        // Calculate actual child offsets
        // Offsets are relative to current buffer, but we write ABSOLUTE offsets for the final file
        let mut child_offsets = Vec::with_capacity(8);
        let mut offset_in_temp = 0;

        for &size in &child_sizes {
            // Offset in final file = base_offset + current_buffer_position + children_start + offset_in_temp
            let absolute_offset = self.base_offset + children_start + offset_in_temp;
            child_offsets.push(absolute_offset);
            offset_in_temp += size;
        }

        // Write node header: type byte
        self.buffer.push(OCTA_POINTERS_BASE | ssss);

        // Write pointers (absolute offsets in final file)
        for &absolute_offset in &child_offsets {
            match pointer_size {
                1 => self.buffer.push(absolute_offset as u8),
                2 => self
                    .buffer
                    .extend_from_slice(&(absolute_offset as u16).to_le_bytes()),
                4 => self
                    .buffer
                    .extend_from_slice(&(absolute_offset as u32).to_le_bytes()),
                8 => self
                    .buffer
                    .extend_from_slice(&(absolute_offset as u64).to_le_bytes()),
                _ => unreachable!(),
            }
        }

        // Append children data
        self.buffer.extend_from_slice(&temp_children);

        node_offset
    }

    fn calc_ssss(max_offset: usize) -> u8 {
        if max_offset <= 0xFF {
            0
        } else if max_offset <= 0xFFFF {
            1
        } else if max_offset <= 0xFFFFFFFF {
            2
        } else {
            3
        }
    }

    fn all_solid(children: &[Rc<Cube<u8>>; 8]) -> bool {
        children
            .iter()
            .all(|c| matches!(c.as_ref(), Cube::Solid(_)))
    }
}
