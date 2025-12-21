//! World collision strategies for voxel terrain
//!
//! This module provides configurable world collision strategies that trade off
//! initialization time, per-frame update cost, and memory usage.
//!
//! # Strategies
//!
//! - **MonolithicCollider**: Single compound collider for entire world (baseline)
//! - **ChunkedCollider**: Loads/unloads chunk colliders based on object proximity
//! - **HybridOctreeCollider**: Bypasses Rapier for world collision using direct octree queries

use crate::collider::VoxelColliderBuilder;
use crate::collision::Aabb;
use crate::PhysicsWorld;
use cube::{visit_faces_in_region, Cube, RegionBounds};
use glam::{IVec3, Vec3};
use rapier3d::prelude::*;
use std::collections::{HashMap, HashSet};
use std::rc::Rc;
use std::time::Instant;

/// Performance metrics for a world collider strategy
#[derive(Debug, Clone, Default)]
pub struct ColliderMetrics {
    /// Name of the collision strategy
    pub strategy_name: &'static str,
    /// Time taken to initialize in milliseconds
    pub init_time_ms: f32,
    /// Average update time per frame in microseconds
    pub update_time_us: f32,
    /// Number of active colliders in physics world
    pub active_colliders: usize,
    /// Total number of faces represented
    pub total_faces: usize,
}

/// Trait for world collision strategies
///
/// Implementors provide different approaches to handling collision between
/// dynamic objects and static world terrain.
pub trait WorldCollider {
    /// Initialize the collider with world cube and physics world
    ///
    /// # Arguments
    /// * `cube` - The world octree cube
    /// * `world_size` - World size in units (collider spans [-world_size/2, world_size/2])
    /// * `border_materials` - Materials for border traversal [bottom_inner, bottom_outer, top_inner, top_outer]
    /// * `physics` - Physics world to add colliders to
    fn init(
        &mut self,
        cube: &Rc<Cube<u8>>,
        world_size: f32,
        border_materials: [u8; 4],
        physics: &mut PhysicsWorld,
    );

    /// Update colliders based on dynamic object positions
    ///
    /// Called each frame before physics step. Strategies may load/unload
    /// colliders based on object proximity.
    ///
    /// # Arguments
    /// * `dynamic_aabbs` - List of (body handle, world AABB) for dynamic objects
    /// * `physics` - Physics world for adding/removing colliders
    fn update(&mut self, dynamic_aabbs: &[(RigidBodyHandle, Aabb)], physics: &mut PhysicsWorld);

    /// Resolve world collisions for a body (for hybrid approach)
    ///
    /// For strategies that bypass Rapier for world collision, this method
    /// computes and returns the penetration correction vector.
    ///
    /// # Arguments
    /// * `body_handle` - Handle to the body to resolve
    /// * `body_aabb` - World AABB of the body
    ///
    /// # Returns
    /// Correction vector to apply to body position
    fn resolve_collision(&self, body_handle: RigidBodyHandle, body_aabb: &Aabb) -> Vec3;

    /// Get performance metrics for this strategy
    fn metrics(&self) -> ColliderMetrics;
}

// ============================================================================
// Monolithic Strategy (Baseline)
// ============================================================================

/// Single compound collider for entire world terrain
///
/// This is the baseline strategy - creates one massive compound collider
/// containing all exposed voxel faces. Simple but inefficient for large worlds.
pub struct MonolithicCollider {
    world_body: Option<RigidBodyHandle>,
    world_collider: Option<ColliderHandle>,
    face_count: usize,
    init_time_ms: f32,
}

impl MonolithicCollider {
    pub fn new() -> Self {
        Self {
            world_body: None,
            world_collider: None,
            face_count: 0,
            init_time_ms: 0.0,
        }
    }
}

impl Default for MonolithicCollider {
    fn default() -> Self {
        Self::new()
    }
}

