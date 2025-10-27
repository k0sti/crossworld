use glam::IVec3;
use std::rc::Rc;

/// Extension trait for IVec3 to add octree-specific functionality
pub trait IVec3Ext {
    /// Convert octant index (0-7) to 3D position (each component 0 or 1)
    /// Layout: index = x*4 + y*2 + z
    fn from_octant_index(index: usize) -> Self;

    /// Convert 3D position to octant index (x*4 + y*2 + z)
    /// Each component should be 0 or 1
    fn to_octant_index(self) -> usize;

    /// Step function: returns 0 if component is 0, else 1
    fn step0(self) -> Self;
}

impl IVec3Ext for IVec3 {
    fn from_octant_index(index: usize) -> Self {
        IVec3::new(
            ((index >> 2) & 1) as i32,
            ((index >> 1) & 1) as i32,
            (index & 1) as i32,
        )
    }

    fn to_octant_index(self) -> usize {
        ((self.x << 2) | (self.y << 1) | self.z) as usize
    }

    fn step0(self) -> Self {
        IVec3::new(
            if self.x == 0 { 0 } else { 1 },
            if self.y == 0 { 0 } else { 1 },
            if self.z == 0 { 0 } else { 1 },
        )
    }
}

/// Axis for 2D (Planes) and 1D (Slices) subdivision
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Axis {
    X,
    Y,
    Z,
}

impl Axis {
    pub fn from_char(c: char) -> Option<Self> {
        match c {
            'x' | 'X' => Some(Axis::X),
            'y' | 'Y' => Some(Axis::Y),
            'z' | 'Z' => Some(Axis::Z),
            _ => None,
        }
    }

    pub fn to_char(self) -> char {
        match self {
            Axis::X => 'x',
            Axis::Y => 'y',
            Axis::Z => 'z',
        }
    }
}

/// 2D quadtree for plane subdivision
#[derive(Debug, Clone, PartialEq)]
pub enum Quad<T> {
    Solid(T),
    Quads(Box<[Rc<Quad<T>>; 4]>),
}

impl<T> Quad<T> {
    pub fn solid(value: T) -> Self {
        Quad::Solid(value)
    }

    pub fn quads(children: [Rc<Quad<T>>; 4]) -> Self {
        Quad::Quads(Box::new(children))
    }
}

/// 3D cube structure with multiple subdivision strategies
#[derive(Debug, Clone, PartialEq)]
pub enum Cube<T> {
    Solid(T),
    Cubes(Box<[Rc<Cube<T>>; 8]>),
    Planes {
        axis: Axis,
        quad: Rc<Quad<T>>,
    },
    Slices {
        axis: Axis,
        layers: Rc<Vec<Rc<Cube<T>>>>,
    },
}

impl<T> Cube<T> {
    pub fn solid(value: T) -> Self {
        Cube::Solid(value)
    }

    pub fn cubes(children: [Rc<Cube<T>>; 8]) -> Self {
        Cube::Cubes(Box::new(children))
    }

    pub fn planes(axis: Axis, quad: Rc<Quad<T>>) -> Self {
        Cube::Planes { axis, quad }
    }

    pub fn slices(axis: Axis, layers: Rc<Vec<Rc<Cube<T>>>>) -> Self {
        Cube::Slices { axis, layers }
    }

    /// Calculate octant index at given depth for position
    /// Returns which octant (0-7) the position falls into at this depth level
    pub fn index(depth: u32, pos: IVec3) -> usize {
        let p = (pos >> depth) & 1; // Get LSB at this depth level
        p.to_octant_index()
    }

    /// Get child cube by octant index (0-7)
    /// Octant layout: a=0 (x-,y-,z-) to h=7 (x+,y+,z+)
    pub fn get_child(&self, index: usize) -> Option<&Rc<Cube<T>>> {
        match self {
            Cube::Cubes(children) if index < 8 => Some(&children[index]),
            _ => None,
        }
    }

    /// Get child or return self for uniform structures
    fn get_child_or_self(&self, index: usize) -> &Cube<T> {
        match self {
            Cube::Cubes(children) if index < 8 => &children[index],
            _ => self, // Solid/Planes/Slices act as uniform
        }
    }

