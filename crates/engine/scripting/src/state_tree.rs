//! State tree data structure with KDL source tracking
//!
//! The StateTree provides:
//! - Named paths based on KDL structure (e.g., "app.scene.world.macro_depth")
//! - Values that can be read and written
//! - Source location tracking for debugging
//! - Change notifications (listeners)

use crate::{Error, Result, Value};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, RwLock};

/// Source location information from KDL file
#[derive(Debug, Clone, Default)]
pub struct SourceLocation {
    /// Path to the KDL source file
    pub file: Option<PathBuf>,
    /// Line number (1-indexed)
    pub line: Option<usize>,
    /// Column number (1-indexed)
    pub column: Option<usize>,
}

impl SourceLocation {
    /// Create a new source location
    pub fn new(file: Option<PathBuf>, line: Option<usize>, column: Option<usize>) -> Self {
        Self { file, line, column }
    }

    /// Check if this location has any source information
    pub fn has_source(&self) -> bool {
        self.file.is_some() || self.line.is_some()
    }
}

impl std::fmt::Display for SourceLocation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match (&self.file, self.line, self.column) {
            (Some(file), Some(line), Some(col)) => {
                write!(f, "{}:{}:{}", file.display(), line, col)
            }
            (Some(file), Some(line), None) => write!(f, "{}:{}", file.display(), line),
            (Some(file), None, _) => write!(f, "{}", file.display()),
            (None, Some(line), Some(col)) => write!(f, "line {}:{}", line, col),
            (None, Some(line), None) => write!(f, "line {}", line),
            (None, None, _) => write!(f, "<unknown>"),
        }
    }
}

/// A node in the state tree
#[derive(Debug, Clone)]
pub struct StateNode {
    /// The value at this node (may be null for pure container nodes)
    pub value: Value,
    /// Source location where this node was defined
    pub source: SourceLocation,
    /// Child nodes
    pub children: HashMap<String, StateNode>,
    /// Attributes (key-value pairs from KDL node arguments/properties)
    pub attributes: HashMap<String, Value>,
}

impl Default for StateNode {
    fn default() -> Self {
        Self {
            value: Value::Null,
            source: SourceLocation::default(),
            children: HashMap::new(),
            attributes: HashMap::new(),
        }
    }
}

impl StateNode {
    /// Create a new node with a value
    pub fn new(value: Value) -> Self {
        Self {
            value,
            ..Default::default()
        }
    }

    /// Create a new node with a value and source location
    pub fn with_source(value: Value, source: SourceLocation) -> Self {
        Self {
            value,
            source,
            ..Default::default()
        }
    }

    /// Get a child node by name
    pub fn child(&self, name: &str) -> Option<&StateNode> {
        self.children.get(name)
    }

    /// Get a mutable reference to a child node
    pub fn child_mut(&mut self, name: &str) -> Option<&mut StateNode> {
        self.children.get_mut(name)
    }

    /// Insert or update a child node
    pub fn set_child(&mut self, name: impl Into<String>, node: StateNode) {
        self.children.insert(name.into(), node);
    }

    /// Get an attribute value
    pub fn attr(&self, name: &str) -> Option<&Value> {
        self.attributes.get(name)
    }

    /// Set an attribute value
    pub fn set_attr(&mut self, name: impl Into<String>, value: Value) {
        self.attributes.insert(name.into(), value);
    }

    /// Check if this node has any children
    pub fn has_children(&self) -> bool {
        !self.children.is_empty()
    }

    /// Get all child names
    pub fn child_names(&self) -> impl Iterator<Item = &str> {
        self.children.keys().map(|s| s.as_str())
    }
}

/// Change listener callback type
pub type ChangeListener = Arc<dyn Fn(&str, &Value, &Value) + Send + Sync>;

/// The main state tree structure
#[derive(Default)]
pub struct StateTree {
    /// Root node of the tree
    root: StateNode,
    /// Change listeners keyed by path prefix
    listeners: RwLock<HashMap<String, Vec<ChangeListener>>>,
}

impl StateTree {
    /// Create a new empty state tree
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a state tree with a root node
    pub fn with_root(root: StateNode) -> Self {
        Self {
            root,
            listeners: RwLock::new(HashMap::new()),
        }
    }

    /// Get the root node
    pub fn root(&self) -> &StateNode {
        &self.root
    }

    /// Get a mutable reference to the root node
    pub fn root_mut(&mut self) -> &mut StateNode {
        &mut self.root
    }

    /// Get a node by path (e.g., "app.scene.world")
    pub fn get_node(&self, path: &str) -> Option<&StateNode> {
        if path.is_empty() {
            return Some(&self.root);
        }

        let mut current = &self.root;
        for part in path.split('.') {
            current = current.child(part)?;
        }
        Some(current)
    }

    /// Get a mutable node by path
    pub fn get_node_mut(&mut self, path: &str) -> Option<&mut StateNode> {
        if path.is_empty() {
            return Some(&mut self.root);
        }

        let mut current = &mut self.root;
        for part in path.split('.') {
            current = current.child_mut(part)?;
        }
        Some(current)
    }

    /// Get a value by path
    pub fn get(&self, path: &str) -> Result<&Value> {
        self.get_node(path)
            .map(|n| &n.value)
            .ok_or_else(|| Error::PathNotFound(path.to_string()))
    }

    /// Get a value by path with a default if not found
    pub fn get_or<'a>(&'a self, path: &str, default: &'a Value) -> &'a Value {
        self.get_node(path).map(|n| &n.value).unwrap_or(default)
    }

