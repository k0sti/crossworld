mod mesh;
mod octree;
mod parser;

// Only compile WASM bindings when "wasm" feature is enabled
#[cfg(feature = "wasm")]
mod wasm;

pub use mesh::generate_mesh;
pub use octree::{
    octant_char_to_index, octant_index_to_char, Axis, Cube, Octree, Quad,
};
pub use parser::{parse_csm, CsmError};
