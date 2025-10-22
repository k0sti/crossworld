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
            cube_ground: cube_ground::CubeGround::new(),
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
        match self.render_mode {
            GroundRenderMode::Flat => self.ground.generate_mesh(),
            GroundRenderMode::Cube => self.cube_ground.generate_mesh(),
        }
    }
}
