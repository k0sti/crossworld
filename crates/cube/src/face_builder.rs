use crate::mesh_builder::MeshBuilder;
use crate::neighbor_traversal::{
    traverse_with_neighbors, NeighborGrid, OFFSET_BACK, OFFSET_DOWN, OFFSET_FRONT, OFFSET_LEFT,
    OFFSET_RIGHT, OFFSET_UP,
};
use crate::octree::Cube;

/// Face direction for cube faces
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Face {
    Top,    // +Y
    Bottom, // -Y
    Left,   // -X
    Right,  // +X
    Front,  // +Z
    Back,   // -Z
}

impl Face {
    /// Get the normal vector for this face (pointing into the solid voxel)
    fn normal(self) -> [f32; 3] {
        match self {
            Face::Top => [0.0, 1.0, 0.0],
            Face::Bottom => [0.0, -1.0, 0.0],
            Face::Left => [-1.0, 0.0, 0.0],
            Face::Right => [1.0, 0.0, 0.0],
            Face::Front => [0.0, 0.0, 1.0],
            Face::Back => [0.0, 0.0, -1.0],
        }
    }

    /// Get the four vertices for this face in counter-clockwise order when viewed from outside
    fn vertices(self, x: f32, y: f32, z: f32, size: f32) -> [[f32; 3]; 4] {
        match self {
            Face::Top => [
                [x, y + size, z],
                [x, y + size, z + size],
                [x + size, y + size, z + size],
                [x + size, y + size, z],
            ],
            Face::Bottom => [
                [x, y, z],
                [x + size, y, z],
                [x + size, y, z + size],
                [x, y, z + size],
            ],
            Face::Left => [
                [x, y, z + size],
                [x, y + size, z + size],
                [x, y + size, z],
                [x, y, z],
            ],
            Face::Right => [
                [x + size, y, z],
                [x + size, y + size, z],
                [x + size, y + size, z + size],
                [x + size, y, z + size],
            ],
            Face::Front => [
                [x + size, y, z + size],
                [x + size, y + size, z + size],
                [x, y + size, z + size],
                [x, y, z + size],
            ],
            Face::Back => [
                [x, y, z],
                [x, y + size, z],
                [x + size, y + size, z],
                [x + size, y, z],
            ],
        }
    }
}

