pub mod cube_ground;
pub mod ground;

use crate::GeometryData;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GroundRenderMode {
    Flat,
    Cube,
}

pub struct GeometryEngine {
    ground: ground::Ground,
    cube_ground: cube_ground::CubeGround,
    render_mode: GroundRenderMode,
}

impl GeometryEngine {
    pub fn new() -> Self {
        Self {
            ground: ground::Ground::new(8, 8),
            cube_ground: cube_ground::CubeGround::new(4), // Depth 4 = 16x16x16 grid
            render_mode: GroundRenderMode::Flat,
        }
    }

    pub fn set_render_mode(&mut self, mode: GroundRenderMode) {
        self.render_mode = mode;
    }

    pub fn get_render_mode(&self) -> GroundRenderMode {
        self.render_mode
    }

    pub fn generate_frame(&self) -> GeometryData {
        // Only render cube ground (flat ground removed as per user request)
        self.cube_ground.generate_mesh()
    }

    /// Set a voxel in the cube ground at specified depth
    /// depth: octree depth (4=single voxel, 3=2x2x2, 2=4x4x4, etc.)
    pub fn set_voxel_at_depth(&mut self, x: i32, y: i32, z: i32, depth: u32, color_index: i32) {
        self.cube_ground.set_voxel_at_depth(x, y, z, depth, color_index);
    }

    /// Set a single voxel in the cube ground
    pub fn set_voxel(&mut self, x: i32, y: i32, z: i32, color_index: i32) {
        self.cube_ground.set_voxel(x, y, z, color_index);
    }

    /// Remove a voxel from the cube ground
    pub fn remove_voxel(&mut self, x: i32, y: i32, z: i32) {
        self.cube_ground.remove_voxel(x, y, z);
    }
}