impl WorldCollider for MonolithicCollider {
    fn init(
        &mut self,
        cube: &Rc<Cube<u8>>,
        world_size: f32,
        _border_materials: [u8; 4],
        physics: &mut PhysicsWorld,
    ) {
        let start = Instant::now();

        // Generate compound collider for entire world
        let collider = VoxelColliderBuilder::from_cube_scaled(cube, 0, world_size);

        // Count shapes in compound collider
        self.face_count = count_compound_shapes(&collider);

        // Add to physics world
        let body = RigidBodyBuilder::fixed().build();
        let body_handle = physics.add_rigid_body(body);
        let collider_handle = physics.add_collider(collider, body_handle);

        self.world_body = Some(body_handle);
        self.world_collider = Some(collider_handle);
        self.init_time_ms = start.elapsed().as_secs_f32() * 1000.0;
    }

    fn update(&mut self, _dynamic_aabbs: &[(RigidBodyHandle, Aabb)], _physics: &mut PhysicsWorld) {
        // No-op: collider is static
    }

    fn resolve_collision(&self, _body_handle: RigidBodyHandle, _body_aabb: &Aabb) -> Vec3 {
        // Rapier handles collision resolution
        Vec3::ZERO
    }

    fn metrics(&self) -> ColliderMetrics {
        ColliderMetrics {
            strategy_name: "monolithic",
            init_time_ms: self.init_time_ms,
            update_time_us: 0.0,
            active_colliders: if self.world_collider.is_some() { 1 } else { 0 },
            total_faces: self.face_count,
        }
    }
}

// ============================================================================
// Chunked Strategy
// ============================================================================

/// Data for an active chunk collider
struct ChunkData {
    collider_handle: ColliderHandle,
    face_count: usize,
}

/// Chunked world collider - loads/unloads chunk colliders on demand
///
/// Divides world into spatial chunks and only maintains colliders for
/// chunks near dynamic objects.
pub struct ChunkedCollider {
    cube: Option<Rc<Cube<u8>>>,
    world_size: f32,
    chunk_size: f32,
    load_radius: f32,
    border_materials: [u8; 4],

    /// World body for all chunk colliders
    world_body: Option<RigidBodyHandle>,

    /// Active chunk colliders: chunk position -> collider data
    active_chunks: HashMap<IVec3, ChunkData>,

    /// Metrics
    init_time_ms: f32,
    last_update_time_us: f32,
    total_faces_loaded: usize,
}

impl ChunkedCollider {
    /// Create a new chunked collider
    ///
    /// # Arguments
    /// * `chunk_size` - Size of each chunk in world units (e.g., 64.0)
    /// * `load_radius` - Distance to load chunks around objects (e.g., 128.0)
    pub fn new(chunk_size: f32, load_radius: f32) -> Self {
        Self {
            cube: None,
            world_size: 0.0,
            chunk_size,
            load_radius,
            border_materials: [1, 1, 0, 0],
            world_body: None,
            active_chunks: HashMap::new(),
            init_time_ms: 0.0,
            last_update_time_us: 0.0,
            total_faces_loaded: 0,
        }
    }

    /// Convert world position to chunk index
    fn world_to_chunk(&self, pos: Vec3) -> IVec3 {
        let half_world = self.world_size / 2.0;
        let normalized = (pos + Vec3::splat(half_world)) / self.chunk_size;
        normalized.floor().as_ivec3()
    }

    /// Get all chunk indices that intersect with an AABB
    fn chunks_in_aabb(&self, aabb: &Aabb) -> impl Iterator<Item = IVec3> {
        let min_chunk = self.world_to_chunk(aabb.min);
        let max_chunk = self.world_to_chunk(aabb.max);

        (min_chunk.x..=max_chunk.x).flat_map(move |x| {
            (min_chunk.y..=max_chunk.y)
                .flat_map(move |y| (min_chunk.z..=max_chunk.z).map(move |z| IVec3::new(x, y, z)))
        })
    }

