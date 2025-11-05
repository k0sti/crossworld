mod face;
mod face_builder;
mod mesh;
mod neighbor_traversal;
mod octree;
mod parser;
mod raycast_aether;
mod render;
mod serializer;
mod vox_loader;

// Only compile WASM bindings when "wasm" feature is enabled
#[cfg(feature = "wasm")]
mod cube_wasm; // Unified Cube-centric interface

pub use face::Face;
pub use face_builder::{generate_face_mesh, DefaultMeshBuilder, MeshBuilder};
pub use mesh::{ColorMapper, HsvColorMapper, PaletteColorMapper, VoxColorMapper};
pub use neighbor_traversal::{
    traverse_with_neighbors, CubeCoord, NeighborGrid, NeighborView, TraversalVisitor, OFFSET_BACK,
    OFFSET_DOWN, OFFSET_FRONT, OFFSET_LEFT, OFFSET_RIGHT, OFFSET_UP,
};
pub use octree::{
    octant_char_to_index, octant_index_to_char, Axis, Cube, IVec3Ext, Octree, Quad,
    OCTANT_POSITIONS,
};
pub use parser::{parse_csm, CsmError};
pub use render::{
    render_orthographic, render_orthographic_2d, render_orthographic_3d, RenderedImage,
    ViewDirection,
};
pub use serializer::serialize_csm;
pub use vox_loader::load_vox_to_cube;

// Re-export glam for convenience
pub use glam;
