mod mesh;
mod octree;
mod parser;
mod render;

// Only compile WASM bindings when "wasm" feature is enabled
#[cfg(feature = "wasm")]
mod wasm;

pub use mesh::{
    generate_mesh, generate_mesh_with_mapper, ColorMapper, HsvColorMapper, MeshData,
    PaletteColorMapper,
};
pub use octree::{
    octant_char_to_index, octant_index_to_char, Axis, Cube, Octree, Quad,
};
pub use parser::{parse_csm, CsmError};
pub use render::{
    render_orthographic, render_orthographic_2d, render_orthographic_3d, RenderedImage,
    ViewDirection,
};
