//! Collision geometry generation from voxel cubes
//!
//! This module provides utilities for generating Rapier3D colliders from
//! voxel octree structures. It uses the cube crate's face traversal to
//! identify exposed faces and creates triangle mesh colliders from them.
//!
//! Triangle mesh colliders provide accurate, infinitely thin collision surfaces
//! with no thickness artifacts. Best for world terrain and static geometry.

use crate::collision::Aabb;
use cube::{visit_faces, visit_faces_in_region, Cube, CubeBox, FaceInfo, RegionBounds};
use glam::Vec3;
use rapier3d::prelude::*;
use std::rc::Rc;

/// Builder for generating triangle mesh collision geometry from voxel cubes
///
/// Uses the cube crate's face traversal to iterate through exposed voxel faces
/// and generates triangle mesh colliders.
pub struct VoxelColliderBuilder {
    rectangles: Vec<FaceRectangle>,
}

/// Represents a face rectangle for collision generation
#[derive(Debug, Clone)]
struct FaceRectangle {
    center: Vec3,
    normal: Vec3,
    /// Face size - either uniform (for unit cubes) or per-axis (for CubeBox)
    /// For uniform: all components equal the voxel size
    /// For non-uniform: components scaled by CubeBox dimensions
    size: Vec3,
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
    /// Processes all exposed faces and creates thin cuboid colliders for each.
    /// The collider is generated in [0,1] normalized space.
    ///
    /// # Arguments
    /// * `cube` - The octree cube to generate collision from
    /// * `_max_depth` - Deprecated parameter (kept for API compatibility)
    ///
    /// # Returns
    /// Rapier Collider containing compound shape of all exposed faces
    pub fn from_cube(cube: &Rc<Cube<u8>>, _max_depth: u32) -> Collider {
        Self::from_cube_with_region(cube, None)
    }

    /// Generate a compound collider from a voxel cube with world-space scaling
    ///
    /// Processes all exposed faces and creates thin cuboid colliders for each,
    /// scaled and positioned to world coordinates.
    ///
    /// # Arguments
    /// * `cube` - The octree cube to generate collision from
    /// * `_max_depth` - Deprecated parameter (kept for API compatibility)
    /// * `world_size` - The world size (collider will span [-world_size/2, world_size/2])
    ///
    /// # Returns
    /// Rapier Collider containing compound shape of all exposed faces in world space
    pub fn from_cube_scaled(cube: &Rc<Cube<u8>>, _max_depth: u32, world_size: f32) -> Collider {
        Self::from_cube_with_region_scaled(cube, None, world_size)
    }

    /// Generate a compound collider from a voxel cube with optional spatial filtering
    ///
    /// This is an optimized version that allows filtering voxels to a specific region.
    /// Only voxels within the region bounds will be processed, significantly reducing
    /// collision complexity when only a subset of the voxel object participates
    /// in collision (e.g., when AABBs barely overlap).
    ///
    /// # Arguments
    /// * `cube` - The octree cube to generate collision from
    /// * `region` - Optional region bounds to filter voxels. If None, processes all voxels.
    ///
    /// # Returns
    /// Rapier Collider containing compound shape of exposed faces in the region
    ///
    /// # Performance
    /// - Full collision (region = None): O(n) faces for n voxels
    /// - Filtered collision: O(k) faces for k voxels in overlap region
    /// - Typical reduction: 70-90% fewer faces for small overlap regions
    pub fn from_cube_with_region(cube: &Rc<Cube<u8>>, region: Option<&RegionBounds>) -> Collider {
        let mut builder = Self::new();

        // Border materials: solid at bottom, empty at top
        let border_materials = [1, 1, 0, 0];

        match region {
            Some(bounds) => {
                // Use region-bounded traversal for efficiency
                visit_faces_in_region(
                    cube,
                    bounds,
                    |face_info| {
                        builder.add_face_from_info(face_info);
                    },
                    border_materials,
                );
            }
            None => {
                // Full traversal
                visit_faces(
                    cube,
                    |face_info| {
                        builder.add_face_from_info(face_info);
                    },
                    border_materials,
                );
            }
        }

        builder.build_compound_collider()
    }

