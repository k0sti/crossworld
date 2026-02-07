//! Shape trait implementations for VoxelTerrainCollider
//!
//! Implements the Rapier/Parry shape traits for terrain collision.
//! Uses `CompositeShape` and `TypedCompositeShape` for collision queries via the BVH.

use super::collider::VoxelTerrainCollider;
use rapier3d::math::{Isometry, Real};
use rapier3d::parry::partitioning::Bvh;
use rapier3d::parry::query::details::NormalConstraints;
use rapier3d::parry::shape::{CompositeShape, Shape, Triangle, TypedCompositeShape};

/// Implement CompositeShape for VoxelTerrainCollider
///
/// This allows Rapier to query the terrain collider for collision detection.
/// The BVH indexes triangles by leaf index (u32).
impl CompositeShape for VoxelTerrainCollider {
    fn map_part_at(
        &self,
        shape_id: u32,
        f: &mut dyn FnMut(Option<&Isometry<Real>>, &dyn Shape, Option<&dyn NormalConstraints>),
    ) {
        if let Some(triangle) = self.get_triangle_by_index(shape_id) {
            // No transform - triangles are already in world space
            f(None, &triangle, None);
        }
    }

    fn bvh(&self) -> &Bvh {
        self.triangle_bvh()
    }
}

/// Implement TypedCompositeShape for VoxelTerrainCollider
///
/// This is the preferred interface for Rapier queries, providing typed access
/// to Triangle shapes.
impl TypedCompositeShape for VoxelTerrainCollider {
    type PartShape = Triangle;
    type PartNormalConstraints = ();

    fn map_typed_part_at<T>(
        &self,
        shape_id: u32,
        mut f: impl FnMut(
            Option<&Isometry<Real>>,
            &Self::PartShape,
            Option<&Self::PartNormalConstraints>,
        ) -> T,
    ) -> Option<T> {
        self.get_triangle_by_index(shape_id)
            .map(|triangle| f(None, &triangle, None))
    }

    fn map_untyped_part_at<T>(
        &self,
        shape_id: u32,
        mut f: impl FnMut(Option<&Isometry<Real>>, &dyn Shape, Option<&dyn NormalConstraints>) -> T,
    ) -> Option<T> {
        self.get_triangle_by_index(shape_id)
            .map(|triangle| f(None, &triangle as &dyn Shape, None))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cube::Cube;
    use glam::IVec3;
    use rapier3d::parry::bounding_volume::Aabb;
    use std::sync::Arc;

    #[test]
    fn test_composite_shape_bvh() {
        // Create a simple solid cube terrain
        let cube = Arc::new(Cube::Solid(1));
        let mut collider = VoxelTerrainCollider::new(cube, 100.0, 2, [0, 0, 0, 0]);

        // Update BVH for center region
        let active_aabb = Aabb::new([-10.0, -10.0, -10.0].into(), [10.0, 10.0, 10.0].into());
        collider.update_triangle_bvh(&active_aabb);

        // Get the BVH via CompositeShape trait
        let _bvh = collider.bvh();

        // Should have some triangles
        println!("Triangle count: {}", collider.active_triangle_count());
    }

    #[test]
    fn test_map_typed_part_at() {
        let cube = Arc::new(Cube::Solid(1));
        let mut collider = VoxelTerrainCollider::new(cube, 100.0, 2, [0, 0, 0, 0]);

        // Cache a region
        let region = super::super::region_id::RegionId::new(IVec3::ZERO, 2);
        collider.cache_region(region);

        // Update BVH
        let active_aabb = Aabb::new([-60.0, -60.0, -60.0].into(), [60.0, 60.0, 60.0].into());
        collider.update_triangle_bvh(&active_aabb);

        // Try to map a triangle
        if collider.active_triangle_count() > 0 {
            let result = collider.map_typed_part_at(0, |_iso, tri, _constraints| {
                // Verify triangle is valid
                assert!(tri.a != tri.b, "Triangle should have distinct vertices");
                true
            });

            assert!(result.is_some(), "Should find triangle at index 0");
        }
    }

    #[test]
    fn test_map_untyped_part_at() {
        let cube = Arc::new(Cube::Solid(1));
        let mut collider = VoxelTerrainCollider::new(cube, 100.0, 2, [0, 0, 0, 0]);

        // Update BVH
        let active_aabb = Aabb::new([-60.0, -60.0, -60.0].into(), [60.0, 60.0, 60.0].into());
        collider.update_triangle_bvh(&active_aabb);

        // Try map_untyped_part_at
        if collider.active_triangle_count() > 0 {
            let result = collider.map_untyped_part_at(0, |_iso, shape, _constraints| {
                // Verify it's a triangle
                assert!(shape.as_triangle().is_some(), "Shape should be a Triangle");
                true
            });

            assert!(result.is_some(), "Should find shape at index 0");
        }
    }

    #[test]
    fn test_map_part_at_invalid_index() {
        let cube = Arc::new(Cube::Solid(1));
        let collider = VoxelTerrainCollider::new(cube, 100.0, 2, [0, 0, 0, 0]);

        // Without building BVH, there are no triangles
        let result = collider.map_typed_part_at(9999, |_iso, _tri, _constraints| true);

        assert!(result.is_none(), "Invalid index should return None");
    }
}