    /// Generate collider for a single chunk
    fn generate_chunk(&self, chunk_pos: IVec3, physics: &mut PhysicsWorld) -> Option<ChunkData> {
        let cube = self.cube.as_ref()?;
        let world_body = self.world_body?;

        // Convert chunk position to world AABB
        let half_world = self.world_size / 2.0;
        let chunk_min = chunk_pos.as_vec3() * self.chunk_size - Vec3::splat(half_world);
        let chunk_max = chunk_min + Vec3::splat(self.chunk_size);

        // Convert to octree local space [0,1]
        let local_min = (chunk_min + Vec3::splat(half_world)) / self.world_size;
        let local_max = (chunk_max + Vec3::splat(half_world)) / self.world_size;

        // Clamp to [0,1] bounds
        let local_min = local_min.max(Vec3::ZERO);
        let local_max = local_max.min(Vec3::ONE);

        // Early return if chunk is outside world
        if local_min.x >= local_max.x || local_min.y >= local_max.y || local_min.z >= local_max.z {
            return None;
        }

        // Generate collider for just this region using depth 3 granularity
        let depth = 3;
        let bounds = RegionBounds::from_local_aabb(local_min, local_max, depth)?;

        // Count faces first to see if chunk has any solid content
        let mut face_count = 0;
        visit_faces_in_region(
            cube,
            &bounds,
            |_| {
                face_count += 1;
            },
            self.border_materials,
        );

        if face_count == 0 {
            return None; // Empty chunk
        }

        // Generate collider with region filtering
        let collider = VoxelColliderBuilder::from_cube_with_region_scaled(
            cube,
            Some(&bounds),
            self.world_size,
        );

        let handle = physics.add_collider(collider, world_body);

        Some(ChunkData {
            collider_handle: handle,
            face_count,
        })
    }
}

impl WorldCollider for ChunkedCollider {
    fn init(
        &mut self,
        cube: &Rc<Cube<u8>>,
        world_size: f32,
        border_materials: [u8; 4],
        physics: &mut PhysicsWorld,
    ) {
        let start = Instant::now();

        self.cube = Some(cube.clone());
        self.world_size = world_size;
        self.border_materials = border_materials;

        // Create fixed body for all chunk colliders
        let body = RigidBodyBuilder::fixed().build();
        self.world_body = Some(physics.add_rigid_body(body));

        self.init_time_ms = start.elapsed().as_secs_f32() * 1000.0;
    }

    fn update(&mut self, dynamic_aabbs: &[(RigidBodyHandle, Aabb)], physics: &mut PhysicsWorld) {
        let start = Instant::now();

        // Collect required chunks based on dynamic object positions
        let mut required_chunks: HashSet<IVec3> = HashSet::new();
        for (_, aabb) in dynamic_aabbs {
            // Expand AABB by load radius
            let expanded = Aabb::new(
                aabb.min - Vec3::splat(self.load_radius),
                aabb.max + Vec3::splat(self.load_radius),
            );
            for chunk_pos in self.chunks_in_aabb(&expanded) {
                required_chunks.insert(chunk_pos);
            }
        }

        // Unload chunks no longer needed
        let to_unload: Vec<IVec3> = self
            .active_chunks
            .keys()
            .filter(|pos| !required_chunks.contains(pos))
            .cloned()
            .collect();

        for pos in to_unload {
            if let Some(chunk) = self.active_chunks.remove(&pos) {
                physics.remove_collider(chunk.collider_handle);
                self.total_faces_loaded -= chunk.face_count;
            }
        }

        // Load new chunks
        for pos in required_chunks {
            if !self.active_chunks.contains_key(&pos) {
                if let Some(chunk) = self.generate_chunk(pos, physics) {
                    self.total_faces_loaded += chunk.face_count;
                    self.active_chunks.insert(pos, chunk);
                }
            }
        }

        self.last_update_time_us = start.elapsed().as_secs_f32() * 1_000_000.0;
    }

    fn resolve_collision(&self, _body_handle: RigidBodyHandle, _body_aabb: &Aabb) -> Vec3 {
        // Rapier handles collision resolution via chunk colliders
        Vec3::ZERO
    }

    fn metrics(&self) -> ColliderMetrics {
        ColliderMetrics {
            strategy_name: "chunked",
            init_time_ms: self.init_time_ms,
            update_time_us: self.last_update_time_us,
            active_colliders: self.active_chunks.len(),
            total_faces: self.total_faces_loaded,
        }
    }
}