    /// Generate a compound collider from a voxel cube with optional spatial filtering and world-space scaling
    ///
    /// # Arguments
    /// * `cube` - The octree cube to generate collision from
    /// * `region` - Optional region bounds to filter voxels. If None, processes all voxels.
    /// * `world_size` - The world size (collider will span [-world_size/2, world_size/2])
    ///
    /// # Returns
    /// Rapier Collider containing compound shape of exposed faces in world space
    pub fn from_cube_with_region_scaled(
        cube: &Rc<Cube<u8>>,
        region: Option<&RegionBounds>,
        world_size: f32,
    ) -> Collider {
        let mut builder = Self::new();

        // Border materials: solid at bottom, empty at top
        let border_materials = [1, 1, 0, 0];

        match region {
            Some(bounds) => {
                // Use region-bounded traversal for efficiency
                visit_faces_in_region(
                    cube,
                    bounds,
                    |face_info| {
                        builder.add_face_from_info(face_info);
                    },
                    border_materials,
                );
            }
            None => {
                // Full traversal
                visit_faces(
                    cube,
                    |face_info| {
                        builder.add_face_from_info(face_info);
                    },
                    border_materials,
                );
            }
        }

        builder.build_compound_collider_scaled(world_size)
    }

    /// Generate a compound collider with an AABB filter (convenience wrapper)
    ///
    /// # Arguments
    /// * `cube` - The octree cube to generate collision from
    /// * `_max_depth` - Deprecated parameter (kept for API compatibility)
    /// * `region` - Optional AABB in local [0,1] space to filter voxels
    ///
    /// # Returns
    /// Rapier Collider containing compound shape of exposed faces in the region
    pub fn from_cube_region(
        cube: &Rc<Cube<u8>>,
        _max_depth: u32,
        region: Option<&Aabb>,
    ) -> Collider {
        match region {
            Some(aabb) => {
                // Convert AABB to RegionBounds
                // Use depth 3 for a reasonable granularity (8x8x8 cells)
                let depth = 3;
                match RegionBounds::from_local_aabb(aabb.min, aabb.max, depth) {
                    Some(bounds) => Self::from_cube_with_region(cube, Some(&bounds)),
                    None => {
                        // AABB doesn't intersect cube - return minimal collider
                        ColliderBuilder::ball(0.001).build()
                    }
                }
            }
            None => Self::from_cube_with_region(cube, None),
        }
    }

    // ===== CubeBox Methods =====
    // These methods handle models with non-uniform dimensions that differ
    // from the power-of-2 octree size.

    /// Generate a compound collider from a CubeBox model with actual dimensions
    ///
    /// CubeBox contains size information that may differ from power-of-2 octree dimensions.
    /// This method accounts for that scaling to generate colliders matching the actual model bounds.
    ///
    /// # Arguments
    /// * `cubebox` - The CubeBox with octree, size, and depth information
    ///
    /// # Returns
    /// Rapier Collider with faces scaled to match cubebox.size (not octree power-of-2)
    ///
    /// # Example
    /// A 16x30x12 avatar in a depth-5 octree (32³) will have colliders scaled by
    /// (0.5, 0.9375, 0.375) compared to the full octree space.
    pub fn from_cubebox(cubebox: &CubeBox<u8>) -> Collider {
        Self::from_cubebox_with_region(cubebox, None)
    }

    /// Generate a collider from CubeBox with world-space scaling
    ///
    /// Combines CubeBox dimension scaling with world-space transformation.
    ///
    /// # Arguments
    /// * `cubebox` - The CubeBox model
    /// * `world_size` - The world size for positioning
    pub fn from_cubebox_scaled(cubebox: &CubeBox<u8>, world_size: f32) -> Collider {
        Self::from_cubebox_with_region_scaled(cubebox, None, world_size)
    }