    /// Get cube at specific position and depth
    /// Similar to Scala's apply(depth, pos)
    pub fn get(&self, depth: u32, pos: IVec3) -> &Cube<T> {
        if depth == 0 {
            self
        } else {
            let d = depth - 1;
            let index = Self::index(d, pos);
            self.get_child_or_self(index).get(d, pos)
        }
    }

    /// Get child by path of octant indices
    pub fn get_at_path(&self, path: &[usize]) -> Option<&Rc<Cube<T>>> {
        if path.is_empty() {
            return None;
        }

        let mut current = self.get_child(path[0])?;
        for &idx in &path[1..] {
            current = current.get_child(idx)?;
        }
        Some(current)
    }

    /// Visit all leaf nodes with their depth and position
    pub fn visit_leaves<F>(&self, depth: u32, pos: IVec3, callback: &mut F)
    where
        F: FnMut(&Cube<T>, u32, IVec3),
    {
        match self {
            Cube::Solid(_) => {
                callback(self, depth, pos);
            }
            Cube::Cubes(children) => {
                if depth > 0 {
                    for i in 0..8 {
                        let d = depth - 1;
                        let p = (pos << 1) + IVec3::from_octant_index(i);
                        children[i].visit_leaves(d, p, callback);
                    }
                } else {
                    callback(self, depth, pos);
                }
            }
            _ => {
                callback(self, depth, pos);
            }
        }
    }

    /// Visit all nodes at a specific depth
    pub fn visit_deep<F>(&self, depth: u32, pos: IVec3, callback: &mut F)
    where
        F: FnMut(&Cube<T>, IVec3),
    {
        if depth == 0 {
            callback(self, pos);
        } else {
            match self {
                Cube::Cubes(children) => {
                    for i in 0..8 {
                        let d = depth - 1;
                        let p = (pos << 1) + IVec3::from_octant_index(i);
                        children[i].visit_deep(d, p, callback);
                    }
                }
                _ => {
                    // For non-branching nodes, treat as uniform and recurse
                    for i in 0..8 {
                        let d = depth - 1;
                        let p = (pos << 1) + IVec3::from_octant_index(i);
                        self.visit_deep(d, p, callback);
                    }
                }
            }
        }
    }
}

impl<T: Clone> Cube<T> {
    /// Create cube by tabulating function over octant indices
    pub fn tabulate<F>(init: F) -> Self
    where
        F: Fn(usize) -> Cube<T>,
    {
        Cube::cubes(std::array::from_fn(|i| Rc::new(init(i))))
    }

    /// Create cube by tabulating function over 3D positions
    pub fn tabulate_vector<F>(init: F) -> Self
    where
        F: Fn(IVec3) -> Cube<T>,
    {
        Cube::cubes(std::array::from_fn(|i| {
            Rc::new(init(IVec3::from_octant_index(i)))
        }))
    }

    /// Get child or create uniform children from solid/other types
    fn get_or_expand_children(&self) -> [Rc<Cube<T>>; 8] {
        match self {
            Cube::Cubes(children) => {
                let vec: Vec<_> = children.to_vec();
                [
                    vec[0].clone(),
                    vec[1].clone(),
                    vec[2].clone(),
                    vec[3].clone(),
                    vec[4].clone(),
                    vec[5].clone(),
                    vec[6].clone(),
                    vec[7].clone(),
                ]
            }
            Cube::Solid(v) => std::array::from_fn(|_| Rc::new(Cube::Solid(v.clone()))),
            _ => std::array::from_fn(|_| Rc::new(Cube::Solid(self.id_default()))),
        }
    }

    /// Get default ID for non-Cubes variants (override in Cube<i32>)
    fn id_default(&self) -> T {
        match self {
            Cube::Solid(v) => v.clone(),
            _ => panic!("Cannot get default ID for non-Solid Cube without i32 type"),
        }
    }

    /// Immutably update child at specific index
    pub fn updated_index(&self, index: usize, cube: Cube<T>) -> Self {
        let mut children = self.get_or_expand_children();
        children[index] = Rc::new(cube);
        Cube::Cubes(Box::new(children))
    }

