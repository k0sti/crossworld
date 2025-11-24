// Cube crate - Voxel octree data structure and operations

// New module structure
pub mod axis;
pub mod core;
pub mod io;
pub mod material;
pub mod mesh;
pub mod raycast;
pub mod render;
pub mod traversal;

#[cfg(feature = "wasm")]
pub mod wasm;

// Re-export main types from core
pub use core::{
    octant_char_to_index, octant_index_to_char, Cube, IVec3Ext, Octree, Quad, OCTANT_POSITIONS,
};

// Re-export axis types
pub use axis::Axis;

// Re-export traversal types
pub use traversal::{
    traverse_octree, visit_faces, CubeCoord, FaceInfo, NeighborGrid, NeighborView,
    TraversalVisitor, OFFSET_BACK, OFFSET_DOWN, OFFSET_FRONT, OFFSET_LEFT, OFFSET_RIGHT, OFFSET_UP,
};

// Re-export mesh types
pub use mesh::{
    generate_face_mesh, ColorMapper, DefaultMeshBuilder, Face, HsvColorMapper, MeshBuilder,
    PaletteColorMapper, VoxColorMapper,
};

// Re-export raycast types
pub use raycast::{RaycastDebugState, RaycastError, RaycastHit};

// Re-export IO types
pub use io::{load_vox_to_cube, parse_csm, serialize_csm, CsmError};

// Re-export render types
pub use render::{
    render_orthographic, render_orthographic_2d, render_orthographic_3d, RenderedImage,
    ViewDirection,
};

// Re-export glam for convenience
pub use glam;