    /// Generate a collider from CubeBox with optional region filtering
    ///
    /// # Arguments
    /// * `cubebox` - The CubeBox model with actual dimensions
    /// * `region` - Optional region bounds to filter voxels
    pub fn from_cubebox_with_region(
        cubebox: &CubeBox<u8>,
        region: Option<&RegionBounds>,
    ) -> Collider {
        let mut builder = Self::new();
        let border_materials = [1, 1, 0, 0];

        // Calculate per-axis scale: actual size / octree size
        let octree_size = cubebox.octree_size() as f32;
        let scale = Vec3::new(
            cubebox.size.x as f32 / octree_size,
            cubebox.size.y as f32 / octree_size,
            cubebox.size.z as f32 / octree_size,
        );

        match region {
            Some(bounds) => {
                visit_faces_in_region(
                    &Rc::new(cubebox.cube.clone()),
                    bounds,
                    |face_info| {
                        builder.add_face_from_info_scaled(face_info, scale);
                    },
                    border_materials,
                );
            }
            None => {
                visit_faces(
                    &Rc::new(cubebox.cube.clone()),
                    |face_info| {
                        builder.add_face_from_info_scaled(face_info, scale);
                    },
                    border_materials,
                );
            }
        }

        builder.build_compound_collider()
    }

    /// Generate a collider from CubeBox with region filtering and world-space scaling
    ///
    /// # Arguments
    /// * `cubebox` - The CubeBox model with actual dimensions
    /// * `region` - Optional region bounds to filter voxels
    /// * `world_size` - The world size for positioning
    pub fn from_cubebox_with_region_scaled(
        cubebox: &CubeBox<u8>,
        region: Option<&RegionBounds>,
        world_size: f32,
    ) -> Collider {
        let mut builder = Self::new();
        let border_materials = [1, 1, 0, 0];

        // Calculate per-axis scale: actual size / octree size
        let octree_size = cubebox.octree_size() as f32;
        let scale = Vec3::new(
            cubebox.size.x as f32 / octree_size,
            cubebox.size.y as f32 / octree_size,
            cubebox.size.z as f32 / octree_size,
        );

        match region {
            Some(bounds) => {
                visit_faces_in_region(
                    &Rc::new(cubebox.cube.clone()),
                    bounds,
                    |face_info| {
                        builder.add_face_from_info_scaled(face_info, scale);
                    },
                    border_materials,
                );
            }
            None => {
                visit_faces(
                    &Rc::new(cubebox.cube.clone()),
                    |face_info| {
                        builder.add_face_from_info_scaled(face_info, scale);
                    },
                    border_materials,
                );
            }
        }

        builder.build_compound_collider_scaled(world_size)
    }

    /// Add a face rectangle from FaceInfo (uniform size for unit cubes)
    fn add_face_from_info(&mut self, face_info: &FaceInfo) {
        // For uniform cubes, size is the same in all directions
        self.add_face_from_info_scaled(face_info, Vec3::ONE);
    }

    /// Add a face rectangle from FaceInfo with per-axis scaling
    ///
    /// Used for CubeBox models where actual dimensions differ from octree power-of-2.
    /// The scale parameter represents size/octree_size for each axis.
    ///
    /// # Arguments
    /// * `face_info` - Face information from traversal (in [0,1] normalized space)
    /// * `scale` - Per-axis scale factor (e.g., (0.5, 0.9375, 0.375) for 16x30x12 in 32³)
    fn add_face_from_info_scaled(&mut self, face_info: &FaceInfo, scale: Vec3) {
        let normal_array = face_info.face.normal();
        let normal = Vec3::from(normal_array);

        // Scale position and size by per-axis factors
        let scaled_pos = face_info.position * scale;
        let scaled_size = Vec3::splat(face_info.size) * scale;

        // Calculate face center position
        // Face center is at voxel center + half size in normal direction
        let voxel_center = scaled_pos + scaled_size * 0.5;
        let face_offset = normal * (scaled_size * 0.5);
        let face_center = voxel_center + face_offset;

        self.rectangles.push(FaceRectangle {
            center: face_center,
            normal,
            size: scaled_size,
        });
    }