    /// Immutably update cube at position and depth
    pub fn updated(&self, cube: Cube<T>, depth: u32, pos: IVec3) -> Self {
        if depth == 0 {
            cube
        } else {
            let d = depth - 1;
            let index = Self::index(d, pos);
            let child = self.get_child_or_self(index);
            let new_child = child.updated(cube, d, pos);
            self.updated_index(index, new_child)
        }
    }
}

impl<T: Clone + PartialEq> Cube<T> {
    /// Simplify cube by collapsing uniform children into a single leaf
    pub fn simplified(self) -> Self {
        match &self {
            Cube::Cubes(children) => {
                // Check if all children are Solid with the same value
                if let Cube::Solid(first_val) = &*children[0] {
                    let all_same = children[1..]
                        .iter()
                        .all(|c| matches!(&**c, Cube::Solid(v) if v == first_val));
                    if all_same {
                        return Cube::Solid(first_val.clone());
                    }
                }
                self
            }
            _ => self,
        }
    }
}

impl Cube<i32> {
    /// Get ID value for this cube
    pub fn id(&self) -> i32 {
        match self {
            Cube::Solid(v) => *v,
            Cube::Cubes(children) => {
                // Return most common ID among children (like Scala version)
                let ids: Vec<i32> = children.iter().map(|c| c.id()).collect();
                // Simple mode calculation - just return first for now
                ids[0]
            }
            _ => 0,
        }
    }

    /// Get ID at specific position and depth
    pub fn get_id(&self, depth: u32, pos: IVec3) -> i32 {
        self.get(depth, pos).id()
    }

    /// Merge two cubes (union operation with preference for non-empty)
    pub fn add(&self, other: &Cube<i32>) -> Self {
        match (self, other) {
            (Cube::Solid(a), Cube::Solid(b)) => {
                // Prefer non-zero value
                if *b != 0 {
                    Cube::Solid(*b)
                } else {
                    Cube::Solid(*a)
                }
            }
            _ => {
                // At least one is branching - recurse on all children
                Cube::tabulate(|i| {
                    let self_child = match self {
                        Cube::Cubes(children) => (*children[i]).clone(),
                        Cube::Solid(v) => Cube::Solid(*v),
                        _ => Cube::Solid(0),
                    };
                    let other_child = match other {
                        Cube::Cubes(children) => (*children[i]).clone(),
                        Cube::Solid(v) => Cube::Solid(*v),
                        _ => Cube::Solid(0),
                    };
                    self_child.add(&other_child)
                })
                .simplified()
            }
        }
    }

    /// Shift octree to position within parent space
    /// Places this cube at 'pos' within depth 'depth' space
    pub fn shift(&self, depth: u32, pos: IVec3) -> Self {
        let index = (IVec3::ONE - pos.step0()).to_octant_index();
        let mut layer: [Cube<i32>; 8] = std::array::from_fn(|_| Cube::Solid(0));
        layer[index] = self.clone();
        Self::shift_internal(&layer, depth, pos)
    }

    /// Internal helper for shift operation
    fn shift_internal(parent: &[Cube<i32>; 8], depth: u32, pos: IVec3) -> Self {
        if depth == 0 {
            return parent[0].clone();
        }

        // Check if all same (optimization)
        let first = &parent[0];
        if parent[1..]
            .iter()
            .all(|c| std::ptr::eq(c, first) || c == first)
        {
            return first.clone();
        }

        let d = depth - 1;
        let offset = (pos >> d) & 1;

        Cube::tabulate_vector(|v| {
            let next = Self::shift_layer(parent, offset + v);
            Self::shift_internal(&next, d, pos)
        })
        .simplified()
    }

