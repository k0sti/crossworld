use crate::axis::Axis;
use crate::CubeCoord;
use glam::IVec3;
use std::rc::Rc;

/// Pre-computed octant positions for fast lookup
/// Uses binary coordinates: 0 or 1 for each axis
/// Octant indexing: index = x + y*2 + z*4
pub const OCTANT_POSITIONS: [IVec3; 8] = [
    IVec3::new(0, 0, 0), // 0: x=0,y=0,z=0
    IVec3::new(1, 0, 0), // 1: x=1,y=0,z=0
    IVec3::new(0, 1, 0), // 2: x=0,y=1,z=0
    IVec3::new(1, 1, 0), // 3: x=1,y=1,z=0
    IVec3::new(0, 0, 1), // 4: x=0,y=0,z=1
    IVec3::new(1, 0, 1), // 5: x=1,y=0,z=1
    IVec3::new(0, 1, 1), // 6: x=0,y=1,z=1
    IVec3::new(1, 1, 1), // 7: x=1,y=1,z=1
];

/// Extension trait for IVec3 to add octree-specific functionality
pub trait IVec3Ext {
    /// Convert octant index (0-7) to 3D position (each component 0 or 1)
    /// Layout: index = x + y*2 + z*4 = x | (y << 1) | (z << 2)
    fn from_octant_index(index: usize) -> Self;

    /// Convert 3D position to octant index using bit manipulation
    /// Each component should be 0 or 1 (binary coordinates)
    /// Formula: x + y*2 + z*4 = x | (y << 1) | (z << 2)
    fn to_octant_index(self) -> usize;

    /// Step function: returns 0 if component < 0, else 1
    fn step0(self) -> Self;
}

impl IVec3Ext for IVec3 {
    #[inline]
    fn from_octant_index(index: usize) -> Self {
        debug_assert!(index < 8, "Octant index must be 0-7");
        // Use bit extraction: x from bit 0, y from bit 1, z from bit 2
        // index = x + y*2 + z*4
        IVec3::new(
            (index & 1) as i32,
            ((index >> 1) & 1) as i32,
            ((index >> 2) & 1) as i32,
        )
    }

    #[inline]
    fn to_octant_index(self) -> usize {
        // Components must be 0 or 1 (binary coordinates)
        // If you need to convert from center-based (-1/+1), use step0() first
        debug_assert!(
            self.x == 0 || self.x == 1,
            "x component must be 0 or 1, got {}. Use step0() to convert from center-based coords.",
            self.x
        );
        debug_assert!(
            self.y == 0 || self.y == 1,
            "y component must be 0 or 1, got {}. Use step0() to convert from center-based coords.",
            self.y
        );
        debug_assert!(
            self.z == 0 || self.z == 1,
            "z component must be 0 or 1, got {}. Use step0() to convert from center-based coords.",
            self.z
        );
        (self.x as usize) | ((self.y as usize) << 1) | ((self.z as usize) << 2)
    }

    #[inline]
    fn step0(self) -> Self {
        // Return 0 if component < 0, else 1
        IVec3::new(
            (self.x >= 0) as i32,
            (self.y >= 0) as i32,
            (self.z >= 0) as i32,
        )
    }
}

/// 3D cube structure with multiple subdivision strategies
#[derive(Debug, Clone, PartialEq)]
pub enum Cube<T> {
    Solid(T),
    Cubes(Box<[Rc<Cube<T>>; 8]>),
    Quad {
        axis: Axis,
        quads: [Rc<Cube<T>>; 4],
    },
    Layers {
        axis: Axis,
        layers: [Rc<Cube<T>>; 2],
    },
}

impl<T> Cube<T> {
    pub fn solid(value: T) -> Self {
        Cube::Solid(value)
    }

    pub fn cubes(children: [Rc<Cube<T>>; 8]) -> Self {
        Cube::Cubes(Box::new(children))
    }

    pub fn quad(axis: Axis, quads: [Rc<Cube<T>>; 4]) -> Self {
        Cube::Quad { axis, quads }
    }

    pub fn layers(axis: Axis, layers: [Rc<Cube<T>>; 2]) -> Self {
        Cube::Layers { axis, layers }
    }