// ============================================================================
// Hybrid Octree Strategy
// ============================================================================

/// Hybrid octree collider - bypasses Rapier for world collision
///
/// Uses Rapier only for dynamic↔dynamic collision. World collision is
/// resolved directly via octree queries, avoiding compound collider overhead.
pub struct HybridOctreeCollider {
    cube: Option<Rc<Cube<u8>>>,
    world_size: f32,
    border_materials: [u8; 4],
    init_time_ms: f32,
}

impl HybridOctreeCollider {
    pub fn new() -> Self {
        Self {
            cube: None,
            world_size: 0.0,
            border_materials: [1, 1, 0, 0],
            init_time_ms: 0.0,
        }
    }
}

impl Default for HybridOctreeCollider {
    fn default() -> Self {
        Self::new()
    }
}

impl WorldCollider for HybridOctreeCollider {
    fn init(
        &mut self,
        cube: &Rc<Cube<u8>>,
        world_size: f32,
        border_materials: [u8; 4],
        _physics: &mut PhysicsWorld,
    ) {
        let start = Instant::now();

        self.cube = Some(cube.clone());
        self.world_size = world_size;
        self.border_materials = border_materials;

        // No Rapier colliders created!

        self.init_time_ms = start.elapsed().as_secs_f32() * 1000.0;
    }

    fn update(&mut self, _dynamic_aabbs: &[(RigidBodyHandle, Aabb)], _physics: &mut PhysicsWorld) {
        // No collider management needed
    }

    fn resolve_collision(&self, _body_handle: RigidBodyHandle, body_aabb: &Aabb) -> Vec3 {
        let cube = match &self.cube {
            Some(c) => c,
            None => return Vec3::ZERO,
        };

        // Convert body AABB to octree local space [0,1]
        let half_world = self.world_size / 2.0;
        let local_min = (body_aabb.min + Vec3::splat(half_world)) / self.world_size;
        let local_max = (body_aabb.max + Vec3::splat(half_world)) / self.world_size;

        // Clamp to [0,1] bounds
        let local_min = local_min.max(Vec3::ZERO);
        let local_max = local_max.min(Vec3::ONE);

        // Get region bounds for octree query
        let depth = 3; // Reasonable granularity for collision
        let bounds = match RegionBounds::from_local_aabb(local_min, local_max, depth) {
            Some(b) => b,
            None => return Vec3::ZERO,
        };

        // Query faces in region and compute penetration
        // Track maximum penetration per axis to avoid over-correction from multiple faces
        let mut max_correction = Vec3::ZERO;

        visit_faces_in_region(
            cube,
            &bounds,
            |face_info| {
                // Convert face to world space
                // face_info.position is the voxel's base position in [0,1] space
                // Face center is at voxel center + half size in normal direction
                let face_normal = Vec3::from(face_info.face.normal());
                let voxel_center_local = face_info.position + Vec3::splat(face_info.size * 0.5);
                let face_offset = face_normal * face_info.size * 0.5;
                let face_center_local = voxel_center_local + face_offset;

                // Face center in world space
                let face_center_world =
                    face_center_local * self.world_size - Vec3::splat(half_world);

                // Compute box-face penetration
                if let Some(penetration) = box_face_penetration(
                    body_aabb,
                    face_center_world,
                    face_normal,
                    face_info.size * self.world_size,
                ) {
                    let correction = penetration.normal * penetration.depth;
                    // Take maximum correction per axis (absolute value comparison)
                    // This prevents over-correction when overlapping multiple coplanar faces
                    if correction.x.abs() > max_correction.x.abs() {
                        max_correction.x = correction.x;
                    }
                    if correction.y.abs() > max_correction.y.abs() {
                        max_correction.y = correction.y;
                    }
                    if correction.z.abs() > max_correction.z.abs() {
                        max_correction.z = correction.z;
                    }
                }
            },
            self.border_materials,
        );

        max_correction
    }

