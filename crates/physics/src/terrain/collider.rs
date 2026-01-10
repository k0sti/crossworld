//! Main voxel terrain collider
//!
//! Implements the `VoxelTerrainCollider` which integrates with Rapier's
//! collision detection system using `TypedCompositeShape`.

use super::region_cache::RegionCollisionData;
use super::region_id::{RegionId, TerrainPartId};
use super::triangle_gen::face_aabb;
use cube::Cube;
use rapier3d::parry::bounding_volume::Aabb;
use rapier3d::parry::partitioning::{Bvh, BvhBuildStrategy};
use std::collections::HashMap;
use std::sync::Arc;

/// Voxel terrain collider using TypedCompositeShape
///
/// This collider presents voxel terrain as a single shape to Rapier while
/// generating triangle geometry lazily during collision queries.
///
/// # Architecture
///
/// Uses a two-level BVH structure:
/// - **Region BVH**: Coarse spatial index of all non-empty regions
/// - **Triangle BVH**: Fine index built for active regions near dynamic bodies
///
/// The triangle BVH is rebuilt when dynamic bodies move significantly,
/// controlled by `ActiveRegionTracker`.
pub struct VoxelTerrainCollider {
    /// Coarse BVH indexing regions
    #[allow(dead_code)]
    region_bvh: Bvh,

    /// Fine BVH indexing individual triangles within active regions
    /// Leaf data is TerrainPartId packed as u32 (lower 32 bits)
    triangle_bvh: Bvh,

    /// Reference to voxel octree
    cube: Arc<Cube<u8>>,

    /// World size in units
    world_size: f32,

    /// Border materials for octree traversal
    border_materials: [u8; 4],

    /// Region depth for collision queries (higher = finer regions)
    region_depth: u32,

    /// Cache of region collision data
    region_cache: HashMap<RegionId, RegionCollisionData>,

    /// Terrain modification counter for cache invalidation
    terrain_version: u64,

    /// Total bounding box of terrain (world space)
    global_aabb: Aabb,

    /// Mapping from BVH leaf index to TerrainPartId
    /// Stored separately because Bvh only stores u32 leaf indices
    part_id_map: Vec<TerrainPartId>,
}

impl VoxelTerrainCollider {
    /// Create a new terrain collider
    ///
    /// # Arguments
    /// * `cube` - The voxel octree
    /// * `world_size` - World size in units
    /// * `region_depth` - Depth for region subdivision (3-4 recommended)
    /// * `border_materials` - Material IDs for border layers \[y0, y1, y2, y3\]
    pub fn new(
        cube: Arc<Cube<u8>>,
        world_size: f32,
        region_depth: u32,
        border_materials: [u8; 4],
    ) -> Self {
        // Compute global AABB centered at origin
        let half_world = world_size / 2.0;
        let global_aabb = Aabb::new(
            [-half_world, -half_world, -half_world].into(),
            [half_world, half_world, half_world].into(),
        );

        // Initialize empty BVHs
        let region_bvh = Bvh::default();
        let triangle_bvh = Bvh::default();

        Self {
            region_bvh,
            triangle_bvh,
            cube,
            world_size,
            border_materials,
            region_depth,
            region_cache: HashMap::new(),
            terrain_version: 0,
            global_aabb,
            part_id_map: Vec::new(),
        }
    }

    /// Get the world size
    pub fn world_size(&self) -> f32 {
        self.world_size
    }

    /// Get the region depth
    pub fn region_depth(&self) -> u32 {
        self.region_depth
    }

    /// Get the current terrain version
    pub fn terrain_version(&self) -> u64 {
        self.terrain_version
    }

    /// Get the global AABB
    pub fn global_aabb(&self) -> &Aabb {
        &self.global_aabb
    }

    /// Get the triangle BVH for Rapier queries
    pub fn triangle_bvh(&self) -> &Bvh {
        &self.triangle_bvh
    }

    /// Get the part ID map for looking up TerrainPartId from BVH leaf index
    pub fn part_id_map(&self) -> &[TerrainPartId] {
        &self.part_id_map
    }

    /// Called when terrain voxels are modified
    ///
    /// Invalidates cache for affected regions and increments terrain version.
    ///
    /// # Arguments
    /// * `affected_regions` - Regions that need cache invalidation
    pub fn on_terrain_modified(&mut self, affected_regions: &[RegionId]) {
        self.terrain_version += 1;

        for &region in affected_regions {
            // Invalidate cache for this region
            self.region_cache.remove(&region);
        }

        // TODO: Update region_bvh when we implement full region tracking
    }

