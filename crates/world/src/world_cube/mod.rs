mod builder;

use crate::GeometryData;
use cube::{ColorMapper, Cube, CubeBox, CubeCoord, DefaultMeshBuilder, glam::IVec3, serialize_csm};
use noise::{Fbm, Perlin};

/// World container that can expand to accommodate models
pub struct World {
    cube: Cube<u8>,
    scale: u32,  // World scale: actual world size = 2^(macro_depth + scale)
}

impl World {
    /// Create new World from a cube at macro_depth
    pub fn new(cube: Cube<u8>, macro_depth: u32) -> Self {
        Self {
            cube,
            scale: macro_depth,
        }
    }

    /// Get the current world scale
    pub fn scale(&self) -> u32 {
        self.scale
    }

    /// Get reference to the root cube
    pub fn root(&self) -> &Cube<u8> {
        &self.cube
    }

    /// Set a new root cube
    pub fn set_root(&mut self, cube: Cube<u8>) {
        self.cube = cube.simplified();
    }

    /// Expand world by one level with sky/ground borders
    /// Doubles the world size by wrapping in border materials
    fn expand_once(&mut self) {
        const HARD_GROUND: u8 = 16;
        const WATER: u8 = 17;
        const AIR: u8 = 0;

        let border_materials = [HARD_GROUND, WATER, AIR, AIR];
        self.cube = Cube::expand(&self.cube, border_materials, 1);
        self.scale += 1;
    }

    /// Ensure the world can fit a model at given position
    /// Expands world if necessary
    /// Returns the adjusted position in world coordinates
    pub fn ensure_fits(&mut self, model_size: IVec3, world_pos: IVec3) -> IVec3 {
        let world_size = 1 << self.scale;
        let half_size = world_size / 2;

        // Calculate required bounds
        let min = world_pos;
        let max = world_pos + model_size;

        // Check if model fits within current world bounds [-half_size, half_size)
        let needs_expansion = min.x < -half_size
            || min.y < -half_size
            || min.z < -half_size
            || max.x > half_size
            || max.y > half_size
            || max.z > half_size;

        if needs_expansion {
            self.expand_once();
            // Recursively check again with expanded world
            self.ensure_fits(model_size, world_pos)
        } else {
            world_pos
        }
    }

    /// Merge a model into the world at specified position
    /// Position is in world coordinates
    /// Automatically expands world if model doesn't fit
    pub fn merge_model(&mut self, model: &CubeBox<u8>, world_pos: IVec3, depth: u32) {
        let model_size = IVec3::new(model.size.x, model.size.y, model.size.z);
        self.ensure_fits(model_size, world_pos);

        // Convert world position to octree coordinates at the target depth
        let world_size = 1 << self.scale;
        let half_size = world_size / 2;

        // Place each voxel from the model
        // This is a simplified implementation - a more efficient version would
        // merge the entire octree structure
        for y in 0..model.size.y {
            for z in 0..model.size.z {
                for x in 0..model.size.x {
                    let model_coord = CubeCoord::new(
                        IVec3::new(x, y, z),
                        model.depth,
                    );

                    let material_cube = model.cube.get(model_coord);
                    if let Some(&material) = material_cube.value()
                        && material > 0 {  // Skip empty voxels
                            // World position for this voxel
                            let voxel_world_pos = world_pos + IVec3::new(x, y, z);

                            // Convert to octree coordinates
                            let octree_x = voxel_world_pos.x + half_size;
                            let octree_y = voxel_world_pos.y + half_size;
                            let octree_z = voxel_world_pos.z + half_size;

                            // Update the world cube
                            let coord = CubeCoord::new(IVec3::new(octree_x, octree_y, octree_z), depth);
                            self.cube = self.cube.update(coord, Cube::Solid(material)).simplified();
                        }
                }
            }
        }
    }
}

pub struct WorldCube {
    cube: Cube<u8>,
    macro_depth: u32,  // World size = 2^macro_depth, terrain generation depth
    render_depth: u32, // Maximum traversal depth for mesh generation
    border_depth: u32, // Number of border cube layers
}

impl WorldCube {
    /// Create new WorldCube with specified macro depth
    ///
    /// # Arguments
    /// * `macro_depth` - World size depth (e.g., 3 = 8×8×8 world units)
    /// * `micro_depth` - Additional subdivision levels for user edits (0-3)
    /// * `border_depth` - Number of border cube layers (0 = no border, 1+ = wrap in border octants)
    ///
    /// Architecture (macro_depth=3, micro_depth=3):
    /// - World size: 8×8×8 units (2^3)
    /// - Terrain generated at macro depth
    /// - User voxels can be placed up to macro+micro depth (depth 6)
    /// - Octree dynamically subdivides for finer edits
    ///
    /// Border layers:
    /// - Each border layer wraps the world in an octa (8 cubes)
    /// - 4 bottom cubes + 4 top cubes surround the world
    /// - Original world placed at octant 0 (bottom-front-left)
    pub fn new(macro_depth: u32, micro_depth: u32, border_depth: u32, seed: u32) -> Self {
        tracing::info!("WorldCube::new called with seed: {}", seed);
        let noise = Perlin::new(seed);
        let fbm = Fbm::new(seed);

        // Build octree with terrain at macro depth
        // The octree automatically subdivides when voxels are placed at deeper levels
        let root = builder::build_ground_octree(&noise, &fbm, macro_depth);

        // Apply border layers if requested
        // Each border layer doubles the world size, so we need to account for this
        let root_with_borders = if border_depth > 0 {
            Self::add_border_layers(root, border_depth)
        } else {
            root
        };

        // Render depth must be deep enough to capture all possible voxel placements
        // macro_depth for terrain + micro_depth for user edits + border_depth for border expansion
        // Each border layer adds 1 to the effective depth (2^1 = doubles the size)
        let render_depth = macro_depth + micro_depth + border_depth;

        Self {
            cube: root_with_borders,
            macro_depth,
            render_depth,
            border_depth,
        }
    }

