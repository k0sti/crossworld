// Core octree data structures

pub mod cube;
pub mod raycast;

// Re-export main types
pub use cube::{
    octant_char_to_index, octant_index_to_char, Cube, IVec3Ext, Octree, Quad, OCTANT_POSITIONS,
};
// Export raycast types with new_ prefix to avoid conflicts with existing raycast module
pub use raycast::{
    raycast as raycast_new, Hit as HitNew, RaycastDebugState as RaycastDebugStateNew,
};
