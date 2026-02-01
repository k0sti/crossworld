//! KDL configuration file reader
//!
//! Parses KDL files into a StateTree structure, preserving source location information.
//!
//! # KDL Mapping
//!
//! KDL nodes map to StateTree as follows:
//! - Node name becomes the path segment
//! - Node arguments become the node value (or array if multiple)
//! - Node properties become node attributes
//! - Child nodes become child StateNodes
//!
//! # Example
//!
//! ```kdl
//! app {
//!     scene {
//!         world macro_depth=3 micro_depth=5 seed=12345
//!     }
//! }
//! ```
//!
//! Maps to paths:
//! - `app.scene.world` with attributes `macro_depth=3`, `micro_depth=5`, `seed=12345`

use crate::{Result, SourceLocation, StateNode, StateTree, Value};
use std::path::Path;

/// KDL configuration reader
pub struct KdlReader;

impl KdlReader {
    /// Parse a KDL file into a StateTree
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<StateTree> {
        let path = path.as_ref();
        let content = std::fs::read_to_string(path)?;
        Self::from_string_with_source(&content, Some(path))
    }

    /// Parse a KDL string into a StateTree
    pub fn from_string(content: &str) -> Result<StateTree> {
        Self::from_string_with_source(content, None)
    }

    /// Parse a KDL string with optional source file info
    fn from_string_with_source(content: &str, source_file: Option<&Path>) -> Result<StateTree> {
        let doc: kdl::KdlDocument = content.parse()?;

        let mut root = StateNode::default();

        for node in doc.nodes() {
            let child = Self::parse_node(node, source_file);
            root.set_child(node.name().value(), child);
        }

        Ok(StateTree::with_root(root))
    }

    /// Parse a KDL node into a StateNode
    fn parse_node(node: &kdl::KdlNode, source_file: Option<&Path>) -> StateNode {
        // Note: span() is only available with "span" feature
        let source = SourceLocation::new(source_file.map(|p| p.to_path_buf()), None, None);

        let mut state_node = StateNode::with_source(Value::Null, source);

        // Parse arguments (positional values) as the node value
        let entries: Vec<&kdl::KdlEntry> = node.entries().iter().collect();
        let args: Vec<&kdl::KdlEntry> = entries
            .iter()
            .filter(|e| e.name().is_none())
            .copied()
            .collect();
        let props: Vec<&kdl::KdlEntry> = entries
            .iter()
            .filter(|e| e.name().is_some())
            .copied()
            .collect();

        // Set node value from arguments
        if args.len() == 1 {
            state_node.value = Self::kdl_value_to_value(args[0].value());
        } else if args.len() > 1 {
            let arr: Vec<Value> = args
                .iter()
                .map(|e| Self::kdl_value_to_value(e.value()))
                .collect();
            state_node.value = Value::Array(arr);
        }

        // Parse properties as attributes
        for prop in props {
            if let Some(name) = prop.name() {
                let value = Self::kdl_value_to_value(prop.value());
                state_node.set_attr(name.value(), value);
            }
        }

        // Parse children recursively
        if let Some(children) = node.children() {
            for child_node in children.nodes() {
                let child = Self::parse_node(child_node, source_file);
                state_node.set_child(child_node.name().value(), child);
            }
        }

        state_node
    }

    /// Convert a KDL value to our Value type
    fn kdl_value_to_value(kdl_val: &kdl::KdlValue) -> Value {
        match kdl_val {
            kdl::KdlValue::String(s) => Value::String(s.clone()),
            kdl::KdlValue::Integer(i) => {
                // i128 to i64 - may truncate for very large values
                Value::Int(*i as i64)
            }
            kdl::KdlValue::Float(f) => Value::Float(*f),
            kdl::KdlValue::Bool(b) => Value::Bool(*b),
            kdl::KdlValue::Null => Value::Null,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple() {
        let kdl = r#"
            app {
                name "Crossworld"
                version 1
            }
        "#;

        let tree = KdlReader::from_string(kdl).unwrap();

        // app.name has value "Crossworld"
        let app_node = tree.get_node("app").unwrap();
        let name_node = app_node.child("name").unwrap();
        assert_eq!(name_node.value.as_str().unwrap(), "Crossworld");

        let version_node = app_node.child("version").unwrap();
        assert_eq!(version_node.value.as_i64().unwrap(), 1);
    }

    #[test]
    fn test_parse_with_properties() {
        let kdl = r#"
            world macro_depth=3 micro_depth=5 seed=12345
        "#;

        let tree = KdlReader::from_string(kdl).unwrap();
        let world_node = tree.get_node("world").unwrap();

        assert_eq!(world_node.attr("macro_depth").unwrap().as_u32().unwrap(), 3);
        assert_eq!(world_node.attr("micro_depth").unwrap().as_u32().unwrap(), 5);
        assert_eq!(world_node.attr("seed").unwrap().as_u32().unwrap(), 12345);
    }

    #[test]
    fn test_parse_nested() {
        let kdl = r#"
            app {
                scene {
                    world macro_depth=3 {
                        ground material="grass"
                    }
                }
            }
        "#;

        let tree = KdlReader::from_string(kdl).unwrap();

        let world = tree.get_node("app.scene.world").unwrap();
        assert_eq!(world.attr("macro_depth").unwrap().as_u32().unwrap(), 3);

        let ground = tree.get_node("app.scene.world.ground").unwrap();
        assert_eq!(ground.attr("material").unwrap().as_str().unwrap(), "grass");
    }

    #[test]
    fn test_parse_array_value() {
        let kdl = r#"
            position 1.0 2.0 3.0
        "#;

        let tree = KdlReader::from_string(kdl).unwrap();
        let pos = tree.get_node("position").unwrap();

        let arr = pos.value.as_array().unwrap();
        assert_eq!(arr.len(), 3);
        assert!((arr[0].as_f64().unwrap() - 1.0).abs() < 0.001);
        assert!((arr[1].as_f64().unwrap() - 2.0).abs() < 0.001);
        assert!((arr[2].as_f64().unwrap() - 3.0).abs() < 0.001);
    }

    #[test]
    fn test_parse_bool() {
        // KDL v2 uses #true and #false for booleans
        let kdl = r#"
            config enabled=#true debug=#false
        "#;

        let tree = KdlReader::from_string(kdl).unwrap();
        let config = tree.get_node("config").unwrap();

        assert!(config.attr("enabled").unwrap().as_bool().unwrap());
        assert!(!config.attr("debug").unwrap().as_bool().unwrap());
    }
}
