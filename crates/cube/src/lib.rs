// Cube crate - Voxel octree data structure and operations

// New module structure
pub mod axis;
pub mod core;
pub mod fabric;
pub mod function;
pub mod io;
pub mod material;
pub mod mesh;
pub mod render;
pub mod traversal;

#[cfg(feature = "wasm")]
pub mod wasm;

// Re-export main types from core
pub use core::{
    octant_char_to_index, octant_index_to_char, raycast, raycast_with_options, Cube, CubeBox, Hit,
    IVec3Ext, RaycastDebugState, RaycastOptions, Voxel, OCTANT_POSITIONS,
};

// Re-export axis types
pub use axis::Axis;

// Re-export traversal types
pub use traversal::{
    traverse_octree, traverse_region, visit_faces, visit_faces_at_coord, visit_faces_in_region,
    visit_voxels_in_region, CubeCoord, FaceInfo, NeighborGrid, NeighborView, RegionBounds,
    TraversalVisitor, VoxelInfo, OFFSET_BACK, OFFSET_DOWN, OFFSET_FRONT, OFFSET_LEFT, OFFSET_RIGHT,
    OFFSET_UP,
};

// Re-export mesh types
pub use mesh::{
    generate_face_mesh, ColorMapper, DefaultMeshBuilder, Face, HsvColorMapper, MeshBuilder,
    PaletteColorMapper, VoxColorMapper,
};

// Re-export IO types
#[allow(deprecated)]
pub use io::load_vox_to_cube;
pub use io::{
    load_vox_to_cubebox, load_vox_to_cubebox_compact, parse_csm, serialize_csm, CsmError,
};

// Re-export fabric types
pub use fabric::{AdditiveState, FabricConfig, FabricGenerator};

// Re-export function types
pub use function::{
    compile as compile_function, parse_expr, BinOpKind, BuiltinFunc, CachedCube, CompileError,
    CpuFunction, DynamicCube, EvalContext, Expr, MatchPattern, ParseError, UnaryOpKind, VarId,
};

// Re-export render types
pub use render::{
    render_orthographic, render_orthographic_2d, render_orthographic_3d, RenderedImage,
    ViewDirection,
};

// Re-export glam for convenience
pub use glam;
