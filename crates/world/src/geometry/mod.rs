pub mod ground;

use crate::GeometryData;

pub struct GeometryEngine {
    ground: ground::Ground,
}

impl GeometryEngine {
    pub fn new() -> Self {
        Self {
            ground: ground::Ground::new(8, 8),
        }
    }

    pub fn generate_frame(&self) -> GeometryData {
        self.ground.generate_mesh()
    }
}
