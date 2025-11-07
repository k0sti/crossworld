// Neighbor-aware octree traversal

pub mod neighbor_grid;

// Re-export main types and functions
pub use neighbor_grid::{
    traverse_octree, CubeCoord, NeighborGrid, NeighborView,
    TraversalVisitor, OFFSET_BACK, OFFSET_DOWN, OFFSET_FRONT, OFFSET_LEFT, OFFSET_RIGHT, OFFSET_UP,
};