    /// Shift 8-cube layer by offset vector
    fn shift_layer(octants: &[Cube<i32>; 8], offset: IVec3) -> [Cube<i32>; 8] {
        std::array::from_fn(|i| {
            let o: IVec3 = IVec3::from_octant_index(i) + offset;
            let parent: IVec3 = o >> 1;
            let child: IVec3 = o & 1;
            let parent_idx = parent.to_octant_index();
            let child_idx = child.to_octant_index();

            match &octants[parent_idx] {
                Cube::Cubes(children) => (*children[child_idx]).clone(),
                Cube::Solid(v) => Cube::Solid(*v),
                _ => Cube::Solid(0),
            }
        })
    }

    /// Apply swap (non-recursive) - swaps children without recursing
    pub fn apply_swap(&self, axes: &[Axis]) -> Self {
        match self {
            Cube::Solid(v) => Cube::Solid(*v),
            Cube::Cubes(children) => {
                let mut new_children: Vec<Rc<Cube<i32>>> = children.to_vec();

                for axis in axes {
                    match axis {
                        Axis::X => {
                            // Swap along X: a<->e, b<->f, c<->g, d<->h (0<->4, 1<->5, 2<->6, 3<->7)
                            new_children.swap(0, 4);
                            new_children.swap(1, 5);
                            new_children.swap(2, 6);
                            new_children.swap(3, 7);
                        }
                        Axis::Y => {
                            // Swap along Y: a<->c, b<->d, e<->g, f<->h (0<->2, 1<->3, 4<->6, 5<->7)
                            new_children.swap(0, 2);
                            new_children.swap(1, 3);
                            new_children.swap(4, 6);
                            new_children.swap(5, 7);
                        }
                        Axis::Z => {
                            // Swap along Z: a<->b, c<->d, e<->f, g<->h (0<->1, 2<->3, 4<->5, 6<->7)
                            new_children.swap(0, 1);
                            new_children.swap(2, 3);
                            new_children.swap(4, 5);
                            new_children.swap(6, 7);
                        }
                    }
                }

                Cube::cubes(new_children.try_into().unwrap())
            }
            Cube::Planes { axis, quad } => Cube::Planes {
                axis: *axis,
                quad: quad.clone(),
            },
            Cube::Slices { axis, layers } => Cube::Slices {
                axis: *axis,
                layers: layers.clone(),
            },
        }
    }

    /// Apply mirror (recursive) - swaps children and recursively mirrors each child
    pub fn apply_mirror(&self, axes: &[Axis]) -> Self {
        match self {
            Cube::Solid(v) => Cube::Solid(*v),
            Cube::Cubes(children) => {
                let mut new_children: Vec<Rc<Cube<i32>>> = children
                    .iter()
                    .map(|c| Rc::new(c.apply_mirror(axes)))
                    .collect();

                for axis in axes {
                    match axis {
                        Axis::X => {
                            // Mirror along X: swap a<->e, b<->f, c<->g, d<->h (0<->4, 1<->5, 2<->6, 3<->7)
                            new_children.swap(0, 4);
                            new_children.swap(1, 5);
                            new_children.swap(2, 6);
                            new_children.swap(3, 7);
                        }
                        Axis::Y => {
                            // Mirror along Y: a<->c, b<->d, e<->g, f<->h (0<->2, 1<->3, 4<->6, 5<->7)
                            new_children.swap(0, 2);
                            new_children.swap(1, 3);
                            new_children.swap(4, 6);
                            new_children.swap(5, 7);
                        }
                        Axis::Z => {
                            // Mirror along Z: a<->b, c<->d, e<->f, g<->h (0<->1, 2<->3, 4<->5, 6<->7)
                            new_children.swap(0, 1);
                            new_children.swap(2, 3);
                            new_children.swap(4, 5);
                            new_children.swap(6, 7);
                        }
                    }
                }

                Cube::cubes(new_children.try_into().unwrap())
            }
            Cube::Planes { axis, quad } => Cube::Planes {
                axis: *axis,
                quad: quad.clone(),
            },
            Cube::Slices { axis, layers } => Cube::Slices {
                axis: *axis,
                layers: layers.clone(),
            },
        }
    }