/// Generate mesh from octree using neighbor-aware face culling
///
/// This function only generates faces where empty voxels meet solid voxels.
/// Each face is rendered from the empty space looking into the solid voxel.
///
/// # Arguments
/// * `root` - The root cube of the octree
/// * `builder` - MeshBuilder to receive the faces
/// * `color_fn` - Function to map voxel values to colors
/// * `max_depth` - Maximum depth of the octree
pub fn generate_face_mesh<B, F>(root: &Cube<i32>, builder: &mut B, color_fn: F, max_depth: u32)
where
    B: MeshBuilder,
    F: Fn(i32) -> [f32; 3] + Copy,
{
    // Create initial neighbor grid and traverse
    let grid = NeighborGrid::new(root, 33, 0);
    let mut face_count = 0;
    traverse_with_neighbors(
        &grid,
        &mut |view, coord, _subleaf| {
            let center = view.center();
            let center_id = center.id();

            // Only process empty voxels
            if center_id != 0 {
                return false;
            }

            // Calculate voxel size based on actual depth of this voxel
            // coord.depth counts down from max_depth as we traverse
            // Coordinate space: positions are in [0, 2^(max_depth - coord.depth + 1))
            // because octants start in [0, 2) space and double each level down
            let voxel_size = 1.0 / (1 << (max_depth - coord.depth + 1)) as f32;

            // Calculate position in normalized [0,1] space
            let x = coord.pos.x as f32 * voxel_size;
            let y = coord.pos.y as f32 * voxel_size;
            let z = coord.pos.z as f32 * voxel_size;

            // Check all 6 neighbors for subdivision requirements and face generation
            // For each direction, we render the opposite face of the solid neighbor
            // with normal pointing from solid into empty space
            const DIRECTIONS: [(Face, i32, f32, f32, f32); 6] = [
                (Face::Right, OFFSET_LEFT, -1.0, 0.0, 0.0), // Left neighbor: render RIGHT face
                (Face::Left, OFFSET_RIGHT, 1.0, 0.0, 0.0),  // Right neighbor: render LEFT face
                (Face::Top, OFFSET_DOWN, 0.0, -1.0, 0.0),   // Down neighbor: render TOP face
                (Face::Bottom, OFFSET_UP, 0.0, 1.0, 0.0),   // Up neighbor: render BOTTOM face
                (Face::Front, OFFSET_BACK, 0.0, 0.0, -1.0), // Back neighbor: render FRONT face
                (Face::Back, OFFSET_FRONT, 0.0, 0.0, 1.0),  // Front neighbor: render BACK face
            ];

            let mut should_subdivide = false;

            for (face, dir_offset, dx, dy, dz) in DIRECTIONS {
                if let Some(neighbor_cube) = view.get(dir_offset) {
                    let neighbor_id = neighbor_cube.id();

                    // Skip empty neighbors
                    if neighbor_id == 0 {
                        continue;
                    }

                    // Check if neighbor is subdivided (branch node)
                    // If so, we need to subdivide this voxel to match the detail level
                    if matches!(**neighbor_cube, Cube::Cubes(_)) {
                        should_subdivide = true;
                        continue; // Don't generate face, need to subdivide first
                    }

                    // Neighbor is a leaf at the same depth - generate face
                    let nx = x + dx * voxel_size;
                    let ny = y + dy * voxel_size;
                    let nz = z + dz * voxel_size;
                    let vertices = face.vertices(nx, ny, nz, voxel_size);
                    let normal = face.normal();
                    let color = color_fn(neighbor_id);
                    builder.add_face(vertices, normal, color);
                    face_count += 1;
                }
            }

            // Subdivide if any neighbor is octa
            should_subdivide
        },
        max_depth,
    );

    tracing::debug!(
        "[generate_face_mesh] Generated {} faces at max_depth={}",
        face_count,
        max_depth
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mesh_builder::DefaultMeshBuilder;
    use crate::octree::Cube;
    use std::rc::Rc;

    fn simple_color_mapper(value: i32) -> [f32; 3] {
        if value <= 0 {
            [0.0, 0.0, 0.0]
        } else {
            [1.0, 0.0, 0.0] // Simple red color for testing
        }
    }

    #[test]
    fn test_single_solid_cube() {
        // A single solid cube should have all 6 faces rendered from surrounding empty space
        let root = Cube::Solid(1);
        let mut builder = DefaultMeshBuilder::new();

        generate_face_mesh(&root, &mut builder, simple_color_mapper, 0);

        // Should have no faces because there are no empty voxels in a solid cube
        // The traversal only processes the octants, and a solid cube has no subdivision
        assert_eq!(builder.indices.len(), 0);
    }

    #[test]
    fn test_checkerboard_pattern() {
        // Create octree with checkerboard solid/empty pattern
        let root = Cube::cubes([
            Rc::new(Cube::Solid(0)), // Empty
            Rc::new(Cube::Solid(1)), // Solid
            Rc::new(Cube::Solid(0)), // Empty
            Rc::new(Cube::Solid(1)), // Solid
            Rc::new(Cube::Solid(0)), // Empty
            Rc::new(Cube::Solid(1)), // Solid
            Rc::new(Cube::Solid(0)), // Empty
            Rc::new(Cube::Solid(1)), // Solid
        ]);

        let mut builder = DefaultMeshBuilder::new();
        generate_face_mesh(&root, &mut builder, simple_color_mapper, 1);

        // Empty voxels should have faces where they touch solid voxels
        // 4 empty voxels, each can have multiple solid neighbors
        assert!(builder.indices.len() > 0, "Should generate some faces");
    }

    #[test]
    fn test_all_empty_with_borders() {
        // All empty octree will generate faces where empty voxels meet border ground
        let root = Cube::cubes([
            Rc::new(Cube::Solid(0)), // All empty
            Rc::new(Cube::Solid(0)),
            Rc::new(Cube::Solid(0)),
            Rc::new(Cube::Solid(0)),
            Rc::new(Cube::Solid(0)),
            Rc::new(Cube::Solid(0)),
            Rc::new(Cube::Solid(0)),
            Rc::new(Cube::Solid(0)),
        ]);

        let mut builder = DefaultMeshBuilder::new();
        generate_face_mesh(&root, &mut builder, simple_color_mapper, 1);

        // Empty voxels at the border will see ground (33) below them and generate upward faces
        // This is expected behavior for terrain rendering
        assert!(
            builder.indices.len() > 0,
            "Empty voxels touching ground borders should generate faces"
        );
    }

    #[test]
    fn test_single_solid_in_empty() {
        // One solid voxel surrounded by empty
        let root = Cube::cubes([
            Rc::new(Cube::Solid(0)), // Empty
            Rc::new(Cube::Solid(0)), // Empty
            Rc::new(Cube::Solid(0)), // Empty
            Rc::new(Cube::Solid(1)), // Solid - octant 3 (0,1,1)
            Rc::new(Cube::Solid(0)), // Empty
            Rc::new(Cube::Solid(0)), // Empty
            Rc::new(Cube::Solid(0)), // Empty
            Rc::new(Cube::Solid(0)), // Empty
        ]);

        let mut builder = DefaultMeshBuilder::new();
        generate_face_mesh(&root, &mut builder, simple_color_mapper, 1);

        // Empty neighbors of the solid voxel should render faces towards it
        // Expect at least some faces
        assert!(
            builder.indices.len() > 0,
            "Should generate faces for visible solid"
        );
    }

    #[test]
    fn test_all_solid() {
        // All solid octree should generate no faces (no empty voxels to render from)
        let root = Cube::cubes([
            Rc::new(Cube::Solid(1)), // All solid
            Rc::new(Cube::Solid(1)),
            Rc::new(Cube::Solid(1)),
            Rc::new(Cube::Solid(1)),
            Rc::new(Cube::Solid(1)),
            Rc::new(Cube::Solid(1)),
            Rc::new(Cube::Solid(1)),
            Rc::new(Cube::Solid(1)),
        ]);

        let mut builder = DefaultMeshBuilder::new();
        generate_face_mesh(&root, &mut builder, simple_color_mapper, 1);

        // No empty voxels means no faces are rendered (we only render from empty voxels)
        assert_eq!(
            builder.indices.len(),
            0,
            "All solid octree should have no faces"
        );
    }
}
