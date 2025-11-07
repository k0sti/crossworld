// Mesh generation and color mapping

pub mod color_mapper;
pub mod face;
pub mod generator;

// Re-export main types
pub use color_mapper::{ColorMapper, HsvColorMapper, PaletteColorMapper, VoxColorMapper};
pub use face::Face;
pub use generator::{generate_face_mesh, visit_faces, DefaultMeshBuilder, FaceInfo, MeshBuilder};
