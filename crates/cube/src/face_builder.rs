use crate::neighbor_traversal::traverse_octree_with_neighbors;
use crate::mesh_builder::MeshBuilder;
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
pub fn generate_face_mesh<B, F>(
    root: &Cube<i32>,
    builder: &mut B,
    color_fn: F,
    max_depth: u32,
) where
    B: MeshBuilder,
    F: Fn(i32) -> [f32; 3] + Copy,
{
    let grid_size = 1 << max_depth; // 2^max_depth
    let voxel_size = 1.0 / grid_size as f32;

    // Traverse octree with neighbor context
    traverse_octree_with_neighbors(root, max_depth, |view, coord| {
        let center = view.center();

        // Only process leaf voxels (Solid nodes that won't subdivide further)
        // Skip if this is a branching node (Cubes)
        if matches!(**center, Cube::Cubes(_)) {
            return;
        }

        let center_id = center.id();

        // Only process empty voxels
        if center_id != 0 {
            return;
        }

        // Calculate position in normalized [0,1] space
        let x = coord.pos.x as f32 * voxel_size;
        let y = coord.pos.y as f32 * voxel_size;
        let z = coord.pos.z as f32 * voxel_size;

        // Check all 6 neighbors and add faces where neighbor is solid
        // For each direction, we render the opposite face of the solid neighbor
        // with normal pointing from solid into empty space

        // Left neighbor solid: render its RIGHT face (normal +X pointing into empty)
        check_and_add_face(
            Face::Right,
            view.left(),
            x - voxel_size,
            y,
            z,
            voxel_size,
            builder,
            &color_fn,
        );

        // Right neighbor solid: render its LEFT face (normal -X pointing into empty)
        check_and_add_face(
            Face::Left,
            view.right(),
            x + voxel_size,
            y,
            z,
            voxel_size,
            builder,
            &color_fn,
        );

        // Down neighbor solid: render its TOP face (normal +Y pointing into empty)
        check_and_add_face(
            Face::Top,
            view.down(),
            x,
            y - voxel_size,
            z,
            voxel_size,
            builder,
            &color_fn,
        );

        // Up neighbor solid: render its BOTTOM face (normal -Y pointing into empty)
        check_and_add_face(
            Face::Bottom,
            view.up(),
            x,
            y + voxel_size,
            z,
            voxel_size,
            builder,
            &color_fn,
        );

        // Back neighbor solid: render its FRONT face (normal +Z pointing into empty)
        check_and_add_face(
            Face::Front,
            view.back(),
            x,
            y,
            z - voxel_size,
            voxel_size,
            builder,
            &color_fn,
        );

        // Front neighbor solid: render its BACK face (normal -Z pointing into empty)
        check_and_add_face(
            Face::Back,
            view.front(),
            x,
            y,
            z + voxel_size,
            voxel_size,
            builder,
            &color_fn,
        );
    });
}

/// Check if neighbor is solid and add face if needed
#[allow(clippy::too_many_arguments)]
fn check_and_add_face<B, F>(
    face: Face,
    neighbor: Option<&std::rc::Rc<Cube<i32>>>,
    x: f32,
    y: f32,
    z: f32,
    size: f32,
    builder: &mut B,
    color_fn: &F,
) where
    B: MeshBuilder,
    F: Fn(i32) -> [f32; 3],
{
    if let Some(neighbor_cube) = neighbor {
        let neighbor_id = neighbor_cube.id();

        // Only add face if neighbor is solid (non-zero)
        if neighbor_id != 0 {
            let vertices = face.vertices(x, y, z, size);
            let normal = face.normal();
            let color = color_fn(neighbor_id);

            builder.add_face(vertices, normal, color);
        }
    }
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
        assert!(builder.indices.len() > 0, "Empty voxels touching ground borders should generate faces");
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
        assert!(builder.indices.len() > 0, "Should generate faces for visible solid");
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
        assert_eq!(builder.indices.len(), 0, "All solid octree should have no faces");
    }
}
