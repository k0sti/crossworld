mod face_builder;
mod mesh;
mod mesh_builder;
mod neighbor_traversal;
mod octree;
mod parser;
mod render;

// Only compile WASM bindings when "wasm" feature is enabled
#[cfg(feature = "wasm")]
mod wasm;

pub use face_builder::generate_face_mesh;
pub use mesh::{ColorMapper, HsvColorMapper, PaletteColorMapper};
pub use mesh_builder::{generate_mesh_hierarchical, DefaultMeshBuilder, MeshBuilder};
pub use neighbor_traversal::{
    traverse_octree_with_neighbors, traverse_with_neighbors, CubeCoord, NeighborGrid, NeighborView,
    TraversalVisitor,
};
pub use octree::{octant_char_to_index, octant_index_to_char, Axis, Cube, IVec3Ext, Octree, Quad};
pub use parser::{parse_csm, CsmError};
pub use render::{
    render_orthographic, render_orthographic_2d, render_orthographic_3d, RenderedImage,
    ViewDirection,
};

// Re-export glam for convenience
pub use glam;