    /// Set a voxel at the given position and depth
    ///
    /// # Arguments
    /// * `x`, `y`, `z` - Voxel coordinates in range [0, 2^depth)
    /// * `depth` - Current depth level (0 = leaf)
    /// * `value` - Value to set
    ///
    /// # Returns
    /// A new Cube with the voxel set
    pub fn set_voxel(&self, x: i32, y: i32, z: i32, depth: u32, value: i32) -> Self {
        if depth == 0 {
            // Base case: set leaf value
            return Cube::Solid(value);
        }

        // Determine which octant contains the target position
        let half_size = 1 << (depth - 1); // 2^(depth-1)

        // Calculate octant index based on which half of each axis
        let octant_x = if x >= half_size { 1 } else { 0 };
        let octant_y = if y >= half_size { 1 } else { 0 };
        let octant_z = if z >= half_size { 1 } else { 0 };
        let octant_idx = (octant_x << 2) | (octant_y << 1) | octant_z;

        // Calculate child position (relative to child's coordinate space)
        let child_x = x % half_size;
        let child_y = y % half_size;
        let child_z = z % half_size;

        // Get or create children
        let children: [Rc<Cube<i32>>; 8] = match self {
            Cube::Cubes(existing_children) => {
                // Clone existing children
                existing_children.to_vec().try_into().unwrap()
            }
            Cube::Solid(v) => {
                // Expand solid into 8 children with the same value
                std::array::from_fn(|_| Rc::new(Cube::Solid(*v)))
            }
            _ => {
                // For Planes and Slices, treat as Solid(0)
                std::array::from_fn(|_| Rc::new(Cube::Solid(0)))
            }
        };

        // Update the target child
        let mut new_children = children;
        new_children[octant_idx] = Rc::new(new_children[octant_idx].set_voxel(
            child_x,
            child_y,
            child_z,
            depth - 1,
            value,
        ));

        Cube::Cubes(Box::new(new_children))
    }
}

/// Convert octant char (a-h) to index (0-7)
pub fn octant_char_to_index(c: char) -> Option<usize> {
    match c {
        'a' => Some(0),
        'b' => Some(1),
        'c' => Some(2),
        'd' => Some(3),
        'e' => Some(4),
        'f' => Some(5),
        'g' => Some(6),
        'h' => Some(7),
        _ => None,
    }
}

/// Convert octant index (0-7) to char (a-h)
pub fn octant_index_to_char(index: usize) -> Option<char> {
    match index {
        0 => Some('a'),
        1 => Some('b'),
        2 => Some('c'),
        3 => Some('d'),
        4 => Some('e'),
        5 => Some('f'),
        6 => Some('g'),
        7 => Some('h'),
        _ => None,
    }
}

/// Main octree structure
#[derive(Debug, Clone)]
pub struct Octree {
    pub root: Cube<i32>,
}

impl Octree {
    pub fn new(root: Cube<i32>) -> Self {
        Octree { root }
    }

    pub fn empty() -> Self {
        Octree {
            root: Cube::Solid(0),
        }
    }

