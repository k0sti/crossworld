pub mod cube_ground;

use crate::GeometryData;

pub struct GeometryEngine {
    cube_ground: cube_ground::CubeGround,
}

impl GeometryEngine {
    pub fn new() -> Self {
        Self {
            cube_ground: cube_ground::CubeGround::new(5), // Depth 5 (32^3 voxels, 64 world units with scale=1)
        }
    }

    pub fn generate_frame(&self) -> GeometryData {
        self.cube_ground.generate_mesh()
    }

    /// Set a voxel in the cube ground at specified depth
    /// depth: octree depth (7=finest detail, 4=coarse, etc.)
    pub fn set_voxel_at_depth(&mut self, x: i32, y: i32, z: i32, depth: u32, color_index: i32) {
        self.cube_ground.set_voxel_at_depth(x, y, z, depth, color_index);
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
}
