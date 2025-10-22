use std::collections::HashMap;

/// Octant indices (a-h) representing child cube positions
/// a=000 (x-,y-,z-)  e=100 (x+,y-,z-)
/// b=001 (x-,y-,z+)  f=101 (x+,y-,z+)
/// c=010 (x-,y+,z-)  g=110 (x+,y+,z-)
/// d=011 (x-,y+,z+)  h=111 (x+,y+,z+)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Octant {
    A = 0, // 000
    B = 1, // 001
    C = 2, // 010
    D = 3, // 011
    E = 4, // 100
    F = 5, // 101
    G = 6, // 110
    H = 7, // 111
}

impl Octant {
    pub fn from_char(c: char) -> Option<Self> {
        match c {
            'a' => Some(Octant::A),
            'b' => Some(Octant::B),
            'c' => Some(Octant::C),
            'd' => Some(Octant::D),
            'e' => Some(Octant::E),
            'f' => Some(Octant::F),
            'g' => Some(Octant::G),
            'h' => Some(Octant::H),
            _ => None,
        }
    }

    pub fn to_char(self) -> char {
        match self {
            Octant::A => 'a',
            Octant::B => 'b',
            Octant::C => 'c',
            Octant::D => 'd',
            Octant::E => 'e',
            Octant::F => 'f',
            Octant::G => 'g',
            Octant::H => 'h',
        }
    }

    pub fn index(self) -> usize {
        self as usize
    }

    /// Get the offset for this octant within its parent cube
    /// Returns (x_offset, y_offset, z_offset) where each is 0 or 0.5
    pub fn offset(self) -> (f32, f32, f32) {
        let idx = self.index();
        let x = if idx & 0b100 != 0 { 0.5 } else { 0.0 };
        let y = if idx & 0b010 != 0 { 0.5 } else { 0.0 };
        let z = if idx & 0b001 != 0 { 0.5 } else { 0.0 };
        (x, y, z)
    }

    pub fn all() -> [Octant; 8] {
        [
            Octant::A,
            Octant::B,
            Octant::C,
            Octant::D,
            Octant::E,
            Octant::F,
            Octant::G,
            Octant::H,
        ]
    }
}

/// A node in the octree - either a value or subdivided into 8 children
#[derive(Debug, Clone)]
pub enum OctreeNode {
    Value(i32),
    Children(Box<[OctreeNode; 8]>),
}

impl OctreeNode {
    pub fn new_value(value: i32) -> Self {
        OctreeNode::Value(value)
    }

    pub fn new_children(children: [OctreeNode; 8]) -> Self {
        OctreeNode::Children(Box::new(children))
    }

    /// Get the child node at the given octant, if this is a subdivided node
    pub fn get_child(&self, octant: Octant) -> Option<&OctreeNode> {
        match self {
            OctreeNode::Children(children) => Some(&children[octant.index()]),
            OctreeNode::Value(_) => None,
        }
    }

    /// Get a node at the given path
    pub fn get_at_path(&self, path: &[Octant]) -> Option<&OctreeNode> {
        if path.is_empty() {
            return Some(self);
        }

        let mut current = self;
        for &octant in path {
            current = current.get_child(octant)?;
        }
        Some(current)
    }

    /// Apply a transform (mirror) to this node
    pub fn apply_transform(&self, axes: &[char]) -> OctreeNode {
        match self {
            OctreeNode::Value(v) => OctreeNode::Value(*v),
            OctreeNode::Children(children) => {
                let mut new_children = children.as_ref().clone();

                for axis in axes {
                    match axis {
                        'x' => {
                            // Mirror along X axis: swap a<->e, b<->f, c<->g, d<->h
                            new_children.swap(0, 4);
                            new_children.swap(1, 5);
                            new_children.swap(2, 6);
                            new_children.swap(3, 7);
                        }
                        'y' => {
                            // Mirror along Y axis: swap a<->c, b<->d, e<->g, f<->h
                            new_children.swap(0, 2);
                            new_children.swap(1, 3);
                            new_children.swap(4, 6);
                            new_children.swap(5, 7);
                        }
                        'z' => {
                            // Mirror along Z axis: swap a<->b, c<->d, e<->f, g<->h
                            new_children.swap(0, 1);
                            new_children.swap(2, 3);
                            new_children.swap(4, 5);
                            new_children.swap(6, 7);
                        }
                        _ => {} // Invalid axis, ignore
                    }
                }

                // Recursively apply transform to children
                let transformed: Vec<OctreeNode> = new_children
                    .iter()
                    .map(|child| child.apply_transform(axes))
                    .collect();

                OctreeNode::new_children(transformed.try_into().unwrap())
            }
        }
    }

    /// Traverse the octree and collect all leaf voxels with their positions
    pub fn collect_voxels(
        &self,
        position: (f32, f32, f32),
        size: f32,
        voxels: &mut Vec<(f32, f32, f32, f32, i32)>,
    ) {
        match self {
            OctreeNode::Value(value) => {
                if *value != 0 {
                    voxels.push((position.0, position.1, position.2, size, *value));
                }
            }
            OctreeNode::Children(children) => {
                let half_size = size / 2.0;
                for octant in Octant::all() {
                    let offset = octant.offset();
                    let child_pos = (
                        position.0 + offset.0 * size,
                        position.1 + offset.1 * size,
                        position.2 + offset.2 * size,
                    );
                    children[octant.index()].collect_voxels(child_pos, half_size, voxels);
                }
            }
        }
    }
}