    /// Wrap a cube with border layers using 4 vertical levels
    ///
    /// Each layer divides depth into 4 levels (y=0,1,2,3):
    /// - Level 0 (bottom): hard_ground (16)
    /// - Level 1: water (17)
    /// - Level 2: air (0)
    /// - Level 3 (top): air (0)
    /// - Original world placed at depth level where it fits centered
    ///
    /// # Arguments
    /// * `world` - The original world cube to wrap
    /// * `layers` - Number of border layers to add (each layer doubles world size)
    ///
    /// # Example
    /// With 1 border layer, the world is subdivided vertically into 4 levels.
    /// At each XZ position, we have 4 vertical slices from bottom to top.
    fn add_border_layers(world: Cube<u8>, layers: u32) -> Cube<u8> {
        const HARD_GROUND: u8 = 16;
        const WATER: u8 = 17;
        const AIR: u8 = 0;

        // Border materials for each Y level [y0, y1, y2, y3]
        let border_materials = [HARD_GROUND, WATER, AIR, AIR];

        Cube::expand(&world, border_materials, layers as i32)
    }

    /// Set a voxel at octree coordinates
    ///
    /// # Coordinate System
    /// - Coordinates are in octree space at the specified depth
    /// - TypeScript handles world→octree conversion via worldToCube()
    /// - At depth d: valid coordinates are [0, 2^d - 1] per axis
    /// - Example with macroDepth=3:
    ///   * depth=0: coords [0,0] (entire world = one voxel)
    ///   * depth=1: coords [0,1] (2×2×2 voxels)
    ///   * depth=2: coords [0,3] (4×4×4 voxels)
    ///   * depth=3: coords [0,7] (8×8×8 voxels, 1 voxel = 1 world unit)
    ///
    /// The octree will automatically subdivide to support the requested depth
    pub fn set_voxel_at_depth(&mut self, x: i32, y: i32, z: i32, depth: u32, color_index: u8) {
        let pos = IVec3::new(x, y, z);
        self.cube = self
            .cube
            .update(CubeCoord::new(pos, depth), Cube::Solid(color_index))
            .simplified();
    }

    /// Remove a voxel at world coordinates at specified depth
    pub fn remove_voxel_at_depth(&mut self, x: i32, y: i32, z: i32, depth: u32) {
        self.set_voxel_at_depth(x, y, z, depth, 0);
    }

    /// Export the octree to CSM format
    pub fn export_to_csm(&self) -> String {
        serialize_csm(&self.cube)
    }

    /// Get a reference to the octree root (for testing)
    #[cfg(test)]
    pub fn get_root(&self) -> &Cube<u8> {
        &self.cube
    }

    pub fn generate_mesh(&self) -> GeometryData {
        // Generate mesh from octree using appropriate mesh builder
        let color_mapper = MaterialColorMapper::new();
        let mut builder = DefaultMeshBuilder::new();

        // Border materials for world at each Y layer [y0, y1, y2, y3]
        // y=0,1 (bottom): bedrock/stone (32)
        // y=2,3 (top): air (0)
        let border_materials = [32, 32, 0, 0];

        // Use render_depth for traversal to find all voxels (terrain + subdivisions)
        // The mesh generator will automatically calculate correct voxel sizes
        // for each depth level, ensuring subdivided voxels render at correct positions

        // Use face-based mesh generation with neighbor culling
        cube::generate_face_mesh(
            &self.cube,
            &mut builder,
            |index| color_mapper.map(index),
            border_materials,
            self.render_depth,
        );

        // Scale and offset vertices to match world coordinates
        // The mesh generator outputs vertices in [0,1] space where:
        // - Terrain voxels (at macro_depth) are correctly normalized
        // - Subdivided voxels (at macro+micro_depth) are correctly normalized
        // - Border layers expand the world by 2^border_depth
        // We scale by world_size (2^(macro_depth + border_depth)) to convert to world units
        let world_size = (1 << (self.macro_depth + self.border_depth)) as f32;
        let half_size = world_size / 2.0;

        let scaled_vertices: Vec<f32> = builder
            .vertices
            .chunks(3)
            .flat_map(|chunk| {
                let x = chunk[0] * world_size - half_size;
                let y = chunk[1] * world_size - half_size;
                let z = chunk[2] * world_size - half_size;
                vec![x, y, z]
            })
            .collect();

        GeometryData::new_with_uvs(
            scaled_vertices,
            builder.indices,
            builder.normals,
            builder.colors,
            builder.uvs,
            builder.material_ids,
        )
    }

    /// Get reference to the root cube
    #[allow(dead_code)]
    pub fn root(&self) -> &Cube<u8> {
        &self.cube
    }

    /// Set a new root cube
    ///
    /// Replaces the entire octree root. The cube will be simplified automatically.
    pub fn set_root(&mut self, cube: Cube<u8>) {
        self.cube = cube.simplified();
    }
}

impl Default for WorldCube {
    fn default() -> Self {
        Self::new(4, 0, 4, 0) // Default: macro depth 4, micro depth 0, border depth 4, seed 0
    }
}

/// Color mapper for cube ground that uses proper material colors
struct MaterialColorMapper;

impl MaterialColorMapper {
    fn new() -> Self {
        Self
    }
}

impl ColorMapper for MaterialColorMapper {
    fn map(&self, index: u8) -> [f32; 3] {
        let color = cube::material::get_material_color(index as i32);
        [color.x, color.y, color.z]
    }
}