    /// Clear all cached region data
    pub fn clear_cache(&mut self) {
        self.region_cache.clear();
        self.terrain_version += 1;
    }

    /// Update the terrain collider with a new octree
    ///
    /// This replaces the internal cube reference and clears all caches.
    pub fn update_cube(&mut self, cube: Arc<Cube<u8>>) {
        self.cube = cube;
        self.clear_cache();
    }

    /// Update the triangle BVH for regions intersecting the active area
    ///
    /// Called before physics step to rebuild the fine BVH for collision queries.
    ///
    /// # Arguments
    /// * `active_aabb` - AABB of the active region (typically from ActiveRegionTracker)
    pub fn update_triangle_bvh(&mut self, active_aabb: &Aabb) {
        let mut aabbs: Vec<Aabb> = Vec::new();
        let mut part_ids: Vec<TerrainPartId> = Vec::new();

        // Find all regions intersecting active area
        for region in RegionId::from_world_aabb(active_aabb, self.world_size, self.region_depth) {
            // Ensure region is cached
            let data = self.region_cache.entry(region).or_insert_with(|| {
                RegionCollisionData::from_octree(
                    &self.cube,
                    region,
                    self.world_size,
                    self.border_materials,
                )
            });

            // Skip empty regions
            if data.is_empty() {
                continue;
            }

            // Add all triangles from this region
            for (face_idx, face) in data.faces().iter().enumerate() {
                let face_box = face_aabb(face, self.world_size);

                // Two triangles per face
                for tri_in_face in 0..2u16 {
                    let tri_idx = (face_idx as u16) * 2 + tri_in_face;
                    let part_id = TerrainPartId::new(region, tri_idx);
                    aabbs.push(face_box);
                    part_ids.push(part_id);
                }
            }
        }

        // Store the part ID mapping
        self.part_id_map = part_ids;

        // Rebuild the triangle BVH
        if !aabbs.is_empty() {
            self.triangle_bvh = Bvh::from_leaves(BvhBuildStrategy::Binned, &aabbs);
        } else {
            self.triangle_bvh = Bvh::default();
        }
    }

    /// Get cached region data if available
    pub fn get_cached_region(&self, region: &RegionId) -> Option<&RegionCollisionData> {
        self.region_cache.get(region)
    }

    /// Populate cache for a specific region
    pub fn cache_region(&mut self, region: RegionId) -> &RegionCollisionData {
        self.region_cache.entry(region).or_insert_with(|| {
            RegionCollisionData::from_octree(
                &self.cube,
                region,
                self.world_size,
                self.border_materials,
            )
        })
    }

    /// Get triangle by part ID
    ///
    /// Returns None if the region is not cached or triangle index is invalid.
    pub fn get_triangle(&self, part_id: TerrainPartId) -> Option<rapier3d::parry::shape::Triangle> {
        let region = part_id.region();
        let tri_idx = part_id.triangle_idx();

        self.region_cache
            .get(&region)
            .and_then(|data| data.get_triangle(tri_idx, self.world_size))
    }

    /// Get triangle by BVH leaf index
    ///
    /// Returns None if index is out of bounds or region is not cached.
    pub fn get_triangle_by_index(&self, leaf_idx: u32) -> Option<rapier3d::parry::shape::Triangle> {
        self.part_id_map
            .get(leaf_idx as usize)
            .and_then(|&part_id| self.get_triangle(part_id))
    }

    /// Number of cached regions
    pub fn cached_region_count(&self) -> usize {
        self.region_cache.len()
    }

    /// Total number of triangles in the current triangle BVH
    pub fn active_triangle_count(&self) -> usize {
        self.part_id_map.len()
    }

    /// Generate a TriMesh shape from current active triangles
    ///
    /// Returns None if there are no active triangles.
    /// The TriMesh can be used to create a Rapier collider.
    pub fn to_trimesh(&self) -> Option<rapier3d::prelude::SharedShape> {
        use rapier3d::prelude::*;

        if self.part_id_map.is_empty() {
            return None;
        }

        let mut vertices = Vec::with_capacity(self.part_id_map.len() * 3);
        let mut indices = Vec::with_capacity(self.part_id_map.len());

        for (tri_idx, &part_id) in self.part_id_map.iter().enumerate() {
            if let Some(triangle) = self.get_triangle(part_id) {
                let base_idx = (tri_idx * 3) as u32;
                vertices.push(triangle.a);
                vertices.push(triangle.b);
                vertices.push(triangle.c);
                indices.push([base_idx, base_idx + 1, base_idx + 2]);
            }
        }

        if vertices.is_empty() {
            return None;
        }

        SharedShape::trimesh(vertices, indices).ok()
    }
}

