mod mesh;
mod mesh_builder;
mod octree;
mod parser;
mod render;

// Only compile WASM bindings when "wasm" feature is enabled
#[cfg(feature = "wasm")]
mod wasm;

pub use mesh::{ColorMapper, HsvColorMapper, PaletteColorMapper};
pub use mesh_builder::{generate_mesh_hierarchical, DefaultMeshBuilder, MeshBuilder};
pub use octree::{octant_char_to_index, octant_index_to_char, Axis, Cube, IVec3Ext, Octree, Quad};
pub use parser::{parse_csm, CsmError};
pub use render::{
    render_orthographic, render_orthographic_2d, render_orthographic_3d, RenderedImage,
    ViewDirection,
};

// Re-export glam for convenience
pub use glam;