    /// Check if this cube is a leaf node (not subdivided into octants)
    #[inline]
    pub fn is_leaf(&self) -> bool {
        !matches!(self, Cube::Cubes(_))
    }

    /// Iterate over octant indices (0-7)
    #[inline]
    pub fn octant_indices() -> impl Iterator<Item = usize> {
        0..8
    }

    /// Iterate over octant positions
    #[inline]
    pub fn octant_positions() -> impl Iterator<Item = IVec3> {
        OCTANT_POSITIONS.iter().copied()
    }

    /// Calculate octant index at given depth for position
    /// Returns which octant (0-7) the position falls into at this depth level
    /// Extracts the bit at the given depth level for each axis to determine octant
    #[inline]
    pub fn index(depth: u32, pos: IVec3) -> usize {
        // Extract the bit at this depth level for each axis
        // This works for both positive corner-based coordinates (0..2^n)
        // and center-based coordinates (sign indicates octant)
        let p = (pos >> depth) & 1; // Extract bit at depth level (0 or 1)
        p.to_octant_index()
    }

    /// Get child cube by octant index (0-7)
    /// Octant layout: a=0 (x-,y-,z-) to h=7 (x+,y+,z+)
    #[inline]
    pub fn get_child(&self, index: usize) -> Option<&Rc<Cube<T>>> {
        match self {
            Cube::Cubes(children) if index < 8 => Some(&children[index]),
            _ => None,
        }
    }

    /// Get child or return self for uniform structures
    #[inline]
    fn get_child_or_self(&self, index: usize) -> &Cube<T> {
        match self {
            Cube::Cubes(children) if index < 8 => &children[index],
            _ => self, // Solid/Planes/Slices act as uniform
        }
    }