    /// Build a collider from all collected face rectangles
    fn build_compound_collider(self) -> Collider {
        self.build_collider_with_scale(1.0, Vec3::ZERO)
    }

    /// Build a collider from all collected face rectangles with world-space scaling
    ///
    /// # Arguments
    /// * `world_size` - The world size (collider will span [-world_size/2, world_size/2])
    fn build_compound_collider_scaled(self, world_size: f32) -> Collider {
        // Transform from [0,1] to [-half_world, half_world]
        let half_world = world_size / 2.0;
        let offset = Vec3::splat(-half_world); // Shift [0,1] to [-0.5,0.5] then scale
        self.build_collider_with_scale(world_size, offset)
    }

    /// Build a collider with scale and offset
    fn build_collider_with_scale(self, scale: f32, offset: Vec3) -> Collider {
        if self.rectangles.is_empty() {
            // Empty collider - just use a tiny sphere
            return ColliderBuilder::ball(0.001).build();
        }

        self.build_trimesh_collider(scale, offset)
    }

    /// Build a triangle mesh collider from face rectangles
    ///
    /// Each face quad is converted to two triangles. This creates infinitely thin
    /// collision surfaces with accurate edge handling.
    fn build_trimesh_collider(self, scale: f32, offset: Vec3) -> Collider {
        // Pre-allocate: 4 vertices per face, 2 triangles (6 indices) per face
        let mut vertices: Vec<Point<Real>> = Vec::with_capacity(self.rectangles.len() * 4);
        let mut indices: Vec<[u32; 3]> = Vec::with_capacity(self.rectangles.len() * 2);

        for rect in &self.rectangles {
            // Get face dimensions based on normal direction
            let (width, height) = Self::face_dimensions(&rect.size, &rect.normal);
            let half_width = width * scale / 2.0;
            let half_height = height * scale / 2.0;

            // Transform center to world coords
            let center = rect.center * scale + offset;

            // Calculate the two tangent vectors perpendicular to the normal
            let (tangent_u, tangent_v) = Self::face_tangents(&rect.normal);

            // Calculate the 4 corners of the face quad
            // Corners are: center ± half_width*tangent_u ± half_height*tangent_v
            let u_offset = tangent_u * half_width;
            let v_offset = tangent_v * half_height;

            let base_idx = vertices.len() as u32;

            // Add 4 vertices (counter-clockwise when viewed from normal direction)
            // v3 --- v2
            // |      |
            // v0 --- v1
            vertices.push(Point::new(
                center.x - u_offset.x - v_offset.x,
                center.y - u_offset.y - v_offset.y,
                center.z - u_offset.z - v_offset.z,
            ));
            vertices.push(Point::new(
                center.x + u_offset.x - v_offset.x,
                center.y + u_offset.y - v_offset.y,
                center.z + u_offset.z - v_offset.z,
            ));
            vertices.push(Point::new(
                center.x + u_offset.x + v_offset.x,
                center.y + u_offset.y + v_offset.y,
                center.z + u_offset.z + v_offset.z,
            ));
            vertices.push(Point::new(
                center.x - u_offset.x + v_offset.x,
                center.y - u_offset.y + v_offset.y,
                center.z - u_offset.z + v_offset.z,
            ));

            // Add 2 triangles with winding that produces outward-facing normals
            // Counter-clockwise when viewed from the face normal direction
            // Triangle 1: v0, v2, v1 (reversed from original v0, v1, v2)
            // Triangle 2: v0, v3, v2 (reversed from original v0, v2, v3)
            indices.push([base_idx, base_idx + 2, base_idx + 1]);
            indices.push([base_idx, base_idx + 3, base_idx + 2]);
        }

        // Use FIX_INTERNAL_EDGES to improve collision quality at face boundaries
        // Fall back to regular trimesh if flags fail (shouldn't happen with valid data)
        match ColliderBuilder::trimesh_with_flags(
            vertices,
            indices,
            TriMeshFlags::FIX_INTERNAL_EDGES,
        ) {
            Ok(builder) => builder.build(),
            Err(_) => {
                // This shouldn't happen with valid face data, but handle gracefully
                ColliderBuilder::ball(0.001).build()
            }
        }
    }