    fn metrics(&self) -> ColliderMetrics {
        ColliderMetrics {
            strategy_name: "hybrid",
            init_time_ms: self.init_time_ms,
            update_time_us: 0.0,
            active_colliders: 0, // No Rapier colliders
            total_faces: 0,
        }
    }
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Penetration data from box-face intersection test
struct Penetration {
    normal: Vec3,
    depth: f32,
}

/// Test if a box penetrates a face and compute correction
///
/// A face is a thin axis-aligned quad. The normal points away from solid matter.
/// Penetration occurs when the box extends through the face into the solid side.
///
/// For example, a ground face at Y=0 with normal +Y means:
/// - Solid is below (Y < 0)
/// - Empty is above (Y > 0)
/// - A box penetrates if its bottom (min.y) goes below the face position
fn box_face_penetration(
    box_aabb: &Aabb,
    face_center: Vec3,
    face_normal: Vec3,
    face_size: f32,
) -> Option<Penetration> {
    // Determine which axis the face is aligned to
    let abs_normal = face_normal.abs();
    let face_axis = if abs_normal.x > abs_normal.y && abs_normal.x > abs_normal.z {
        0
    } else if abs_normal.y > abs_normal.z {
        1
    } else {
        2
    };

    let face_pos = match face_axis {
        0 => face_center.x,
        1 => face_center.y,
        _ => face_center.z,
    };

    // Get box bounds along face axis
    let (box_min, box_max) = match face_axis {
        0 => (box_aabb.min.x, box_aabb.max.x),
        1 => (box_aabb.min.y, box_aabb.max.y),
        _ => (box_aabb.min.z, box_aabb.max.z),
    };

    // Determine which direction the face is pointing
    let face_sign = match face_axis {
        0 => face_normal.x.signum(),
        1 => face_normal.y.signum(),
        _ => face_normal.z.signum(),
    };

    // Calculate penetration depth
    // Face normal points away from solid, so:
    // - If normal is positive, solid is on the negative side
    //   Penetration occurs when box_min < face_pos (box extends into solid)
    //   Depth = face_pos - box_min
    // - If normal is negative, solid is on the positive side
    //   Penetration occurs when box_max > face_pos
    //   Depth = box_max - face_pos
    let penetration_depth = if face_sign > 0.0 {
        // Face pointing positive: solid is on negative side
        // Box penetrates if its min goes below face position
        face_pos - box_min
    } else {
        // Face pointing negative: solid is on positive side
        // Box penetrates if its max goes above face position
        box_max - face_pos
    };

    if penetration_depth <= 0.0 {
        return None; // No penetration
    }

    // Check if box overlaps face in other axes
    let half_size = face_size / 2.0;
    let (axis_a, axis_b) = match face_axis {
        0 => (1, 2), // YZ plane
        1 => (0, 2), // XZ plane
        _ => (0, 1), // XY plane
    };

    let (face_a, face_b) = match face_axis {
        0 => (face_center.y, face_center.z),
        1 => (face_center.x, face_center.z),
        _ => (face_center.x, face_center.y),
    };

    let (box_min_a, box_max_a, box_min_b, box_max_b) = match (axis_a, axis_b) {
        (0, 1) => (
            box_aabb.min.x,
            box_aabb.max.x,
            box_aabb.min.y,
            box_aabb.max.y,
        ),
        (0, 2) => (
            box_aabb.min.x,
            box_aabb.max.x,
            box_aabb.min.z,
            box_aabb.max.z,
        ),
        (1, 2) => (
            box_aabb.min.y,
            box_aabb.max.y,
            box_aabb.min.z,
            box_aabb.max.z,
        ),
        _ => return None,
    };

    // Check overlap in both tangent axes
    if box_max_a < face_a - half_size || box_min_a > face_a + half_size {
        return None;
    }
    if box_max_b < face_b - half_size || box_min_b > face_b + half_size {
        return None;
    }

    Some(Penetration {
        normal: face_normal, // Push in direction of normal (away from solid)
        depth: penetration_depth,
    })
}

/// Count the number of shapes in a compound collider
pub fn count_compound_shapes(collider: &Collider) -> usize {
    if let Some(compound) = collider.shape().as_compound() {
        compound.shapes().len()
    } else {
        1
    }
}

/// Factory function to create a world collider from strategy name
///
/// # Arguments
/// * `strategy` - Strategy name: "monolithic", "chunked", or "hybrid"
/// * `chunk_size` - Chunk size for chunked strategy (ignored for others)
/// * `load_radius` - Load radius for chunked strategy (ignored for others)
///
/// # Returns
/// Boxed WorldCollider trait object
pub fn create_world_collider(
    strategy: &str,
    chunk_size: f32,
    load_radius: f32,
) -> Box<dyn WorldCollider> {
    match strategy {
        "chunked" => Box::new(ChunkedCollider::new(chunk_size, load_radius)),
        "hybrid" => Box::new(HybridOctreeCollider::new()),
        _ => Box::new(MonolithicCollider::new()), // Default to monolithic
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_monolithic_metrics() {
        let collider = MonolithicCollider::new();
        let metrics = collider.metrics();
        assert_eq!(metrics.strategy_name, "monolithic");
        assert_eq!(metrics.active_colliders, 0);
    }

    #[test]
    fn test_chunked_world_to_chunk() {
        let collider = ChunkedCollider::new(64.0, 128.0);
        let mut collider = collider;
        collider.world_size = 1024.0;

        // Center of world should be chunk (8, 8, 8) for 1024/64 = 16 chunks per axis
        let chunk = collider.world_to_chunk(Vec3::ZERO);
        assert_eq!(chunk, IVec3::new(8, 8, 8));

        // Corner at -512 should be chunk (0, 0, 0)
        let chunk = collider.world_to_chunk(Vec3::splat(-512.0));
        assert_eq!(chunk, IVec3::new(0, 0, 0));
    }

    #[test]
    fn test_chunked_chunks_in_aabb() {
        let collider = ChunkedCollider::new(64.0, 128.0);
        let mut collider = collider;
        collider.world_size = 1024.0;

        // Small AABB entirely within one chunk (chunk_size=64, so stay within -32..32)
        let aabb = Aabb::new(Vec3::splat(10.0), Vec3::splat(30.0));
        let chunks: Vec<_> = collider.chunks_in_aabb(&aabb).collect();
        assert_eq!(chunks.len(), 1);

        // AABB spanning multiple chunks
        let aabb = Aabb::new(Vec3::splat(-100.0), Vec3::splat(100.0));
        let chunks: Vec<_> = collider.chunks_in_aabb(&aabb).collect();
        assert!(chunks.len() > 1);
    }

    #[test]
    fn test_hybrid_metrics() {
        let collider = HybridOctreeCollider::new();
        let metrics = collider.metrics();
        assert_eq!(metrics.strategy_name, "hybrid");
        assert_eq!(metrics.active_colliders, 0);
    }

    #[test]
    fn test_create_world_collider() {
        let mono = create_world_collider("monolithic", 64.0, 128.0);
        assert_eq!(mono.metrics().strategy_name, "monolithic");

        let chunked = create_world_collider("chunked", 64.0, 128.0);
        assert_eq!(chunked.metrics().strategy_name, "chunked");

        let hybrid = create_world_collider("hybrid", 64.0, 128.0);
        assert_eq!(hybrid.metrics().strategy_name, "hybrid");

        // Unknown defaults to monolithic
        let unknown = create_world_collider("unknown", 64.0, 128.0);
        assert_eq!(unknown.metrics().strategy_name, "monolithic");
    }

    #[test]
    fn test_count_compound_shapes() {
        // Ball is not compound - returns 1
        let ball = ColliderBuilder::ball(1.0).build();
        assert_eq!(count_compound_shapes(&ball), 1);

        // Compound with one shape
        let shape = SharedShape::ball(1.0);
        let compound = ColliderBuilder::compound(vec![(Isometry::identity(), shape)]).build();
        assert_eq!(count_compound_shapes(&compound), 1);
    }

    #[test]
    fn test_box_face_penetration_upward_facing() {
        // Face at Y=0 pointing up (ground)
        // Normal +Y means solid is BELOW (Y < 0), empty is ABOVE (Y > 0)
        let face_center = Vec3::new(0.0, 0.0, 0.0);
        let face_normal = Vec3::Y; // Pointing up
        let face_size = 10.0;

        // Box with bottom at Y=0 (touching face) - no penetration
        let box_aabb = Aabb::new(Vec3::new(-0.5, 0.0, -0.5), Vec3::new(0.5, 1.0, 0.5));
        let penetration = box_face_penetration(&box_aabb, face_center, face_normal, face_size);
        // When box_min == face_pos, depth = face_pos - box_min = 0, which is not > 0
        assert!(penetration.is_none());

        // Box penetrating into ground by 0.2 units (box_min = -0.2, below face at Y=0)
        let box_aabb = Aabb::new(Vec3::new(-0.5, -0.2, -0.5), Vec3::new(0.5, 0.8, 0.5));
        let penetration = box_face_penetration(&box_aabb, face_center, face_normal, face_size);
        assert!(penetration.is_some());
        let pen = penetration.unwrap();
        // Normal should push box UP (in direction of face normal, away from solid below)
        assert_eq!(pen.normal, Vec3::Y);
        assert!((pen.depth - 0.2).abs() < 0.01, "Expected depth ~0.2, got {}", pen.depth);
    }

    #[test]
    fn test_box_face_penetration_no_overlap() {
        // Face at Y=0 pointing up
        let face_center = Vec3::new(0.0, 0.0, 0.0);
        let face_normal = Vec3::Y;
        let face_size = 2.0;

        // Box completely above the face (no penetration)
        let box_aabb = Aabb::new(Vec3::new(-0.5, 0.5, -0.5), Vec3::new(0.5, 1.5, 0.5));
        let penetration = box_face_penetration(&box_aabb, face_center, face_normal, face_size);
        assert!(penetration.is_none());
    }

    #[test]
    fn test_box_face_penetration_outside_face_extent() {
        // Face at Y=0 pointing up, but only 2x2 units
        let face_center = Vec3::new(0.0, 0.0, 0.0);
        let face_normal = Vec3::Y;
        let face_size = 2.0; // Face extends from -1 to 1 in XZ

        // Box penetrating in Y but outside face extent in X
        let box_aabb = Aabb::new(Vec3::new(5.0, -0.2, -0.5), Vec3::new(6.0, 0.8, 0.5));
        let penetration = box_face_penetration(&box_aabb, face_center, face_normal, face_size);
        assert!(penetration.is_none(), "Box outside face extent should not collide");
    }

    #[test]
    fn test_hybrid_resolve_collision_with_solid_cube() {
        use cube::Cube;
        use std::rc::Rc;

        // Create a solid cube (represents ground)
        let cube = Rc::new(Cube::Solid(1u8));

        // Create hybrid collider with world size 100, world centered at origin
        let mut collider = HybridOctreeCollider::new();
        collider.world_size = 100.0;
        collider.cube = Some(cube);
        // Border materials: solid at bottom (y=0,1), empty at top (y=2,3)
        collider.border_materials = [1, 1, 0, 0];

        // Test 1: Box above solid cube (no penetration expected)
        // Solid cube fills [0,1] in local space → [-50, 50] in world space
        // World top surface is at Y=50
        // Box at Y=55 (well above surface) should not penetrate
        let box_above = Aabb::new(Vec3::new(-5.0, 55.0, -5.0), Vec3::new(5.0, 65.0, 5.0));
        let correction_above = collider.resolve_collision(RigidBodyHandle::from_raw_parts(0, 0), &box_above);
        assert!(
            correction_above.length() < 0.1,
            "Box above surface should not get correction, got {:?}",
            correction_above
        );

        // Test 2: Box penetrating into solid cube
        // Box at Y=45 to Y=55 (10 units tall, center at Y=50)
        // This should penetrate 5 units into the top surface at Y=50
        let box_penetrating = Aabb::new(Vec3::new(-5.0, 45.0, -5.0), Vec3::new(5.0, 55.0, 5.0));
        let correction = collider.resolve_collision(RigidBodyHandle::from_raw_parts(0, 0), &box_penetrating);

        // Correction should push upward (positive Y)
        assert!(
            correction.y > 0.0,
            "Correction should push upward, got {:?}",
            correction
        );
    }

    #[test]
    fn test_hybrid_with_half_solid_world() {
        use cube::{Cube, IVec3Ext};
        use glam::IVec3;
        use std::rc::Rc;

        // Create a world where bottom half is solid, top half is empty
        // Octant index = x + y*2 + z*4
        // Y=0 (bottom): octants 0,1,4,5
        // Y=1 (top): octants 2,3,6,7
        let cube = Rc::new(Cube::tabulate(|octant| {
            let pos = IVec3::from_octant_index(octant);
            if pos.y == 0 {
                Cube::Solid(1) // Ground (bottom half)
            } else {
                Cube::Solid(0) // Air (top half)
            }
        }));

        let mut collider = HybridOctreeCollider::new();
        collider.world_size = 100.0;
        collider.cube = Some(cube);
        collider.border_materials = [1, 1, 0, 0];

        // Test: Box at world center (Y=0) should be at the ground surface
        // In world space: bottom is Y=-50, top is Y=50, ground surface is at Y=0
        // Box from Y=-5 to Y=5 should penetrate 5 units into ground
        let box_at_surface = Aabb::new(Vec3::new(-5.0, -5.0, -5.0), Vec3::new(5.0, 5.0, 5.0));
        let correction = collider.resolve_collision(RigidBodyHandle::from_raw_parts(0, 0), &box_at_surface);

        // Should get upward correction
        println!("Half-solid world correction: {:?}", correction);
        assert!(
            correction.y > 0.0,
            "Box penetrating ground should get upward correction, got {:?}",
            correction
        );
    }

    #[test]
    fn test_hybrid_query_depth_scaling() {
        use cube::{Cube, IVec3Ext};
        use glam::IVec3;
        use std::rc::Rc;

        // Test with a large world (like proto-gl's 8192 units)
        let cube = Rc::new(Cube::tabulate(|octant| {
            let pos = IVec3::from_octant_index(octant);
            if pos.y == 0 {
                Cube::Solid(1) // Ground
            } else {
                Cube::Solid(0) // Air
            }
        }));

        let mut collider = HybridOctreeCollider::new();
        collider.world_size = 8192.0; // Large world
        collider.cube = Some(cube);
        collider.border_materials = [32, 32, 0, 0];

        // Small object (10 units) penetrating the surface
        // Object from Y=-5 to Y=5 at world center
        let small_box = Aabb::new(Vec3::new(-5.0, -5.0, -5.0), Vec3::new(5.0, 5.0, 5.0));
        let correction = collider.resolve_collision(RigidBodyHandle::from_raw_parts(0, 0), &small_box);

        println!("Large world correction: {:?}", correction);
        // With proper depth scaling, we should still detect the collision
        assert!(
            correction.y > 0.0,
            "Should detect collision even in large world, got {:?}",
            correction
        );
    }

    #[test]
    fn test_face_info_position_debug() {
        use cube::{visit_faces, Cube, IVec3Ext};
        use glam::IVec3;
        use std::rc::Rc;

        // Create ground cube (bottom half solid)
        let cube = Rc::new(Cube::tabulate(|octant| {
            let pos = IVec3::from_octant_index(octant);
            if pos.y == 0 {
                Cube::Solid(1)
            } else {
                Cube::Solid(0)
            }
        }));

        println!("\n=== Face positions from octree ===");
        visit_faces(
            &cube,
            |face_info| {
                let normal = Vec3::from(face_info.face.normal());
                let voxel_center = face_info.position + Vec3::splat(face_info.size * 0.5);
                let face_center = voxel_center + normal * face_info.size * 0.5;
                println!(
                    "Face {:?}: voxel_pos={:?}, size={}, normal={:?}, face_center={:?}",
                    face_info.face, face_info.position, face_info.size, normal, face_center
                );
            },
            [1, 1, 0, 0],
        );
    }
}
