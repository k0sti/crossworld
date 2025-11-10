use cube::{
    traverse_octree, Cube, CubeCoord, Face, NeighborGrid, NeighborView, OFFSET_BACK, OFFSET_DOWN,
    OFFSET_FRONT, OFFSET_LEFT, OFFSET_RIGHT, OFFSET_UP,
};
use glam::{Quat, Vec3};
use rapier3d::prelude::*;
use std::rc::Rc;

/// Builder for generating collision geometry from voxel cubes
///
/// Uses the cube crate's traverse_with_neighbors to iterate through voxel faces
/// and generates rectangle colliders for exposed faces.
pub struct VoxelColliderBuilder {
    rectangles: Vec<FaceRectangle>,
}

/// Represents a face rectangle for collision generation
#[derive(Debug, Clone)]
struct FaceRectangle {
    center: Vec3,
    normal: Vec3,
    size: f32,
}

impl VoxelColliderBuilder {
    /// Create a new collider builder
    pub fn new() -> Self {
        Self {
            rectangles: Vec::new(),
        }
    }

    /// Generate a compound collider from a voxel cube
    ///
    /// # Arguments
    /// * `cube` - The octree cube to generate collision from
    /// * `max_depth` - Maximum depth to traverse (higher = more detailed collision)
    ///
    /// # Returns
    /// Rapier Collider containing compound shape of all exposed faces
    pub fn from_cube(cube: &Rc<Cube<i32>>, max_depth: u32) -> Collider {
        let mut builder = Self::new();

        // Create neighbor grid with appropriate border materials
        // For world terrain: solid bottom (1), empty top (0)
        let border_materials = [1, 1, 0, 0];
        let grid = NeighborGrid::new(cube, border_materials);

        // Traverse all voxels and collect face rectangles
        traverse_octree(
            &grid,
            &mut |view, coord, _subleaf| {
                builder.process_voxel(view, coord);
                false // Don't subdivide further
            },
            max_depth,
        );

        builder.build_compound_collider()
    }

    /// Process a single voxel and add face rectangles for exposed faces
    fn process_voxel(&mut self, view: NeighborView, coord: CubeCoord) {
        let center = view.center();

        // Skip empty voxels (value <= 0 typically means empty)
        if center.id() <= 0 {
            return;
        }

        // Check each of 6 faces
        let faces = [
            (OFFSET_LEFT, Face::Left),
            (OFFSET_RIGHT, Face::Right),
            (OFFSET_DOWN, Face::Bottom),
            (OFFSET_UP, Face::Top),
            (OFFSET_BACK, Face::Back),
            (OFFSET_FRONT, Face::Front),
        ];

        for (offset, face) in faces {
            if let Some(neighbor) = view.get(offset) {
                // Face is exposed if neighbor is empty or different material
                if neighbor.id() <= 0 || neighbor.id() != center.id() {
                    self.add_face_rectangle(coord, face);
                }
            }
        }
    }

    /// Add a face rectangle for collision generation
    fn add_face_rectangle(&mut self, coord: CubeCoord, face: Face) {
        // Calculate voxel size based on depth
        let voxel_size = 1.0 / (1 << coord.depth) as f32;

        // Calculate world position from octree coordinate
        let world_pos = coord.pos.as_vec3() * voxel_size;

        // Get face normal
        let normal_array = face.normal();
        let normal = Vec3::from(normal_array);

        // Calculate face center position
        // Face is offset by half voxel size in the normal direction
        let voxel_center = world_pos + Vec3::splat(voxel_size * 0.5);
        let face_offset = normal * voxel_size * 0.5;
        let face_center = voxel_center + face_offset;

        self.rectangles.push(FaceRectangle {
            center: face_center,
            normal,
            size: voxel_size,
        });
    }

