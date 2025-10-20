use crate::GeometryData;
use crossworld_cube::{Octree, OctreeNode, Octant};

pub struct CubeGround {
    octree: Octree,
}

impl CubeGround {
    pub fn new() -> Self {
        // Build an 8x8x8 cube with 3 levels deep
        // Bottom half (y=0-3): colored ground
        // Top half (y=4-7): empty (0)

        let root = Self::build_ground_octree();

        Self {
            octree: Octree::new(root),
        }
    }

    fn build_ground_octree() -> OctreeNode {
        // Build 8 children at level 1 (each represents 4x4x4 space)
        let level1_children = Octant::all().map(|octant| {
            // Check if this octant is in the bottom half (y- means y bit is 0)
            let (_, y, _) = octant.offset();
            if y == 0.0 {
                // Bottom half: build ground
                Self::build_ground_quadrant(octant)
            } else {
                // Top half: empty
                OctreeNode::Value(0)
            }
        });

        OctreeNode::new_children(level1_children)
    }

    fn build_ground_quadrant(parent_octant: Octant) -> OctreeNode {
        // Build 8 children at level 2 (each represents 2x2x2 space)
        let level2_children = Octant::all().map(|octant| {
            // Only bottom half has ground
            let (_, y, _) = octant.offset();
            if y == 0.0 {
                Self::build_ground_cell(parent_octant, octant)
            } else {
                OctreeNode::Value(0)
            }
        });

        OctreeNode::new_children(level2_children)
    }

    fn build_ground_cell(parent_octant: Octant, octant: Octant) -> OctreeNode {
        // Build 8 children at level 3 (each represents 1x1x1 voxel)
        // Calculate x, z position based on octants
        let (px, _, pz) = parent_octant.offset();
        let (x, _, z) = octant.offset();

        // Calculate grid position (0-7 range, scaled from 0-0.5 offsets)
        let grid_x = ((px * 8.0) + (x * 4.0)) as i32;
        let grid_z = ((pz * 8.0) + (z * 4.0)) as i32;

        let level3_children = Octant::all().map(|sub_octant| {
            let (sx, sy, sz) = sub_octant.offset();
            let final_x = grid_x + (sx * 2.0) as i32;
            let final_z = grid_z + (sz * 2.0) as i32;

            // Only the bottom layer (y=0) has voxels
            let value = if sy == 0.0 {
                // Checkerboard pattern with color values
                let is_light = (final_x + final_z) % 2 == 0;
                if is_light { 1 } else { 2 }
            } else {
                0
            };

            OctreeNode::Value(value)
        });

        OctreeNode::new_children(level3_children)
    }

    pub fn generate_mesh(&self) -> GeometryData {
        // Generate mesh from octree using the cube mesher
        let mesh_data = crossworld_cube::generate_mesh(&self.octree);

        GeometryData::new(
            mesh_data.vertices,
            mesh_data.indices,
            mesh_data.normals,
            mesh_data.colors,
        )
    }
}

impl Default for CubeGround {
    fn default() -> Self {
        Self::new()
    }
}