    /// Set a value by path, creating intermediate nodes as needed
    pub fn set(&mut self, path: &str, value: Value) -> Result<()> {
        let old_value = self.get(path).cloned().unwrap_or(Value::Null);

        // Navigate to parent, creating nodes as needed
        let parts: Vec<&str> = path.split('.').collect();
        if parts.is_empty() {
            self.root.value = value.clone();
        } else {
            let mut current = &mut self.root;
            for &part in &parts[..parts.len() - 1] {
                current = current.children.entry(part.to_string()).or_default();
            }

            let last_part = parts.last().unwrap();
            if let Some(node) = current.children.get_mut(*last_part) {
                node.value = value.clone();
            } else {
                current
                    .children
                    .insert((*last_part).to_string(), StateNode::new(value.clone()));
            }
        }

        // Notify listeners
        self.notify_change(path, &old_value, &value);
        Ok(())
    }

    /// Get value as specific type
    pub fn get_bool(&self, path: &str) -> Result<bool> {
        self.get(path)?.as_bool()
    }

    pub fn get_i64(&self, path: &str) -> Result<i64> {
        self.get(path)?.as_i64()
    }

    pub fn get_u32(&self, path: &str) -> Result<u32> {
        self.get(path)?.as_u32()
    }

    pub fn get_f32(&self, path: &str) -> Result<f32> {
        self.get(path)?.as_f32()
    }

    pub fn get_f64(&self, path: &str) -> Result<f64> {
        self.get(path)?.as_f64()
    }

    pub fn get_str(&self, path: &str) -> Result<&str> {
        self.get(path)?.as_str()
    }

    /// Add a change listener for a path prefix
    /// The listener will be called when any value at or under the prefix changes
    pub fn add_listener(&self, path_prefix: &str, listener: ChangeListener) {
        let mut listeners = self.listeners.write().unwrap();
        listeners
            .entry(path_prefix.to_string())
            .or_default()
            .push(listener);
    }

    /// Remove all listeners for a path prefix
    pub fn remove_listeners(&self, path_prefix: &str) {
        let mut listeners = self.listeners.write().unwrap();
        listeners.remove(path_prefix);
    }

    /// Notify listeners of a change
    fn notify_change(&self, path: &str, old_value: &Value, new_value: &Value) {
        let listeners = self.listeners.read().unwrap();

        for (prefix, callbacks) in listeners.iter() {
            // Check if the changed path is under this prefix
            if path.starts_with(prefix) || prefix.is_empty() {
                for callback in callbacks {
                    callback(path, old_value, new_value);
                }
            }
        }
    }

    /// Iterate over all nodes depth-first
    pub fn iter(&self) -> StateTreeIter<'_> {
        StateTreeIter::new(&self.root)
    }
}

impl std::fmt::Debug for StateTree {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("StateTree")
            .field("root", &self.root)
            .finish()
    }
}

/// Iterator over state tree nodes
pub struct StateTreeIter<'a> {
    stack: Vec<(&'a str, &'a StateNode, String)>,
}

impl<'a> StateTreeIter<'a> {
    fn new(root: &'a StateNode) -> Self {
        let mut stack = Vec::new();
        // Push children in reverse order so they come out in order
        for (name, child) in root.children.iter() {
            stack.push((name.as_str(), child, name.clone()));
        }
        Self { stack }
    }
}

impl<'a> Iterator for StateTreeIter<'a> {
    type Item = (String, &'a StateNode);

    fn next(&mut self) -> Option<Self::Item> {
        let (_name, node, path) = self.stack.pop()?;

        // Push children
        for (child_name, child) in node.children.iter() {
            let child_path = format!("{}.{}", path, child_name);
            self.stack.push((child_name.as_str(), child, child_path));
        }

        Some((path, node))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_state_tree_basic() {
        let mut tree = StateTree::new();

        tree.set("app.name", Value::from("Crossworld")).unwrap();
        tree.set("app.version", Value::from(1)).unwrap();
        tree.set("app.scene.world.macro_depth", Value::from(3))
            .unwrap();

        assert_eq!(tree.get_str("app.name").unwrap(), "Crossworld");
        assert_eq!(tree.get_i64("app.version").unwrap(), 1);
        assert_eq!(tree.get_u32("app.scene.world.macro_depth").unwrap(), 3);
    }

    #[test]
    fn test_state_tree_path_not_found() {
        let tree = StateTree::new();
        assert!(tree.get("nonexistent.path").is_err());
    }

    #[test]
    fn test_state_node_children() {
        let mut node = StateNode::default();
        node.set_child("child1", StateNode::new(Value::from(1)));
        node.set_child("child2", StateNode::new(Value::from(2)));

        assert!(node.has_children());
        assert_eq!(node.child("child1").unwrap().value.as_i64().unwrap(), 1);
        assert_eq!(node.child("child2").unwrap().value.as_i64().unwrap(), 2);
    }

    #[test]
    fn test_change_listener() {
        use std::sync::atomic::{AtomicBool, Ordering};

        let mut tree = StateTree::new();
        let called = Arc::new(AtomicBool::new(false));
        let called_clone = called.clone();

        tree.add_listener(
            "app",
            Arc::new(move |path, _old, _new| {
                assert!(path.starts_with("app"));
                called_clone.store(true, Ordering::SeqCst);
            }),
        );

        tree.set("app.value", Value::from(42)).unwrap();
        assert!(called.load(Ordering::SeqCst));
    }

    #[test]
    fn test_source_location() {
        let loc = SourceLocation::new(Some(PathBuf::from("config.kdl")), Some(10), Some(5));
        assert!(loc.has_source());
        assert_eq!(loc.to_string(), "config.kdl:10:5");
    }
}