    /// Set a voxel at the given position and depth
    ///
    /// # Arguments
    /// * `x`, `y`, `z` - Voxel coordinates in range [0, 2^depth)
    /// * `depth` - Tree depth (0 = single voxel, 4 = 16x16x16 grid)
    /// * `value` - Value to set
    ///
    /// # Returns
    /// A new Octree with the voxel set
    pub fn set_voxel(&self, x: i32, y: i32, z: i32, depth: u32, value: i32) -> Self {
        Octree {
            root: self.root.set_voxel(x, y, z, depth, value),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_octant_conversions() {
        assert_eq!(octant_char_to_index('a'), Some(0));
        assert_eq!(octant_char_to_index('h'), Some(7));
        assert_eq!(octant_char_to_index('z'), None);
        assert_eq!(octant_index_to_char(0), Some('a'));
        assert_eq!(octant_index_to_char(7), Some('h'));
    }

    #[test]
    fn test_simple_cube() {
        let tree = Octree::new(Cube::Solid(42));

        // Use visitor pattern to count voxels
        let mut count = 0;
        tree.root
            .visit_leaves(0, IVec3::ZERO, &mut |cube, _depth, _pos| {
                if let Cube::Solid(value) = cube {
                    if *value > 0 {
                        count += 1;
                    }
                }
            });

        assert_eq!(count, 1);
    }

    #[test]
    fn test_swap_vs_mirror() {
        // Create nested structure: [1 [2 3 4 5 6 7 8 9] 10 11 12 13 14 15]
        let inner = Cube::cubes([
            Rc::new(Cube::Solid(2)),
            Rc::new(Cube::Solid(3)),
            Rc::new(Cube::Solid(4)),
            Rc::new(Cube::Solid(5)),
            Rc::new(Cube::Solid(6)),
            Rc::new(Cube::Solid(7)),
            Rc::new(Cube::Solid(8)),
            Rc::new(Cube::Solid(9)),
        ]);

        let outer = Cube::cubes([
            Rc::new(inner.clone()),
            Rc::new(Cube::Solid(10)),
            Rc::new(Cube::Solid(11)),
            Rc::new(Cube::Solid(12)),
            Rc::new(Cube::Solid(13)),
            Rc::new(Cube::Solid(14)),
            Rc::new(Cube::Solid(15)),
            Rc::new(Cube::Solid(16)),
        ]);

        // Swap: only swaps top-level children, inner structure unchanged
        let swapped = outer.apply_swap(&[Axis::X]);
        if let Cube::Cubes(children) = &swapped {
            // Child 0 and 4 should be swapped
            assert!(matches!(&*children[4], Cube::Cubes(_))); // inner moved to position 4
            assert!(matches!(&*children[0], Cube::Solid(13))); // 13 moved to position 0
        } else {
            panic!("Expected Cubes variant");
        }

        // Mirror: swaps children AND recursively mirrors inner structure
        let mirrored = outer.apply_mirror(&[Axis::X]);
        if let Cube::Cubes(children) = &mirrored {
            // Child 0 and 4 should be swapped
            assert!(matches!(&*children[4], Cube::Cubes(_))); // inner moved to position 4

            // Check that inner structure was also mirrored
            if let Cube::Cubes(inner_children) = &*children[4] {
                // Inner children should be mirrored too
                assert!(matches!(&*inner_children[0], Cube::Solid(6)));
                assert!(matches!(&*inner_children[4], Cube::Solid(2)));
            } else {
                panic!("Expected inner Cubes variant");
            }
        } else {
            panic!("Expected Cubes variant");
        }
    }

    #[test]
    fn test_ivec3_ext() {
        // Test octant index conversions
        assert_eq!(IVec3::from_octant_index(0), IVec3::new(0, 0, 0));
        assert_eq!(IVec3::from_octant_index(7), IVec3::new(1, 1, 1));
        assert_eq!(IVec3::from_octant_index(4), IVec3::new(1, 0, 0));

        assert_eq!(IVec3::new(0, 0, 0).to_octant_index(), 0);
        assert_eq!(IVec3::new(1, 1, 1).to_octant_index(), 7);
        assert_eq!(IVec3::new(1, 0, 0).to_octant_index(), 4);

        // Test step0
        assert_eq!(IVec3::new(0, 0, 0).step0(), IVec3::new(0, 0, 0));
        assert_eq!(IVec3::new(5, -3, 2).step0(), IVec3::new(1, 1, 1));
        assert_eq!(IVec3::new(0, 1, 0).step0(), IVec3::new(0, 1, 0));
    }

    #[test]
    fn test_cube_get() {
        // Create a simple 2-level tree
        let cube = Cube::cubes([
            Rc::new(Cube::Solid(1)),
            Rc::new(Cube::Solid(2)),
            Rc::new(Cube::Solid(3)),
            Rc::new(Cube::Solid(4)),
            Rc::new(Cube::Solid(5)),
            Rc::new(Cube::Solid(6)),
            Rc::new(Cube::Solid(7)),
            Rc::new(Cube::Solid(8)),
        ]);

        // Test get at depth 1
        assert_eq!(cube.get(1, IVec3::new(0, 0, 0)).id(), 1);
        assert_eq!(cube.get(1, IVec3::new(1, 0, 0)).id(), 5);
        assert_eq!(cube.get(1, IVec3::new(1, 1, 1)).id(), 8);
    }

    #[test]
    fn test_cube_updated() {
        let cube = Cube::Solid(0);

        // Update at depth 2, position (1, 0, 0)
        let updated = cube.updated(Cube::Solid(42), 2, IVec3::new(2, 0, 0));

        // Verify the update
        assert_eq!(updated.get(2, IVec3::new(2, 0, 0)).id(), 42);
        assert_eq!(updated.get(2, IVec3::new(0, 0, 0)).id(), 0);
    }

    #[test]
    fn test_cube_simplified() {
        // Create a cube with all children having the same value
        let uniform = Cube::cubes([
            Rc::new(Cube::Solid(5)),
            Rc::new(Cube::Solid(5)),
            Rc::new(Cube::Solid(5)),
            Rc::new(Cube::Solid(5)),
            Rc::new(Cube::Solid(5)),
            Rc::new(Cube::Solid(5)),
            Rc::new(Cube::Solid(5)),
            Rc::new(Cube::Solid(5)),
        ]);

        let simplified = uniform.simplified();
        assert!(matches!(simplified, Cube::Solid(5)));

        // Non-uniform should not simplify
        let non_uniform = Cube::cubes([
            Rc::new(Cube::Solid(5)),
            Rc::new(Cube::Solid(6)),
            Rc::new(Cube::Solid(5)),
            Rc::new(Cube::Solid(5)),
            Rc::new(Cube::Solid(5)),
            Rc::new(Cube::Solid(5)),
            Rc::new(Cube::Solid(5)),
            Rc::new(Cube::Solid(5)),
        ]);

        let not_simplified = non_uniform.simplified();
        assert!(matches!(not_simplified, Cube::Cubes(_)));
    }

    #[test]
    fn test_cube_visit_leaves() {
        let cube = Cube::cubes([
            Rc::new(Cube::Solid(1)),
            Rc::new(Cube::cubes([
                Rc::new(Cube::Solid(2)),
                Rc::new(Cube::Solid(3)),
                Rc::new(Cube::Solid(4)),
                Rc::new(Cube::Solid(5)),
                Rc::new(Cube::Solid(6)),
                Rc::new(Cube::Solid(7)),
                Rc::new(Cube::Solid(8)),
                Rc::new(Cube::Solid(9)),
            ])),
            Rc::new(Cube::Solid(10)),
            Rc::new(Cube::Solid(11)),
            Rc::new(Cube::Solid(12)),
            Rc::new(Cube::Solid(13)),
            Rc::new(Cube::Solid(14)),
            Rc::new(Cube::Solid(15)),
        ]);

        let mut count = 0;
        cube.visit_leaves(2, IVec3::ZERO, &mut |_, _, _| count += 1);

        // Should visit 1 leaf at depth 2, and 8 leaves from nested cube (at depth 1)
        // Plus 6 more solid leaves = 15 total leaves
        assert_eq!(count, 15);
    }

    #[test]
    fn test_cube_add() {
        let a = Cube::Solid(1);
        let b = Cube::Solid(2);

        // Adding two solids prefers non-zero
        let merged = a.add(&b);
        assert_eq!(merged.id(), 2);

        let c = Cube::Solid(0);
        let merged2 = a.add(&c);
        assert_eq!(merged2.id(), 1);
    }

    #[test]
    fn test_cube_tabulate() {
        let cube = Cube::tabulate(|i| Cube::Solid(i as i32));

        // Verify each octant has correct value
        for i in 0..8 {
            if let Some(child) = cube.get_child(i) {
                assert_eq!(child.id(), i as i32);
            }
        }
    }

    #[test]
    fn test_cube_tabulate_vector() {
        let cube = Cube::tabulate_vector(|v| Cube::Solid(v.x + v.y * 2 + v.z * 4));

        // Test a few positions
        assert_eq!(cube.get_child(0).unwrap().id(), 0); // (0,0,0)
        assert_eq!(cube.get_child(7).unwrap().id(), 7); // (1,1,1)
        assert_eq!(cube.get_child(4).unwrap().id(), 1); // (1,0,0)
    }
}