/// The main octree structure representing a voxel model
#[derive(Debug, Clone)]
pub struct Octree {
    pub root: OctreeNode,
}

impl Octree {
    pub fn new(root: OctreeNode) -> Self {
        Octree { root }
    }

    pub fn empty() -> Self {
        Octree {
            root: OctreeNode::Value(0),
        }
    }

    /// Get a node at the given path
    pub fn get_at_path(&self, path: &[Octant]) -> Option<&OctreeNode> {
        self.root.get_at_path(path)
    }

    /// Collect all voxels in the tree
    pub fn collect_voxels(&self) -> Vec<(f32, f32, f32, f32, i32)> {
        let mut voxels = Vec::new();
        self.root.collect_voxels((0.0, 0.0, 0.0), 1.0, &mut voxels);
        voxels
    }
}

/// A builder for constructing octrees using paths
pub struct OctreeBuilder {
    assignments: HashMap<Vec<Octant>, OctreeNode>,
}

impl OctreeBuilder {
    pub fn new() -> Self {
        OctreeBuilder {
            assignments: HashMap::new(),
        }
    }

    pub fn set(&mut self, path: Vec<Octant>, node: OctreeNode) {
        self.assignments.insert(path, node);
    }

    pub fn build(self) -> Octree {
        if self.assignments.is_empty() {
            return Octree::empty();
        }

        // Build from the assignments
        let root = self.build_node(&[]);
        Octree::new(root)
    }

    fn build_node(&self, prefix: &[Octant]) -> OctreeNode {
        // Check if any assignments exist deeper than this path
        let has_deeper_children = self
            .assignments
            .keys()
            .any(|path| path.len() > prefix.len() && path[..prefix.len()] == *prefix);

        // Check if there's a direct assignment for this path
        let direct_assignment = self.assignments.get(prefix);

        // If we have both a direct assignment and deeper children, we need to merge
        if let Some(node) = direct_assignment {
            if has_deeper_children {
                // Clone the node and replace children with deeper assignments
                let mut base_children = match node {
                    OctreeNode::Children(children) => children.as_ref().clone(),
                    OctreeNode::Value(_) => {
                        // If it's a value but there are deeper children, we can't merge
                        // This shouldn't happen with valid CSM, but handle it gracefully
                        // by ignoring the value and building children
                        [
                            OctreeNode::Value(0),
                            OctreeNode::Value(0),
                            OctreeNode::Value(0),
                            OctreeNode::Value(0),
                            OctreeNode::Value(0),
                            OctreeNode::Value(0),
                            OctreeNode::Value(0),
                            OctreeNode::Value(0),
                        ]
                    }
                };

                // Replace children where we have deeper assignments
                for (i, octant) in Octant::all().iter().enumerate() {
                    let mut child_prefix = prefix.to_vec();
                    child_prefix.push(*octant);

                    // Check if there's anything at or below this child path
                    let has_child_assignment = self.assignments.keys().any(|path| {
                        path.len() >= child_prefix.len()
                            && path[..child_prefix.len()] == *child_prefix
                    });

                    if has_child_assignment {
                        base_children[i] = self.build_node(&child_prefix);
                    }
                }

                return OctreeNode::new_children(base_children);
            } else {
                // No deeper children, just return the direct assignment
                return node.clone();
            }
        }

        if !has_deeper_children {
            // No assignments for this subtree, return empty
            return OctreeNode::Value(0);
        }

        // Build children from deeper assignments
        let children: Vec<OctreeNode> = Octant::all()
            .iter()
            .map(|&octant| {
                let mut child_prefix = prefix.to_vec();
                child_prefix.push(octant);
                self.build_node(&child_prefix)
            })
            .collect();

        OctreeNode::new_children(children.try_into().unwrap())
    }
}

impl Default for OctreeBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_octant_conversions() {
        assert_eq!(Octant::from_char('a'), Some(Octant::A));
        assert_eq!(Octant::from_char('h'), Some(Octant::H));
        assert_eq!(Octant::from_char('z'), None);
        assert_eq!(Octant::A.to_char(), 'a');
        assert_eq!(Octant::H.to_char(), 'h');
    }

    #[test]
    fn test_octant_offset() {
        assert_eq!(Octant::A.offset(), (0.0, 0.0, 0.0));
        assert_eq!(Octant::H.offset(), (0.5, 0.5, 0.5));
        assert_eq!(Octant::E.offset(), (0.5, 0.0, 0.0));
    }

    #[test]
    fn test_simple_octree() {
        let tree = Octree::new(OctreeNode::Value(42));
        let voxels = tree.collect_voxels();
        assert_eq!(voxels.len(), 1);
        assert_eq!(voxels[0], (0.0, 0.0, 0.0, 1.0, 42));
    }

    #[test]
    fn test_builder() {
        let mut builder = OctreeBuilder::new();
        builder.set(vec![Octant::A], OctreeNode::Value(10));
        builder.set(vec![Octant::B], OctreeNode::Value(20));

        let tree = builder.build();
        let voxels = tree.collect_voxels();
        assert_eq!(voxels.len(), 2);
    }
}