    /// Build a compound collider from all collected face rectangles
    fn build_compound_collider(self) -> Collider {
        if self.rectangles.is_empty() {
            // Empty collider - just use a tiny sphere
            return ColliderBuilder::ball(0.001).build();
        }

        // Create thin cuboid colliders for each face
        let thickness = 0.05; // Thin collider for faces
        let shapes: Vec<_> = self
            .rectangles
            .iter()
            .map(|rect| {
                let half_size = rect.size / 2.0;

                // Create cuboid shape
                // The cuboid is oriented with Z as the normal direction initially
                let shape = SharedShape::cuboid(half_size, half_size, thickness);

                // Calculate rotation to align Z-axis with face normal
                let rotation = Self::rotation_from_normal(rect.normal);

                // Create isometry (position + rotation)
                let pos = rect.center;
                let isometry = Isometry::new(
                    vector![pos.x, pos.y, pos.z],
                    vector![rotation.x, rotation.y, rotation.z],
                );

                (isometry, shape)
            })
            .collect();

        ColliderBuilder::compound(shapes).build()
    }

    /// Calculate rotation axis-angle from a normal vector
    ///
    /// Rotates from Z-axis to align with the given normal
    fn rotation_from_normal(normal: Vec3) -> Vec3 {
        let z_axis = Vec3::Z;

        // If normal is already aligned with Z, no rotation needed
        if (normal - z_axis).length() < 0.001 {
            return Vec3::ZERO;
        }

        // If normal is opposite to Z, rotate 180 degrees around X
        if (normal + z_axis).length() < 0.001 {
            return Vec3::new(std::f32::consts::PI, 0.0, 0.0);
        }

        // Calculate rotation quaternion
        let quat = Quat::from_rotation_arc(z_axis, normal);

        // Convert to axis-angle representation for Rapier
        let (axis, angle) = quat.to_axis_angle();
        axis * angle
    }

    /// Get the number of face rectangles collected
    pub fn face_count(&self) -> usize {
        self.rectangles.len()
    }
}

impl Default for VoxelColliderBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Helper function to create a simple box collider
///
/// # Arguments
/// * `half_extents` - Half the size in each dimension
pub fn create_box_collider(half_extents: Vec3) -> Collider {
    ColliderBuilder::cuboid(half_extents.x, half_extents.y, half_extents.z).build()
}

/// Helper function to create a sphere collider
///
/// # Arguments
/// * `radius` - Sphere radius
pub fn create_sphere_collider(radius: f32) -> Collider {
    ColliderBuilder::ball(radius).build()
}

/// Helper function to create a capsule collider
///
/// # Arguments
/// * `half_height` - Half the height of the cylindrical part
/// * `radius` - Radius of the capsule
pub fn create_capsule_collider(half_height: f32, radius: f32) -> Collider {
    ColliderBuilder::capsule_y(half_height, radius).build()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_collider_from_solid_cube() {
        let cube = Rc::new(Cube::Solid(1));
        let collider = VoxelColliderBuilder::from_cube(&cube, 3);

        // A solid cube should generate colliders
        assert!(collider.shape().as_compound().is_some());
    }

    #[test]
    fn test_collider_from_empty_cube() {
        let cube = Rc::new(Cube::Solid(0)); // Empty
        let collider = VoxelColliderBuilder::from_cube(&cube, 3);

        // Empty cube should generate minimal collider
        assert!(collider.shape().as_ball().is_some() || collider.shape().as_compound().is_some());
    }

    #[test]
    fn test_rotation_from_normal() {
        // Test Z-axis (no rotation)
        let rot = VoxelColliderBuilder::rotation_from_normal(Vec3::Z);
        assert!(rot.length() < 0.01);

        // Test -Z-axis (180 degree rotation)
        let rot = VoxelColliderBuilder::rotation_from_normal(-Vec3::Z);
        assert!((rot.length() - std::f32::consts::PI).abs() < 0.01);

        // Test other axes
        let _rot_x = VoxelColliderBuilder::rotation_from_normal(Vec3::X);
        let _rot_y = VoxelColliderBuilder::rotation_from_normal(Vec3::Y);
        // Just verify they don't panic
    }

    #[test]
    fn test_simple_colliders() {
        let box_collider = create_box_collider(Vec3::ONE);
        assert!(box_collider.shape().as_cuboid().is_some());

        let sphere_collider = create_sphere_collider(1.0);
        assert!(sphere_collider.shape().as_ball().is_some());

        let capsule_collider = create_capsule_collider(1.0, 0.5);
        assert!(capsule_collider.shape().as_capsule().is_some());
    }
}