    /// Get cube at specific coordinate
    pub fn get(&self, cube_coord: CubeCoord) -> &Cube<T> {
        if cube_coord.depth == 0 {
            self
        } else {
            let d = cube_coord.depth - 1;
            let index = Self::index(d, cube_coord.pos);
            let child_coord = CubeCoord::new(cube_coord.pos, d);
            self.get_child_or_self(index).get(child_coord)
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
                        // Convert 0/1 octant coords to center-based: 2*octant - 1
                        let octant_offset = IVec3::from_octant_index(i) * 2 - IVec3::ONE;
                        let p = pos * 2 + octant_offset;
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
                        // Convert 0/1 octant coords to center-based: 2*octant - 1
                        let octant_offset = IVec3::from_octant_index(i) * 2 - IVec3::ONE;
                        let p = pos * 2 + octant_offset;
                        children[i].visit_deep(d, p, callback);
                    }
                }
                _ => {
                    // For non-branching nodes, treat as uniform and recurse
                    for i in 0..8 {
                        let d = depth - 1;
                        // Convert 0/1 octant coords to center-based: 2*octant - 1
                        let octant_offset = IVec3::from_octant_index(i) * 2 - IVec3::ONE;
                        let p = pos * 2 + octant_offset;
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
    /// Positions are in center-based coordinates (-1 or +1 for each axis)
    pub fn tabulate_vector<F>(init: F) -> Self
    where
        F: Fn(IVec3) -> Cube<T>,
    {
        Cube::cubes(std::array::from_fn(|i| {
            // Convert 0/1 octant coords to center-based: 2*octant - 1
            let pos = IVec3::from_octant_index(i) * 2 - IVec3::ONE;
            Rc::new(init(pos))
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

    /// Get default ID for non-Cubes variants
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

    /// Update this cube with subcube at cube_coord
    ///
    /// # Depth and Position Scaling
    /// - depth: Octree depth level (0 = replace entire cube, higher = finer subdivision)
    /// - pos: Position at the CURRENT depth level, range [0, 2^depth - 1]
    /// - When recursing, position must be adjusted for child's coordinate space
    pub fn update(&self, cube_coord: CubeCoord, cube: Cube<T>) -> Self {
        if cube_coord.depth == 0 {
            // Depth 0: replace this entire cube
            cube
        } else {
            // Recurse into child octant
            let d = cube_coord.depth - 1;

            // Calculate which octant this position falls into at depth d
            // index uses bit shift: (pos >> d) & 1 extracts the bit at depth d
            let index = Self::index(d, cube_coord.pos);

            // Get the child cube (or expand if solid)
            let child = self.get_child_or_self(index);

            // Recursively update the child with same position but decremented depth
            // The position stays the same - the index function handles extracting
            // the correct octant bit at each depth level
            let child_coord = CubeCoord::new(cube_coord.pos, d);
            let new_child = child.update(child_coord, cube);

            // Replace the child in this cube's children array
            self.updated_index(index, new_child)
        }
    }

    /// Update this cube with cube at depth and offset, scaled with given scale depth
    ///
    /// Places the source cube at the target depth and offset, where the source cube
    /// occupies 2^scale voxels in each dimension at the target depth.
    ///
    /// Example: update_depth(2, (0, 2, 0), 1, cube) places cube at depth 2
    /// covering positions x=[0,1], y=[2,3], z=[0,1]
    pub fn update_depth(&self, depth: u32, offset: IVec3, scale: u32, cube: Cube<T>) -> Self {
        if scale == 0 {
            self.update(CubeCoord::new(offset, depth), cube)
        } else {
            let mut result = self.clone();
            let size = 1 << scale; // 2^scale

            for z in 0..size {
                for y in 0..size {
                    for x in 0..size {
                        let target_pos = offset + IVec3::new(x, y, z);
                        let source_pos = IVec3::new(x, y, z);
                        let source_coord = CubeCoord::new(source_pos, scale);
                        let subcube = (*cube.get(source_coord)).clone();
                        let target_coord = CubeCoord::new(target_pos, depth);
                        result = result.update(target_coord, subcube);
                    }
                }
            }

            result
        }
    }

    /// Recursive version: Update this cube with cube at depth and offset, scaled with given scale depth
    ///
    /// More efficient than update_depth as it recursively traverses the octree structure
    /// instead of iterating through all leaf positions.
    ///
    /// Places the source cube at the target depth and offset, where the source cube
    /// occupies 2^scale voxels in each dimension at the target depth.
    ///
    /// Example: update_depth_tree(2, (0, 2, 0), 1, cube) places cube at depth 2
    /// covering positions x=[0,1], y=[2,3], z=[0,1]
    pub fn update_depth_tree(&self, depth: u32, offset: IVec3, scale: u32, cube: &Cube<T>) -> Self {
        if scale == 0 {
            self.update(CubeCoord::new(offset, depth), cube.clone())
        } else {
            let mut result = self.clone();
            let half_size = 1 << (scale - 1); // 2^(scale-1)

            // Process each octant
            for octant_idx in 0..8 {
                // Octant position is 0/1, use directly for offset calculation
                let octant_pos = IVec3::from_octant_index(octant_idx);
                let target_offset = offset + octant_pos * half_size;

                // Get the corresponding child from source
                let source_child = match cube {
                    Cube::Cubes(children) => children[octant_idx].as_ref(),
                    Cube::Solid(_) => cube, // Uniform cube, use same value for all octants
                    _ => cube,              // Planes/Slices treated as uniform
                };

                result = result.update_depth_tree(depth, target_offset, scale - 1, source_child);
            }

            result
        }
    }
}

impl<T: Clone + PartialEq> Cube<T> {
    /// Simplify cube by collapsing uniform children into a single leaf
    /// This is recursive - first simplifies all children, then checks if parent can be simplified
    pub fn simplified(self) -> Self {
        match self {
            Cube::Cubes(children) => {
                // First, recursively simplify all children
                let simplified_children: [Rc<Cube<T>>; 8] = std::array::from_fn(|i| {
                    let child = (*children[i]).clone();
                    Rc::new(child.simplified())
                });

                // Now check if all simplified children are Solid with the same value
                if let Cube::Solid(first_val) = &*simplified_children[0] {
                    let all_same = simplified_children[1..]
                        .iter()
                        .all(|c| matches!(&**c, Cube::Solid(v) if v == first_val));
                    if all_same {
                        return Cube::Solid(first_val.clone());
                    }
                }

                // Return with simplified children
                Cube::Cubes(Box::new(simplified_children))
            }
            _ => self,
        }
    }
}

impl Cube<u8> {
    /// Get ID value for this cube
    #[inline]
    pub fn id(&self) -> u8 {
        match self {
            Cube::Solid(v) => *v,
            Cube::Cubes(children) => {
                // Return most common ID among children (like Scala version)
                let ids: Vec<u8> = children.iter().map(|c| c.id()).collect();
                // Simple mode calculation - just return first for now
                ids[0]
            }
            _ => 0,
        }
    }

    /// Get ID at specific position and depth
    #[inline]
    pub fn get_id(&self, depth: u32, pos: IVec3) -> u8 {
        self.get(CubeCoord::new(pos, depth)).id()
    }

    /// Merge two cubes (union operation with preference for non-empty)
    pub fn add(&self, other: &Cube<u8>) -> Self {
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
        // step0 returns 0/1, invert with (1 - x) to get opposite octant
        let index = (IVec3::ONE - pos.step0()).to_octant_index();
        let mut layer: [Cube<u8>; 8] = std::array::from_fn(|_| Cube::Solid(0));
        layer[index] = self.clone();
        Self::shift_internal(&layer, depth, pos)
    }

    /// Internal helper for shift operation
    fn shift_internal(parent: &[Cube<u8>; 8], depth: u32, pos: IVec3) -> Self {
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
    fn shift_layer(octants: &[Cube<u8>; 8], offset: IVec3) -> [Cube<u8>; 8] {
        std::array::from_fn(|i| {
            // from_octant_index returns 0/1, offset is also in 0/1-based coords
            let o: IVec3 = IVec3::from_octant_index(i) + offset;
            let parent: IVec3 = o >> 1;  // Parent octant (still 0/1)
            let child: IVec3 = o & 1;     // Child within parent (0/1)
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
                let mut new_children: Vec<Rc<Cube<u8>>> = children.to_vec();

                for axis in axes {
                    match axis {
                        Axis::PosX | Axis::NegX => {
                            // Swap along X: a<->e, b<->f, c<->g, d<->h (0<->4, 1<->5, 2<->6, 3<->7)
                            new_children.swap(0, 4);
                            new_children.swap(1, 5);
                            new_children.swap(2, 6);
                            new_children.swap(3, 7);
                        }
                        Axis::PosY | Axis::NegY => {
                            // Swap along Y: a<->c, b<->d, e<->g, f<->h (0<->2, 1<->3, 4<->6, 5<->7)
                            new_children.swap(0, 2);
                            new_children.swap(1, 3);
                            new_children.swap(4, 6);
                            new_children.swap(5, 7);
                        }
                        Axis::PosZ | Axis::NegZ => {
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
            Cube::Quad { axis, quads } => Cube::Quad {
                axis: *axis,
                quads: quads.clone(),
            },
            Cube::Layers { axis, layers } => Cube::Layers {
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
                let mut new_children: Vec<Rc<Cube<u8>>> = children
                    .iter()
                    .map(|c| Rc::new(c.apply_mirror(axes)))
                    .collect();

                for axis in axes {
                    match axis {
                        Axis::PosX | Axis::NegX => {
                            // Mirror along X: swap a<->e, b<->f, c<->g, d<->h (0<->4, 1<->5, 2<->6, 3<->7)
                            new_children.swap(0, 4);
                            new_children.swap(1, 5);
                            new_children.swap(2, 6);
                            new_children.swap(3, 7);
                        }
                        Axis::PosY | Axis::NegY => {
                            // Mirror along Y: a<->c, b<->d, e<->g, f<->h (0<->2, 1<->3, 4<->6, 5<->7)
                            new_children.swap(0, 2);
                            new_children.swap(1, 3);
                            new_children.swap(4, 6);
                            new_children.swap(5, 7);
                        }
                        Axis::PosZ | Axis::NegZ => {
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
            Cube::Quad { axis, quads } => Cube::Quad {
                axis: *axis,
                quads: quads.clone(),
            },
            Cube::Layers { axis, layers } => Cube::Layers {
                axis: *axis,
                layers: layers.clone(),
            },
        }
    }

    /// Create an empty cube (all air/zeros)
    pub fn empty() -> Self {
        Cube::Solid(0)
    }

    /// Set a voxel at the given position and depth
    ///
    /// # Arguments
    /// * `x`, `y`, `z` - Voxel coordinates in range [0, 2^depth)
    /// * `depth` - Tree depth (0 = single voxel, 4 = 16x16x16 grid)
    /// * `value` - Value to set
    ///
    /// # Returns
    /// A new Cube with the voxel set
    pub fn set_voxel(&self, x: i32, y: i32, z: i32, depth: u32, value: u8) -> Self {
        self.update(
            CubeCoord {
                pos: IVec3 { x, y, z },
                depth,
            },
            Cube::solid(value),
        )
    }

    /// Expand a cube once by wrapping it with border layers
    ///
    /// Creates a 4x4x4 grid (depth 2) where:
    /// - Center 2x2x2 region contains the original cube
    /// - Border voxels are filled with materials based on Y level
    ///
    /// # Arguments
    /// * `root` - The cube to expand
    /// * `border_materials` - Array of 4 material IDs for border voxels at each Y layer [y0, y1, y2, y3]
    ///
    /// # Returns
    /// The expanded cube (one layer added)
    pub fn expand_once(root: &Cube<u8>, border_materials: [u8; 4]) -> Self {
        // Create two-level octree using direct octant index iteration
        Cube::tabulate(|i1| {
            Cube::tabulate(|i2| {
                // Check if we're in the center region using XOR condition
                // i1 ^ i2 ^ 0x7 == 0 means i1 and i2 are bitwise complements
                // This identifies the center 2x2x2 region in the 4x4x4 grid
                if i1 ^ i2 ^ 0x7 == 0 {
                    // Use i1 as the root octant index
                    // Global position = i1*2 + i2, center offset = global - 1 = i1
                    root.get_child_or_self(i1).clone()
                } else {
                    // In border region - use border material based on Y level
                    // Extract Y bit from each index: bit 1 is Y coordinate
                    let y_pos = ((i1 >> 1) & 1) * 2 + ((i2 >> 1) & 1);
                    Cube::Solid(border_materials[y_pos])
                }
            })
        })
    }
    /// Expand a cube by wrapping it with border layers multiple times
    ///
    /// Each expansion layer doubles the cube's size by surrounding it with border materials.
    /// The expansion creates nested 4x4x4 grids where:
    /// - Border voxels are filled with materials based on Y level
    /// - The original cube is placed in progressively larger center regions
    ///
    /// # Arguments
    /// * `root` - The cube to expand
    /// * `border_materials` - Array of 4 material IDs for border voxels at each Y layer [y0, y1, y2, y3]
    ///   - For world terrain: [hard_rock, water, air, air] or similar
    ///   - For avatars/objects: [0, 0, 0, 0] for empty borders
    /// * `depth` - Number of expansion layers (each layer doubles the size)
    ///
    /// # Returns
    /// The expanded cube
    ///
    /// # Example
    /// ```
    /// use cube::Cube;
    ///
    /// let original = Cube::Solid(5);
    /// let border_materials = [16, 17, 0, 0]; // bedrock, water, air, air
    /// let expanded = Cube::expand(&original, border_materials, 2);
    /// // After 2 expansions, the original is wrapped twice
    /// ```
    pub fn expand(root: &Cube<u8>, border_materials: [u8; 4], depth: i32) -> Self {
        let mut result = root.clone();
        for _ in 0..depth {
            result = Self::expand_once(&result, border_materials);
        }
        result
    }

    /// Calculate the maximum depth of the octree
    pub fn max_depth(&self) -> usize {
        match self {
            Cube::Solid(_) => 0,
            Cube::Cubes(children) => {
                1 + children
                    .iter()
                    .map(|c| c.max_depth())
                    .max()
                    .unwrap_or(0)
            }
            _ => 0,
        }
    }

    /// Count nodes at each depth level
    /// Returns a Vec where index i contains the count of nodes at depth i
    pub fn count_nodes_by_depth(&self) -> Vec<usize> {
        let max_depth = self.max_depth();
        let mut counts = vec![0; max_depth + 1];
        self.count_nodes_by_depth_internal(0, &mut counts);
        counts
    }

    fn count_nodes_by_depth_internal(&self, current_depth: usize, counts: &mut Vec<usize>) {
        counts[current_depth] += 1;
        if let Cube::Cubes(children) = self {
            for child in children.iter() {
                child.count_nodes_by_depth_internal(current_depth + 1, counts);
            }
        }
    }

    /// Collect all unique materials used in the cube
    pub fn collect_materials(&self) -> std::collections::HashSet<u8> {
        let mut materials = std::collections::HashSet::new();
        self.collect_materials_internal(&mut materials);
        materials
    }

    fn collect_materials_internal(&self, materials: &mut std::collections::HashSet<u8>) {
        match self {
            Cube::Solid(val) => {
                materials.insert(*val);
            }
            Cube::Cubes(children) => {
                for child in children.iter() {
                    child.collect_materials_internal(materials);
                }
            }
            _ => {}
        }
    }

    /// Print debug statistics about the cube structure
    pub fn print_debug_stats(&self, name: &str) {
        eprintln!("\n=== Cube Debug Stats: {} ===", name);

        // Max depth
        let max_depth = self.max_depth();
        eprintln!("Max depth: {}", max_depth);
        eprintln!("Cube size: {}^3", 1 << max_depth);

        // Nodes per depth
        let nodes_by_depth = self.count_nodes_by_depth();
        eprintln!("Nodes by depth:");
        for (depth, count) in nodes_by_depth.iter().enumerate() {
            if *count > 0 {
                eprintln!("  Depth {}: {} nodes", depth, count);
            }
        }

        // Total nodes
        let total_nodes: usize = nodes_by_depth.iter().sum();
        eprintln!("Total nodes: {}", total_nodes);

        // Materials
        let materials = self.collect_materials();
        let mut materials_vec: Vec<u8> = materials.iter().copied().collect();
        materials_vec.sort();
        eprintln!("Unique materials: {} - {:?}", materials.len(), materials_vec);

        // Root type
        match self {
            Cube::Solid(val) => eprintln!("Root: Solid({})", val),
            Cube::Cubes(_) => eprintln!("Root: Cubes (octree)"),
            Cube::Quad { .. } => eprintln!("Root: Quad"),
            Cube::Layers { .. } => eprintln!("Root: Layers"),
        }

        eprintln!("=========================\n");
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
        let cube = Cube::Solid(42);

        // Use visitor pattern to count voxels
        let mut count = 0;
        cube.visit_leaves(0, IVec3::ZERO, &mut |cube, _depth, _pos| {
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
        let swapped = outer.apply_swap(&[Axis::PosX]);
        if let Cube::Cubes(children) = &swapped {
            // Child 0 and 4 should be swapped
            assert!(matches!(&*children[4], Cube::Cubes(_))); // inner moved to position 4
            assert!(matches!(&*children[0], Cube::Solid(13))); // 13 moved to position 0
        } else {
            panic!("Expected Cubes variant");
        }

        // Mirror: swaps children AND recursively mirrors inner structure
        let mirrored = outer.apply_mirror(&[Axis::PosX]);
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
        // Test octant index conversions (binary: 0 or 1)
        // index = x + y*2 + z*4
        assert_eq!(IVec3::from_octant_index(0), IVec3::new(0, 0, 0));
        assert_eq!(IVec3::from_octant_index(1), IVec3::new(1, 0, 0));
        assert_eq!(IVec3::from_octant_index(2), IVec3::new(0, 1, 0));
        assert_eq!(IVec3::from_octant_index(4), IVec3::new(0, 0, 1));
        assert_eq!(IVec3::from_octant_index(7), IVec3::new(1, 1, 1));

        assert_eq!(IVec3::new(0, 0, 0).to_octant_index(), 0);
        assert_eq!(IVec3::new(1, 0, 0).to_octant_index(), 1);
        assert_eq!(IVec3::new(0, 1, 0).to_octant_index(), 2);
        assert_eq!(IVec3::new(0, 0, 1).to_octant_index(), 4);
        assert_eq!(IVec3::new(1, 1, 1).to_octant_index(), 7);

        // Test step0 (binary: return 0 or 1)
        assert_eq!(IVec3::new(0, 0, 0).step0(), IVec3::new(1, 1, 1)); // Zero maps to 1
        assert_eq!(IVec3::new(5, -3, 2).step0(), IVec3::new(1, 0, 1));
        assert_eq!(IVec3::new(-1, 1, -1).step0(), IVec3::new(0, 1, 0));
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

        // Test get at depth 1 (center-based: positions are -1 or +1)
        // With new indexing: index = x + y*2 + z*4 where x,y,z are 0/1 from step0()
        assert_eq!(
            <Cube<u8>>::id(&cube.get(CubeCoord::new(IVec3::new(-1, -1, -1), 1))),
            1
        ); // (-1,-1,-1) → (0,0,0) → index 0 → value 1
        assert_eq!(
            <Cube<u8>>::id(&cube.get(CubeCoord::new(IVec3::new(1, -1, -1), 1))),
            2
        ); // (1,-1,-1) → (1,0,0) → index 1 → value 2
        assert_eq!(
            <Cube<u8>>::id(&cube.get(CubeCoord::new(IVec3::new(1, 1, 1), 1))),
            8
        ); // (1,1,1) → (1,1,1) → index 7 → value 8
    }

    #[test]
    fn test_cube_update() {
        let cube = Cube::Solid(0);

        // Update at depth 2, position (3, -1, -1) - center-based
        // At depth 2, valid positions are {-3, -1, +1, +3} for each axis
        let updated = cube.update(CubeCoord::new(IVec3::new(3, -1, -1), 2), Cube::Solid(42));

        // Verify the update
        assert_eq!(
            <Cube<u8>>::id(&updated.get(CubeCoord::new(IVec3::new(3, -1, -1), 2))),
            42
        );
        assert_eq!(
            <Cube<u8>>::id(&updated.get(CubeCoord::new(IVec3::new(-3, -3, -3), 2))),
            0
        );
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
        assert_eq!(<Cube<u8>>::id(&merged), 2);

        let c = Cube::Solid(0);
        let merged2 = a.add(&c);
        assert_eq!(<Cube<u8>>::id(&merged2), 1);
    }

    #[test]
    fn test_cube_tabulate() {
        let cube = Cube::tabulate(|i| Cube::Solid(i as u8));

        // Verify each octant has correct value
        for i in 0..8 {
            if let Some(child) = cube.get_child(i) {
                assert_eq!(<Cube<u8>>::id(child), i as u8);
            }
        }
    }

    #[test]
    fn test_cube_tabulate_vector() {
        let cube = Cube::tabulate_vector(|v| {
            // v is center-based coordinates (-1 or +1 for each axis)
            // Map to positive values for u8
            let val = ((v.x + 1) + (v.y + 1) * 4 + (v.z + 1) * 16) as u8;
            Cube::Solid(val)
        });

        // Test a few positions
        // Octant 0: from_octant_index(0) = (0,0,0) => center-based (-1,-1,-1) => mapped (0, 0, 0) => 0
        assert_eq!(<Cube<u8>>::id(cube.get_child(0).unwrap()), 0);
        // Octant 7: from_octant_index(7) = (1,1,1) => center-based (1,1,1) => mapped (2, 2, 2) => 2 + 8 + 32 = 42
        assert_eq!(<Cube<u8>>::id(cube.get_child(7).unwrap()), 42);
    }

    #[test]
    fn test_update_depth_vs_update_depth_tree() {
        // Create a source cube with distinct values in each octant
        let source = Cube::cubes([
            Rc::new(Cube::Solid(1)),
            Rc::new(Cube::Solid(2)),
            Rc::new(Cube::Solid(3)),
            Rc::new(Cube::Solid(4)),
            Rc::new(Cube::Solid(5)),
            Rc::new(Cube::Solid(6)),
            Rc::new(Cube::Solid(7)),
            Rc::new(Cube::Solid(8)),
        ]);

        let target = Cube::Solid(0);
        let offset = IVec3::new(0, 2, 0);
        let depth = 3;
        let scale = 1;

        // Update using both methods
        let result1 = target.update_depth(depth, offset, scale, source.clone());
        let result2 = target.update_depth_tree(depth, offset, scale, &source);

        // Verify all positions match
        let size = 1 << scale;
        for z in 0..size {
            for y in 0..size {
                for x in 0..size {
                    let pos = offset + IVec3::new(x, y, z);
                    let id1 = <Cube<u8>>::id(&result1.get(CubeCoord::new(pos, depth)));
                    let id2 = <Cube<u8>>::id(&result2.get(CubeCoord::new(pos, depth)));
                    assert_eq!(
                        id1, id2,
                        "Mismatch at position {:?}: update_depth={}, update_depth_tree={}",
                        pos, id1, id2
                    );
                }
            }
        }
    }

    #[test]
    fn test_update_depth_tree_nested() {
        // Test with scale=2 (4x4x4 region)
        let source = Cube::cubes([
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
            Rc::new(Cube::Solid(20)),
            Rc::new(Cube::Solid(30)),
            Rc::new(Cube::Solid(40)),
            Rc::new(Cube::Solid(50)),
            Rc::new(Cube::Solid(60)),
            Rc::new(Cube::Solid(70)),
            Rc::new(Cube::Solid(80)),
        ]);

        let target = Cube::Solid(0);
        let offset = IVec3::new(0, 0, 0);
        let depth = 3;
        let scale = 2;

        // Test both methods produce same result
        let result_loop = target.update_depth(depth, offset, scale, source.clone());
        let result_tree = target.update_depth_tree(depth, offset, scale, &source);

        // Verify all positions match between methods
        let size = 1 << scale;
        for z in 0..size {
            for y in 0..size {
                for x in 0..size {
                    let pos = offset + IVec3::new(x, y, z);
                    let id_loop = <Cube<u8>>::id(&result_loop.get(CubeCoord::new(pos, depth)));
                    let id_tree = <Cube<u8>>::id(&result_tree.get(CubeCoord::new(pos, depth)));
                    assert_eq!(
                        id_loop, id_tree,
                        "Mismatch at position {:?}: update_depth={}, update_depth_tree={}",
                        pos, id_loop, id_tree
                    );
                }
            }
        }
    }


    #[test]
    fn test_expand() {
        // Test basic expansion
        let root = Cube::Solid(5);
        let border_materials = [16, 17, 0, 0]; // bedrock, water, air, air

        // Expand once
        let expanded = Cube::expand(&root, border_materials, 1);

        // The expand function creates a 4x4x4 grid (depth 2) with:
        // - Border materials based on Y level
        // - Original cube placed in center via update_depth
        //
        // Note: update_depth can create subdivisions at non-standard positions,
        // so we just verify the overall structure is reasonable.

        // Check that the bottom corner has bedrock (Y level 0)
        let bottom = expanded.get(CubeCoord::new(IVec3::new(-3, -3, -3), 2));
        assert_eq!(*bottom, Cube::Solid(16), "Bottom corner should be bedrock");

        // Check that top levels have air (Y level 3)
        let top = expanded.get(CubeCoord::new(IVec3::new(-3, 3, -3), 2));
        assert_eq!(*top, Cube::Solid(0), "Top should be air");

        // The structure should be a Cubes variant (not solid/uniform)
        assert!(matches!(expanded, Cube::Cubes(_)), "Expanded cube should be subdivided");
    }

    #[test]
    fn test_expand_zero_depth() {
        let root = Cube::Solid(42);
        let border_materials = [1, 2, 3, 4];

        // Zero depth should return clone of original
        let result = Cube::expand(&root, border_materials, 0);
        assert_eq!(result, root);
    }

    #[test]
    fn test_expand_multiple_layers() {
        let root = Cube::Solid(10);
        let border_materials = [1, 2, 3, 4];

        // Expand twice
        let expanded = Cube::expand(&root, border_materials, 2);

        // After 2 expansions, the depth is 4
        // Bottom corner at depth 4 should have Y=0 material (border_materials[0])

        // Calculate bottom corner position (follow child[0] path 4 times)
        let coord_root = CubeCoord::new(IVec3::new(0, 0, 0), 0);
        let mut coord = coord_root;
        for _ in 0..4 {
            coord = coord.child(0);  // Keep going to child[0] (bottom-left-back)
        }

        let bottom = expanded.get(coord);
        assert_eq!(*bottom, Cube::Solid(1), "Bottom should have Y=0 material");
    }
}
