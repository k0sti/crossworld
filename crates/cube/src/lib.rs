// Cube crate - Voxel octree data structure and operations

// New module structure
pub mod core;
pub mod traversal;
pub mod mesh;
pub mod raycast;
pub mod io;
pub mod render;

#[cfg(feature = "wasm")]
pub mod wasm;

// Re-export main types from core
pub use core::{
    octant_char_to_index, octant_index_to_char, Axis, Cube, IVec3Ext, Octree, Quad,
    OCTANT_POSITIONS,
};

// Re-export traversal types
pub use traversal::{
    traverse_octree, CubeCoord, NeighborGrid, NeighborView,
    TraversalVisitor, OFFSET_BACK, OFFSET_DOWN, OFFSET_FRONT, OFFSET_LEFT, OFFSET_RIGHT, OFFSET_UP,
};

// Re-export mesh types
pub use mesh::{
    generate_face_mesh, visit_faces, ColorMapper, DefaultMeshBuilder, Face, FaceInfo,
    HsvColorMapper, MeshBuilder, PaletteColorMapper, VoxColorMapper,
};

// Re-export raycast types
pub use raycast::RaycastHit;

// Re-export IO types
pub use io::{parse_csm, serialize_csm, load_vox_to_cube, CsmError};

// Re-export render types
pub use render::{
    render_orthographic, render_orthographic_2d, render_orthographic_3d, RenderedImage,
    ViewDirection,
};

// Re-export glam for convenience
pub use glam;