impl Clone for VoxelTerrainCollider {
    fn clone(&self) -> Self {
        Self {
            region_bvh: Bvh::default(), // Fresh BVH, will be rebuilt
            triangle_bvh: Bvh::default(),
            cube: self.cube.clone(),
            world_size: self.world_size,
            border_materials: self.border_materials,
            region_depth: self.region_depth,
            region_cache: HashMap::new(), // Fresh cache
            terrain_version: self.terrain_version,
            global_aabb: self.global_aabb,
            part_id_map: Vec::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_test_collider() -> VoxelTerrainCollider {
        let cube = Arc::new(Cube::Solid(1));
        VoxelTerrainCollider::new(cube, 100.0, 2, [0, 0, 0, 0])
    }

    #[test]
    fn test_new_collider() {
        let collider = make_test_collider();

        assert_eq!(collider.world_size(), 100.0);
        assert_eq!(collider.region_depth(), 2);
        assert_eq!(collider.terrain_version(), 0);
        assert_eq!(collider.cached_region_count(), 0);
    }

    #[test]
    fn test_global_aabb() {
        let collider = make_test_collider();
        let aabb = collider.global_aabb();

        assert!((aabb.mins.x - (-50.0)).abs() < 1e-6);
        assert!((aabb.mins.y - (-50.0)).abs() < 1e-6);
        assert!((aabb.mins.z - (-50.0)).abs() < 1e-6);
        assert!((aabb.maxs.x - 50.0).abs() < 1e-6);
        assert!((aabb.maxs.y - 50.0).abs() < 1e-6);
        assert!((aabb.maxs.z - 50.0).abs() < 1e-6);
    }

    #[test]
    fn test_cache_region() {
        let mut collider = make_test_collider();
        let region = RegionId::new(glam::IVec3::ZERO, 2);

        // Cache should be empty initially
        assert!(collider.get_cached_region(&region).is_none());

        // Cache the region
        collider.cache_region(region);

        // Now should be cached
        assert!(collider.get_cached_region(&region).is_some());
        assert_eq!(collider.cached_region_count(), 1);
    }

    #[test]
    fn test_terrain_modified() {
        let mut collider = make_test_collider();
        let region = RegionId::new(glam::IVec3::ZERO, 2);

        // Cache a region
        collider.cache_region(region);
        assert_eq!(collider.cached_region_count(), 1);
        assert_eq!(collider.terrain_version(), 0);

        // Modify terrain
        collider.on_terrain_modified(&[region]);

        // Cache should be invalidated
        assert!(collider.get_cached_region(&region).is_none());
        assert_eq!(collider.terrain_version(), 1);
    }

    #[test]
    fn test_clear_cache() {
        let mut collider = make_test_collider();

        // Cache several regions
        for i in 0..4 {
            let region = RegionId::new(glam::IVec3::new(i, 0, 0), 2);
            collider.cache_region(region);
        }
        assert_eq!(collider.cached_region_count(), 4);

        // Clear cache
        collider.clear_cache();

        assert_eq!(collider.cached_region_count(), 0);
    }

    #[test]
    fn test_update_triangle_bvh() {
        let mut collider = make_test_collider();

        // Query a small region
        let active_aabb = Aabb::new([-10.0, -10.0, -10.0].into(), [10.0, 10.0, 10.0].into());

        collider.update_triangle_bvh(&active_aabb);

        // Should have cached some regions
        assert!(collider.cached_region_count() > 0);
    }

    #[test]
    fn test_update_cube() {
        let mut collider = make_test_collider();

        // Cache some regions
        let region = RegionId::new(glam::IVec3::ZERO, 2);
        collider.cache_region(region);
        assert_eq!(collider.cached_region_count(), 1);

        // Update with new cube
        let new_cube = Arc::new(Cube::Solid(2));
        collider.update_cube(new_cube);

        // Cache should be cleared
        assert_eq!(collider.cached_region_count(), 0);
    }

    #[test]
    fn test_clone() {
        let mut collider = make_test_collider();

        // Cache a region
        let region = RegionId::new(glam::IVec3::ZERO, 2);
        collider.cache_region(region);

        // Clone
        let cloned = collider.clone();

        // Clone should have fresh cache
        assert_eq!(cloned.cached_region_count(), 0);
        assert_eq!(cloned.world_size(), collider.world_size());
        assert_eq!(cloned.terrain_version(), collider.terrain_version());
    }
}
