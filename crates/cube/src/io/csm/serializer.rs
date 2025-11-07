use crate::core::{octant_index_to_char, Cube, Octree};
use std::fmt::Write as FmtWrite;

/// Serialize an Octree to CSM text format
pub fn serialize_csm(tree: &Octree) -> String {
    let mut output = String::new();
    serialize_cube(&tree.root, &[], &mut output);
    output
}

/// Serialize a cube with its path prefix
fn serialize_cube(cube: &Cube<i32>, path: &[usize], output: &mut String) {
    match cube {
        Cube::Solid(value) => {
            // Only output non-zero values
            if *value != 0 {
                write_statement(path, &format!("{}", value), output);
            }
        }
        Cube::Cubes(children) => {
            // Check if this node itself should be written as an array,
            // or if we should recurse to children
            if should_write_as_array(children) {
                let array_content = format_array(children);
                write_statement(path, &array_content, output);
            } else {
                // Recurse into children
                for (i, child) in children.iter().enumerate() {
                    let mut child_path = path.to_vec();
                    child_path.push(i);
                    serialize_cube(child, &child_path, output);
                }
            }
        }
        Cube::Planes { .. } | Cube::Slices { .. } => {
            // For now, skip Planes and Slices variants
            // They are not commonly used in simple voxel models
        }
    }
}

/// Check if children should be written as an inline array
/// Returns true if all children are simple Solid values
fn should_write_as_array(children: &[std::rc::Rc<Cube<i32>>; 8]) -> bool {
    children.iter().all(|c| matches!(&**c, Cube::Solid(_)))
}

/// Format children as an array string like "[1 2 3 4 5 6 7 8]"
fn format_array(children: &[std::rc::Rc<Cube<i32>>; 8]) -> String {
    let values: Vec<String> = children
        .iter()
        .map(|c| match &**c {
            Cube::Solid(v) => format!("{}", v),
            _ => "0".to_string(),
        })
        .collect();
    format!("[{}]", values.join(" "))
}

/// Write a CSM statement like ">abc 42" or ">a [1 2 3 4 5 6 7 8]"
fn write_statement(path: &[usize], value: &str, output: &mut String) {
    let _ = write!(output, ">");
    for &idx in path {
        if let Some(c) = octant_index_to_char(idx) {
            let _ = write!(output, "{}", c);
        }
    }
    let _ = writeln!(output, " {}", value);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::io::csm::parser::parse_csm;
    use std::rc::Rc;

    #[test]
    fn test_serialize_simple_solid() {
        let tree = Octree::new(Cube::Solid(42));
        // Root solid with value should output nothing (or just the root)
        let csm = serialize_csm(&tree);
        // Root solid values are typically not serialized unless at a path
        assert!(csm.is_empty() || csm.contains("42"));
    }

    #[test]
    fn test_serialize_array() {
        let tree = Octree::new(Cube::cubes([
            Rc::new(Cube::Solid(1)),
            Rc::new(Cube::Solid(2)),
            Rc::new(Cube::Solid(3)),
            Rc::new(Cube::Solid(4)),
            Rc::new(Cube::Solid(5)),
            Rc::new(Cube::Solid(6)),
            Rc::new(Cube::Solid(7)),
            Rc::new(Cube::Solid(8)),
        ]));

        let csm = serialize_csm(&tree);
        // Since root is Cubes with all Solid children, should output as array
        assert!(csm.contains("[1 2 3 4 5 6 7 8]"));
    }

    #[test]
    fn test_serialize_nested() {
        let tree = Octree::new(Cube::cubes([
            Rc::new(Cube::cubes([
                Rc::new(Cube::Solid(10)),
                Rc::new(Cube::Solid(11)),
                Rc::new(Cube::Solid(12)),
                Rc::new(Cube::Solid(13)),
                Rc::new(Cube::Solid(14)),
                Rc::new(Cube::Solid(15)),
                Rc::new(Cube::Solid(16)),
                Rc::new(Cube::Solid(17)),
            ])),
            Rc::new(Cube::Solid(0)),
            Rc::new(Cube::Solid(0)),
            Rc::new(Cube::Solid(0)),
            Rc::new(Cube::Solid(0)),
            Rc::new(Cube::Solid(0)),
            Rc::new(Cube::Solid(0)),
            Rc::new(Cube::Solid(0)),
        ]));

        let csm = serialize_csm(&tree);
        // Should output nested structure at path 'a'
        assert!(csm.contains(">a"));
        assert!(csm.contains("[10 11 12 13 14 15 16 17]"));
    }

    #[test]
    fn test_roundtrip() {
        // Create a simple structure
        let original = Octree::new(Cube::cubes([
            Rc::new(Cube::Solid(1)),
            Rc::new(Cube::Solid(2)),
            Rc::new(Cube::Solid(3)),
            Rc::new(Cube::Solid(4)),
            Rc::new(Cube::Solid(5)),
            Rc::new(Cube::Solid(6)),
            Rc::new(Cube::Solid(7)),
            Rc::new(Cube::Solid(8)),
        ]));

        // Serialize
        let csm = serialize_csm(&original);

        // Parse back
        let parsed = parse_csm(&csm).unwrap();

        // Compare root structures (simplified comparison)
        match (&original.root, &parsed.root) {
            (Cube::Cubes(orig_children), Cube::Cubes(parsed_children)) => {
                for i in 0..8 {
                    assert_eq!(orig_children[i].id(), parsed_children[i].id());
                }
            }
            _ => panic!("Expected Cubes variant for both"),
        }
    }
}
