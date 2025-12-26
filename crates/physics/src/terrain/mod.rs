//! Terrain collision system using Rapier's TypedCompositeShape
//!
//! This module implements lazy on-demand triangle generation from voxel data for
//! efficient terrain collision. The terrain appears as a single collider to Rapier,
//! but geometry is generated only for regions actively queried during collision detection.
//!
//! # Architecture
//!
//! - [`RegionId`]: Identifies a collision region in the octree using corner-based coordinates
//! - [`RegionCollisionData`]: Caches faces for a region, avoiding repeated octree traversal
//! - [`VoxelTerrainCollider`]: Main collider implementing `TypedCompositeShape`
//! - [`ActiveRegionTracker`]: Tracks which regions need triangle-level indexing
//!
//! # Usage
//!
//! ```ignore
//! use crossworld_physics::terrain::{VoxelTerrainCollider, ActiveRegionTracker};
//!
//! // Create terrain collider from octree
//! let terrain = VoxelTerrainCollider::new(cube, world_size, region_depth, border_materials);
//!
//! // Each physics frame, update active regions based on dynamic bodies
//! let tracker = ActiveRegionTracker::new(margin);
//! if let Some(active_aabb) = tracker.update(&dynamic_aabbs) {
//!     terrain.update_triangle_bvh(&active_aabb);
//! }
//! ```

mod active_region;
mod collider;
mod region_cache;
mod region_id;
mod shape_impl;
mod triangle_gen;

pub use active_region::ActiveRegionTracker;
pub use collider::VoxelTerrainCollider;
pub use region_cache::RegionCollisionData;
pub use region_id::RegionId;
pub use triangle_gen::{face_to_triangle, face_to_triangles};