    /// Calculate tangent vectors for a face based on its normal
    ///
    /// Returns two orthogonal unit vectors that lie in the face plane.
    fn face_tangents(normal: &Vec3) -> (Vec3, Vec3) {
        // Choose tangent vectors based on which axis the normal is aligned with
        if normal.x.abs() > 0.5 {
            // X-axis normal (Left/Right faces)
            // Tangent U = Y axis, Tangent V = Z axis
            (Vec3::Y, Vec3::Z)
        } else if normal.y.abs() > 0.5 {
            // Y-axis normal (Top/Bottom faces)
            // Tangent U = X axis, Tangent V = Z axis
            (Vec3::X, Vec3::Z)
        } else {
            // Z-axis normal (Front/Back faces)
            // Tangent U = X axis, Tangent V = Y axis
            (Vec3::X, Vec3::Y)
        }
    }

    /// Determine face dimensions based on size vector and face normal
    ///
    /// Returns (width, height) for the face plane perpendicular to the normal.
    fn face_dimensions(size: &Vec3, normal: &Vec3) -> (f32, f32) {
        // Face lies on two axes perpendicular to the normal
        // For X-facing faces (Left/Right): use Y and Z
        // For Y-facing faces (Top/Bottom): use X and Z
        // For Z-facing faces (Front/Back): use X and Y
        if normal.x.abs() > 0.5 {
            // X-axis normal (Left/Right)
            (size.y, size.z)
        } else if normal.y.abs() > 0.5 {
            // Y-axis normal (Top/Bottom)
            (size.x, size.z)
        } else {
            // Z-axis normal (Front/Back)
            (size.x, size.y)
        }
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
    use glam::IVec3;

    #[test]
    fn test_collider_from_solid_cube() {
        let cube = Rc::new(Cube::Solid(1));
        let collider = VoxelColliderBuilder::from_cube(&cube, 3);

        // Solid cube generates trimesh collider
        assert!(
            collider.shape().as_trimesh().is_some(),
            "Expected trimesh collider for solid cube"
        );
    }

    #[test]
    fn test_collider_from_empty_cube() {
        let cube = Rc::new(Cube::Solid(0)); // Empty
        let collider = VoxelColliderBuilder::from_cube(&cube, 3);

        // Empty cube should generate minimal collider (ball)
        assert!(collider.shape().as_ball().is_some());
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

    #[test]
    fn test_from_cube_with_region() {
        let cube = Rc::new(Cube::Solid(1));

        // Full traversal - default is trimesh mode
        let full_collider = VoxelColliderBuilder::from_cube_with_region(&cube, None);
        assert!(
            full_collider.shape().as_trimesh().is_some(),
            "Expected trimesh collider"
        );

        // Partial region (corner)
        let bounds = RegionBounds::from_local_aabb(Vec3::ZERO, Vec3::splat(0.4), 2).unwrap();
        let partial_collider = VoxelColliderBuilder::from_cube_with_region(&cube, Some(&bounds));

        // Both should produce valid colliders (trimesh or ball if empty)
        assert!(
            partial_collider.shape().as_trimesh().is_some()
                || partial_collider.shape().as_ball().is_some()
        );
    }

    #[test]
    fn test_from_cube_region_with_aabb() {
        let cube = Rc::new(Cube::Solid(1));

        // Test with AABB filter
        let aabb = Aabb::new(Vec3::ZERO, Vec3::splat(0.5));
        let collider = VoxelColliderBuilder::from_cube_region(&cube, 3, Some(&aabb));

        assert!(collider.shape().as_trimesh().is_some() || collider.shape().as_ball().is_some());
    }

    #[test]
    fn test_empty_region() {
        let cube = Rc::new(Cube::Solid(1));

        // AABB completely outside the cube
        let aabb = Aabb::new(Vec3::splat(10.0), Vec3::splat(11.0));
        let collider = VoxelColliderBuilder::from_cube_region(&cube, 3, Some(&aabb));

        // Should return minimal collider
        assert!(collider.shape().as_ball().is_some());
    }

    #[test]
    fn test_region_reduces_faces() {
        // Create a subdivided cube
        let cube = Rc::new(Cube::tabulate(|_| Cube::Solid(1)));

        // Full traversal
        let mut full_builder = VoxelColliderBuilder::new();
        visit_faces(&cube, |f| full_builder.add_face_from_info(f), [1, 1, 0, 0]);
        let full_count = full_builder.face_count();

        // Partial region
        let bounds = RegionBounds::from_local_aabb(Vec3::ZERO, Vec3::splat(0.4), 2).unwrap();
        let mut partial_builder = VoxelColliderBuilder::new();
        visit_faces_in_region(
            &cube,
            &bounds,
            |f| partial_builder.add_face_from_info(f),
            [1, 1, 0, 0],
        );
        let partial_count = partial_builder.face_count();

        // Region should have fewer faces
        assert!(
            partial_count < full_count,
            "Partial count {} should be < full count {}",
            partial_count,
            full_count
        );
    }

    // ===== CubeBox Tests =====

    #[test]
    fn test_collider_from_cubebox_uniform() {
        // A 32x32x32 model in depth-5 octree fills entire space
        let cube = Cube::Solid(1u8);
        let cubebox = CubeBox::new(cube, IVec3::splat(32), 5);
        let collider = VoxelColliderBuilder::from_cubebox(&cubebox);

        // Default mode is Trimesh - should generate trimesh collider with 6 faces (12 triangles)
        assert!(
            collider.shape().as_trimesh().is_some(),
            "Expected trimesh collider for cubebox"
        );
    }

    #[test]
    fn test_collider_from_cubebox_non_uniform() {
        // A 16x30x12 avatar in depth-5 octree (32³)
        let cube = Cube::Solid(1u8);
        let cubebox = CubeBox::new(cube, IVec3::new(16, 30, 12), 5);
        let collider = VoxelColliderBuilder::from_cubebox(&cubebox);

        // Should generate trimesh collider
        assert!(
            collider.shape().as_trimesh().is_some(),
            "Expected trimesh collider for non-uniform cubebox"
        );
    }

    #[test]
    fn test_collider_from_empty_cubebox() {
        // Empty cube in CubeBox
        let cube = Cube::Solid(0u8);
        let cubebox = CubeBox::new(cube, IVec3::splat(16), 4);
        let collider = VoxelColliderBuilder::from_cubebox(&cubebox);

        // Empty should generate minimal collider (ball)
        assert!(collider.shape().as_ball().is_some());
    }

    #[test]
    fn test_cubebox_face_count_scaling() {
        // Compare face counts: uniform vs non-uniform
        // Both should have same face count since it's same solid cube,
        // just different sizing
        let cube_uniform = Cube::Solid(1u8);
        let cubebox_uniform = CubeBox::new(cube_uniform, IVec3::splat(32), 5);

        let cube_nonuniform = Cube::Solid(1u8);
        let cubebox_nonuniform = CubeBox::new(cube_nonuniform, IVec3::new(16, 30, 12), 5);

        let mut builder_uniform = VoxelColliderBuilder::new();
        visit_faces(
            &Rc::new(cubebox_uniform.cube.clone()),
            |f| builder_uniform.add_face_from_info(f),
            [1, 1, 0, 0],
        );

        let mut builder_nonuniform = VoxelColliderBuilder::new();
        let scale = Vec3::new(0.5, 0.9375, 0.375);
        visit_faces(
            &Rc::new(cubebox_nonuniform.cube.clone()),
            |f| builder_nonuniform.add_face_from_info_scaled(f, scale),
            [1, 1, 0, 0],
        );

        // Same cube = same face count
        assert_eq!(
            builder_uniform.face_count(),
            builder_nonuniform.face_count()
        );
    }

    #[test]
    fn test_face_dimensions() {
        let size = Vec3::new(0.5, 0.9375, 0.375);

        // X-facing face uses Y and Z
        let (w, h) = VoxelColliderBuilder::face_dimensions(&size, &Vec3::X);
        assert!((w - 0.9375).abs() < 0.0001);
        assert!((h - 0.375).abs() < 0.0001);

        // Y-facing face uses X and Z
        let (w, h) = VoxelColliderBuilder::face_dimensions(&size, &Vec3::Y);
        assert!((w - 0.5).abs() < 0.0001);
        assert!((h - 0.375).abs() < 0.0001);

        // Z-facing face uses X and Y
        let (w, h) = VoxelColliderBuilder::face_dimensions(&size, &Vec3::Z);
        assert!((w - 0.5).abs() < 0.0001);
        assert!((h - 0.9375).abs() < 0.0001);
    }

    #[test]
    fn test_face_tangents() {
        // X-facing faces use Y and Z as tangents
        let (u, v) = VoxelColliderBuilder::face_tangents(&Vec3::X);
        assert_eq!(u, Vec3::Y);
        assert_eq!(v, Vec3::Z);

        let (u, v) = VoxelColliderBuilder::face_tangents(&-Vec3::X);
        assert_eq!(u, Vec3::Y);
        assert_eq!(v, Vec3::Z);

        // Y-facing faces use X and Z as tangents
        let (u, v) = VoxelColliderBuilder::face_tangents(&Vec3::Y);
        assert_eq!(u, Vec3::X);
        assert_eq!(v, Vec3::Z);

        // Z-facing faces use X and Y as tangents
        let (u, v) = VoxelColliderBuilder::face_tangents(&Vec3::Z);
        assert_eq!(u, Vec3::X);
        assert_eq!(v, Vec3::Y);
    }

    #[test]
    fn test_trimesh_triangle_count() {
        // Verify trimesh is valid:
        // - Has triangles
        // - All indices reference valid vertices
        let cube = Rc::new(Cube::Solid(1));
        let collider = VoxelColliderBuilder::from_cube(&cube, 3);

        let trimesh = collider.shape().as_trimesh().expect("Expected trimesh");

        let num_triangles = trimesh.indices().len();
        let num_vertices = trimesh.vertices().len();

        // Should have at least some triangles (solid cube has 6 faces = 12 triangles minimum)
        assert!(
            num_triangles >= 10,
            "Solid cube should have at least 10 triangles, got {}",
            num_triangles
        );

        // Rapier may deduplicate vertices, so we just verify indices are valid
        for tri in trimesh.indices() {
            assert!(
                (tri[0] as usize) < num_vertices,
                "Triangle index {} out of bounds (vertices: {})",
                tri[0],
                num_vertices
            );
            assert!(
                (tri[1] as usize) < num_vertices,
                "Triangle index {} out of bounds (vertices: {})",
                tri[1],
                num_vertices
            );
            assert!(
                (tri[2] as usize) < num_vertices,
                "Triangle index {} out of bounds (vertices: {})",
                tri[2],
                num_vertices
            );
        }
    }
}
