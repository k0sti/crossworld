pub mod cube_ground;

use crate::GeometryData;

pub struct GeometryEngine {
    cube_ground: cube_ground::CubeGround,
}

impl GeometryEngine {
    /// Create new GeometryEngine with specified depths
    ///
    /// # Arguments
    /// * `macro_depth` - World size depth (e.g., 3 = 8×8×8 world units)
    /// * `micro_depth` - Subdivision depth (0-3), used for mesh generation
    ///
    /// World size is determined by macro depth (2^macro_depth).
    /// Total depth (macro + micro) is used for mesh generation to get correct voxel sizes.
    pub fn new(macro_depth: u32, micro_depth: u32) -> Self {
        Self {
            cube_ground: cube_ground::CubeGround::new(macro_depth, micro_depth),
        }
    }

    pub fn generate_frame(&self) -> GeometryData {
        self.cube_ground.generate_mesh()
    }

    /// Set a voxel in the cube ground at specified depth
    /// depth: octree depth (7=finest detail, 4=coarse, etc.)
    pub fn set_voxel_at_depth(&mut self, x: i32, y: i32, z: i32, depth: u32, color_index: i32) {
        self.cube_ground
            .set_voxel_at_depth(x, y, z, depth, color_index);
    }

    /// Set a single voxel in the cube ground
    pub fn set_voxel(&mut self, x: i32, y: i32, z: i32, color_index: i32) {
        self.cube_ground.set_voxel(x, y, z, color_index);
    }

    /// Remove a voxel from the cube ground at specified depth
    pub fn remove_voxel_at_depth(&mut self, x: i32, y: i32, z: i32, depth: u32) {
        self.cube_ground.remove_voxel_at_depth(x, y, z, depth);
    }

    /// Remove a voxel from the cube ground
    pub fn remove_voxel(&mut self, x: i32, y: i32, z: i32) {
        self.cube_ground.remove_voxel(x, y, z);
    }

    /// Set face mesh mode (neighbor-aware culling)
    pub fn set_face_mesh_mode(&mut self, enabled: bool) {
        self.cube_ground.set_face_mesh_mode(enabled);
    }

    /// Set ground render mode (cube vs plane)
    pub fn set_ground_render_mode(&mut self, use_cube: bool) {
        self.cube_ground.set_ground_render_mode(use_cube);
    }

    /// Export the current world state to CSM format
    pub fn export_to_csm(&self) -> String {
        self.cube_ground.export_to_csm()
    }
}
