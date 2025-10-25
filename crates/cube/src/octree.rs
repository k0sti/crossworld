use std::rc::Rc;

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
#[derive(Debug, Clone)]
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
#[derive(Debug, Clone)]
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

    /// Get child cube by octant index (0-7)
    /// Octant layout: a=0 (x-,y-,z-) to h=7 (x+,y+,z+)
    pub fn get_child(&self, index: usize) -> Option<&Rc<Cube<T>>> {
        match self {
            Cube::Cubes(children) if index < 8 => Some(&children[index]),
            _ => None,
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
}

impl Cube<i32> {
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
                existing_children.iter().cloned().collect::<Vec<_>>().try_into().unwrap()
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
        new_children[octant_idx] = Rc::new(
            new_children[octant_idx].set_voxel(child_x, child_y, child_z, depth - 1, value)
        );

        Cube::Cubes(Box::new(new_children))
    }

    /// Traverse and collect all leaf voxels with positions
    pub fn collect_voxels(
        &self,
        position: (f32, f32, f32),
        size: f32,
        voxels: &mut Vec<(f32, f32, f32, f32, i32)>,
    ) {
        match self {
            Cube::Solid(value) => {
                if *value > 0 {
                    voxels.push((position.0, position.1, position.2, size, *value));
                }
            }
            Cube::Cubes(children) => {
                let half_size = size / 2.0;
                for (idx, child) in children.iter().enumerate() {
                    let offset = octant_offset(idx);
                    let child_pos = (
                        position.0 + offset.0 * size,
                        position.1 + offset.1 * size,
                        position.2 + offset.2 * size,
                    );
                    child.collect_voxels(child_pos, half_size, voxels);
                }
            }
            Cube::Planes { .. } => {
                // TODO: Implement plane subdivision voxel collection
            }
            Cube::Slices { .. } => {
                // TODO: Implement slice subdivision voxel collection
            }
        }
    }
}

/// Get the offset for octant index within parent cube
/// Returns (x_offset, y_offset, z_offset) where each is 0 or 0.5
fn octant_offset(index: usize) -> (f32, f32, f32) {
    let x = if index & 0b100 != 0 { 0.5 } else { 0.0 };
    let y = if index & 0b010 != 0 { 0.5 } else { 0.0 };
    let z = if index & 0b001 != 0 { 0.5 } else { 0.0 };
    (x, y, z)
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

    pub fn collect_voxels(&self) -> Vec<(f32, f32, f32, f32, i32)> {
        let mut voxels = Vec::new();
        self.root.collect_voxels((0.0, 0.0, 0.0), 1.0, &mut voxels);
        voxels
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
    fn test_octant_offset() {
        assert_eq!(octant_offset(0), (0.0, 0.0, 0.0));
        assert_eq!(octant_offset(7), (0.5, 0.5, 0.5));
        assert_eq!(octant_offset(4), (0.5, 0.0, 0.0));
    }

    #[test]
    fn test_simple_cube() {
        let tree = Octree::new(Cube::Solid(42));
        let voxels = tree.collect_voxels();
        assert_eq!(voxels.len(), 1);
        assert_eq!(voxels[0], (0.0, 0.0, 0.0, 1.0, 42));
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
}
