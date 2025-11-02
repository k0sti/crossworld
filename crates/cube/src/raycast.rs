use crate::neighbor_traversal::CubeCoord;
use crate::Cube;
use glam::Vec3;

/// Normal direction for ray entry into a cube face
/// Normals ordered as: -X, +X, -Y, +Y, -Z, +Z
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Normal {
    NegX = 0, // Left
    PosX = 1, // Right
    NegY = 2, // Down
    PosY = 3, // Up
    NegZ = 4, // Back
    PosZ = 5, // Front
}

impl Normal {
    /// Convert u8 index (0-5) to Normal
    pub fn from_u8(value: u8) -> Option<Self> {
        match value {
            0 => Some(Normal::NegX),
            1 => Some(Normal::PosX),
            2 => Some(Normal::NegY),
            3 => Some(Normal::PosY),
            4 => Some(Normal::NegZ),
            5 => Some(Normal::PosZ),
            _ => None,
        }
    }

    /// Get opposite normal (for exit face)
    pub fn opposite(self) -> Self {
        match self {
            Normal::NegX => Normal::PosX,
            Normal::PosX => Normal::NegX,
            Normal::NegY => Normal::PosY,
            Normal::PosY => Normal::NegY,
            Normal::NegZ => Normal::PosZ,
            Normal::PosZ => Normal::NegZ,
        }
    }

    /// Get the normal direction as a Vec3
    pub fn to_vec3(self) -> Vec3 {
        match self {
            Normal::NegX => Vec3::new(-1.0, 0.0, 0.0),
            Normal::PosX => Vec3::new(1.0, 0.0, 0.0),
            Normal::NegY => Vec3::new(0.0, -1.0, 0.0),
            Normal::PosY => Vec3::new(0.0, 1.0, 0.0),
            Normal::NegZ => Vec3::new(0.0, 0.0, -1.0),
            Normal::PosZ => Vec3::new(0.0, 0.0, 1.0),
        }
    }
}

/// Voxel data type for raycast results
pub type Voxel = i32;

/// Result of a raycast operation
#[derive(Debug, Clone)]
pub struct RaycastResult {
    /// The voxel that was hit (None if ray exited without collision)
    pub voxel: Option<Voxel>,
    /// The cube coordinate of the result (collision or exit point)
    pub coord: CubeCoord,
    /// Position vector inside the cube coordinate space
    pub position: Vec3,
}

/// Perform a raycast through an octree cube structure
///
/// # Arguments
/// * `cube` - The cube being cast through
/// * `cast_state` - Number of enters/exits (for tracking recursion depth)
/// * `cube_coord` - Current cube coordinate in octree space
/// * `entry_normal` - Entry face normal (0-5 for six directions)
/// * `ray_origin` - Ray origin in cube space
/// * `ray_direction` - Normalized ray direction
///
/// # Returns
/// * `RaycastResult` containing:
///   - `voxel`: Some(voxel_id) if collision, None if no collision
///   - `coord`: Exit cube coordinate if no collision, collision cube coord if collision
///   - `position`: Position inside the cube coordinate space
pub fn raycast(
    cube: &Cube<Voxel>,
    cast_state: u32,
    cube_coord: CubeCoord,
    entry_normal: u8,
    ray_origin: Vec3,
    ray_direction: Vec3,
) -> RaycastResult {
    let normal = Normal::from_u8(entry_normal).unwrap_or(Normal::NegX);

    // Check if this cube is solid (potential collision)
    match cube {
        Cube::Solid(voxel_id) if *voxel_id != 0 => {
            // Collision with solid voxel
            RaycastResult {
                voxel: Some(*voxel_id),
                coord: cube_coord,
                position: calculate_entry_position(cube_coord, normal, ray_origin, ray_direction),
            }
        }
        Cube::Solid(0) | Cube::Solid(_) => {
            // Empty voxel - ray passes through
            let (exit_coord, exit_pos) =
                calculate_exit(cube_coord, normal, ray_origin, ray_direction);
            RaycastResult {
                voxel: None,
                coord: exit_coord,
                position: exit_pos,
            }
        }
        Cube::Cubes(children) if cube_coord.depth > 0 => {
            // Subdivided cube - need to traverse children
            raycast_subdivided(
                children,
                cast_state,
                cube_coord,
                normal,
                ray_origin,
                ray_direction,
            )
        }
        _ => {
            // Other types (Planes, Slices) or depth 0 - treat as empty
            let (exit_coord, exit_pos) =
                calculate_exit(cube_coord, normal, ray_origin, ray_direction);
            RaycastResult {
                voxel: None,
                coord: exit_coord,
                position: exit_pos,
            }
        }
    }
}

/// Raycast through a subdivided cube
fn raycast_subdivided(
    children: &[std::rc::Rc<Cube<Voxel>>; 8],
    cast_state: u32,
    cube_coord: CubeCoord,
    entry_normal: Normal,
    ray_origin: Vec3,
    ray_direction: Vec3,
) -> RaycastResult {
    // Calculate which child octant the ray enters first
    let cube_size = 1.0 / (1u32 << cube_coord.depth) as f32;
    let cube_center = cube_coord_to_world_center(cube_coord, cube_size);

    // Determine entry child based on entry normal and ray position
    let entry_child_idx = calculate_entry_child(
        cube_center,
        cube_size,
        ray_origin,
        ray_direction,
        entry_normal,
    );

    // Traverse the child
    let child_coord = cube_coord.child(entry_child_idx);
    let child = &children[entry_child_idx];

    raycast(
        child,
        cast_state + 1,
        child_coord,
        entry_normal as u8,
        ray_origin,
        ray_direction,
    )
}

/// Calculate which child octant the ray enters
fn calculate_entry_child(
    cube_center: Vec3,
    cube_size: f32,
    ray_origin: Vec3,
    ray_direction: Vec3,
    entry_normal: Normal,
) -> usize {
    // Calculate the actual entry point on the cube's face
    let entry_point = calculate_face_entry_point(
        cube_center,
        cube_size,
        ray_origin,
        ray_direction,
        entry_normal,
    );

    // Calculate octant based on which side of center the entry point is
    let rel = entry_point - cube_center;
    let x_bit = if rel.x >= 0.0 { 1 } else { 0 };
    let y_bit = if rel.y >= 0.0 { 1 } else { 0 };
    let z_bit = if rel.z >= 0.0 { 1 } else { 0 };

    // Octant index: x*4 + y*2 + z
    (x_bit << 2) | (y_bit << 1) | z_bit
}

/// Calculate where the ray intersects the entry face of the cube
fn calculate_face_entry_point(
    cube_center: Vec3,
    cube_size: f32,
    ray_origin: Vec3,
    ray_direction: Vec3,
    entry_normal: Normal,
) -> Vec3 {
    const EPSILON: f32 = 1e-8;
    let half_size = cube_size * 0.5;

    // Get the plane position for the entry face
    let face_offset = match entry_normal {
        Normal::NegX => Vec3::new(-half_size, 0.0, 0.0),
        Normal::PosX => Vec3::new(half_size, 0.0, 0.0),
        Normal::NegY => Vec3::new(0.0, -half_size, 0.0),
        Normal::PosY => Vec3::new(0.0, half_size, 0.0),
        Normal::NegZ => Vec3::new(0.0, 0.0, -half_size),
        Normal::PosZ => Vec3::new(0.0, 0.0, half_size),
    };
    let face_point = cube_center + face_offset;

    // Calculate intersection with the plane
    let normal_vec = entry_normal.to_vec3();
    let denom = ray_direction.dot(normal_vec);

    if denom.abs() > EPSILON {
        let t = (face_point - ray_origin).dot(normal_vec) / denom;
        if t >= 0.0 {
            return ray_origin + ray_direction * t;
        }
    }

    // Fallback: if ray is parallel to face or other edge case, use ray origin
    ray_origin
}

/// Calculate the entry position on a cube face
fn calculate_entry_position(
    cube_coord: CubeCoord,
    _entry_normal: Normal,
    ray_origin: Vec3,
    _ray_direction: Vec3,
) -> Vec3 {
    let cube_size = 1.0 / (1u32 << cube_coord.depth) as f32;
    let cube_min = cube_coord_to_world_min(cube_coord, cube_size);

    // Calculate relative position within cube [0,1]³
    let rel = (ray_origin - cube_min) / cube_size;
    rel.clamp(Vec3::ZERO, Vec3::ONE)
}

/// Calculate exit coordinate and position when ray passes through empty space
fn calculate_exit(
    cube_coord: CubeCoord,
    _entry_normal: Normal,
    ray_origin: Vec3,
    ray_direction: Vec3,
) -> (CubeCoord, Vec3) {
    let cube_size = 1.0 / (1u32 << cube_coord.depth) as f32;
    let cube_min = cube_coord_to_world_min(cube_coord, cube_size);
    let cube_max = cube_min + Vec3::splat(cube_size);

    // Calculate exit face (opposite of entry, or determined by ray direction)
    let t_max = calculate_ray_exit_t(ray_origin, ray_direction, cube_min, cube_max);
    let exit_point = ray_origin + ray_direction * t_max;

    // Determine which face we exit from
    let exit_normal = determine_exit_normal(exit_point, cube_min, cube_max, cube_size);

    // Calculate neighbor coordinate
    let neighbor_offset = exit_normal.to_vec3() * cube_size;
    let neighbor_center = (cube_min + cube_max) * 0.5 + neighbor_offset * 0.5;

    // Convert back to cube coordinate
    let neighbor_coord = world_to_cube_coord(neighbor_center, cube_coord.depth);

    // Calculate position within neighbor cube
    let neighbor_min = cube_coord_to_world_min(neighbor_coord, cube_size);
    let rel_pos = (exit_point - neighbor_min) / cube_size;

    (neighbor_coord, rel_pos.clamp(Vec3::ZERO, Vec3::ONE))
}

/// Calculate t parameter for ray exit from AABB
fn calculate_ray_exit_t(
    ray_origin: Vec3,
    ray_direction: Vec3,
    cube_min: Vec3,
    cube_max: Vec3,
) -> f32 {
    const EPSILON: f32 = 1e-8;
    let mut t_max = f32::MIN;

    // Calculate intersection for each axis, handling near-zero directions
    for i in 0..3 {
        if ray_direction[i].abs() > EPSILON {
            let inv_dir = 1.0 / ray_direction[i];
            let t0 = (cube_min[i] - ray_origin[i]) * inv_dir;
            let t1 = (cube_max[i] - ray_origin[i]) * inv_dir;
            let t_far = t0.max(t1);
            t_max = t_max.max(t_far);
        }
    }

    // Return the furthest intersection (exit point), clamped to non-negative
    t_max.max(0.0)
}

/// Determine which face the ray exits from
fn determine_exit_normal(point: Vec3, cube_min: Vec3, cube_max: Vec3, epsilon: f32) -> Normal {
    let eps = epsilon * 0.01;

    if (point.x - cube_min.x).abs() < eps {
        Normal::NegX
    } else if (point.x - cube_max.x).abs() < eps {
        Normal::PosX
    } else if (point.y - cube_min.y).abs() < eps {
        Normal::NegY
    } else if (point.y - cube_max.y).abs() < eps {
        Normal::PosY
    } else if (point.z - cube_min.z).abs() < eps {
        Normal::NegZ
    } else {
        Normal::PosZ
    }
}

/// Convert cube coordinate to world space minimum corner
fn cube_coord_to_world_min(coord: CubeCoord, cube_size: f32) -> Vec3 {
    Vec3::new(
        coord.pos.x as f32 * cube_size,
        coord.pos.y as f32 * cube_size,
        coord.pos.z as f32 * cube_size,
    )
}

/// Convert cube coordinate to world space center
fn cube_coord_to_world_center(coord: CubeCoord, cube_size: f32) -> Vec3 {
    cube_coord_to_world_min(coord, cube_size) + Vec3::splat(cube_size * 0.5)
}

/// Convert world position to cube coordinate
fn world_to_cube_coord(world_pos: Vec3, depth: u32) -> CubeCoord {
    let scale = (1u32 << depth) as f32;
    let pos = (world_pos * scale).floor().as_ivec3();
    CubeCoord::new(pos, depth)
}

#[cfg(test)]
mod tests {
    use super::*;
    use glam::IVec3;
    use std::rc::Rc;

    // Helper function to create a subdivided cube with specific child configuration
    fn create_subdivided_cube(children_voxels: [Voxel; 8]) -> Cube<Voxel> {
        let children: [Rc<Cube<Voxel>>; 8] = [
            Rc::new(Cube::Solid(children_voxels[0])),
            Rc::new(Cube::Solid(children_voxels[1])),
            Rc::new(Cube::Solid(children_voxels[2])),
            Rc::new(Cube::Solid(children_voxels[3])),
            Rc::new(Cube::Solid(children_voxels[4])),
            Rc::new(Cube::Solid(children_voxels[5])),
            Rc::new(Cube::Solid(children_voxels[6])),
            Rc::new(Cube::Solid(children_voxels[7])),
        ];
        Cube::Cubes(Box::new(children))
    }

    #[test]
    fn test_normal_conversions() {
        assert_eq!(Normal::from_u8(0), Some(Normal::NegX));
        assert_eq!(Normal::from_u8(5), Some(Normal::PosZ));
        assert_eq!(Normal::from_u8(6), None);
    }

    #[test]
    fn test_normal_opposite() {
        assert_eq!(Normal::NegX.opposite(), Normal::PosX);
        assert_eq!(Normal::PosY.opposite(), Normal::NegY);
        assert_eq!(Normal::PosZ.opposite(), Normal::NegZ);
    }

    #[test]
    fn test_normal_to_vec3() {
        assert_eq!(Normal::NegX.to_vec3(), Vec3::new(-1.0, 0.0, 0.0));
        assert_eq!(Normal::PosX.to_vec3(), Vec3::new(1.0, 0.0, 0.0));
        assert_eq!(Normal::NegY.to_vec3(), Vec3::new(0.0, -1.0, 0.0));
        assert_eq!(Normal::PosY.to_vec3(), Vec3::new(0.0, 1.0, 0.0));
        assert_eq!(Normal::NegZ.to_vec3(), Vec3::new(0.0, 0.0, -1.0));
        assert_eq!(Normal::PosZ.to_vec3(), Vec3::new(0.0, 0.0, 1.0));
    }

    #[test]
    fn test_raycast_solid_collision() {
        let cube = Cube::Solid(42);
        let coord = CubeCoord::new(IVec3::ZERO, 1);
        let result = raycast(
            &cube,
            0,
            coord,
            0,
            Vec3::new(0.5, 0.5, 0.5),
            Vec3::new(1.0, 0.0, 0.0),
        );

        assert_eq!(result.voxel, Some(42));
        assert_eq!(result.coord.depth, 1);
    }

    #[test]
    fn test_raycast_empty_passthrough() {
        let cube = Cube::Solid(0);
        let coord = CubeCoord::new(IVec3::ZERO, 1);
        let result = raycast(
            &cube,
            0,
            coord,
            0,
            Vec3::new(0.5, 0.5, 0.5),
            Vec3::new(1.0, 0.0, 0.0),
        );

        assert_eq!(result.voxel, None);
    }

    // --- COMPLEX TESTS FOR EDGE CASES ---

    #[test]
    fn test_ray_exit_t_axial_positive_directions() {
        // Test ray traveling in positive X direction
        let origin = Vec3::new(0.25, 0.5, 0.5);
        let direction = Vec3::new(1.0, 0.0, 0.0);
        let cube_min = Vec3::new(0.0, 0.0, 0.0);
        let cube_max = Vec3::new(1.0, 1.0, 1.0);

        let t = calculate_ray_exit_t(origin, direction, cube_min, cube_max);
        let exit_point = origin + direction * t;

        // Should exit at x=1.0
        assert!(
            (exit_point.x - 1.0).abs() < 0.001,
            "Exit X should be 1.0, got {}",
            exit_point.x
        );
        assert!((exit_point.y - 0.5).abs() < 0.001, "Y should remain 0.5");
        assert!((exit_point.z - 0.5).abs() < 0.001, "Z should remain 0.5");
    }

    #[test]
    fn test_ray_exit_t_axial_negative_directions() {
        // Test ray traveling in negative Y direction
        let origin = Vec3::new(0.5, 0.75, 0.5);
        let direction = Vec3::new(0.0, -1.0, 0.0);
        let cube_min = Vec3::new(0.0, 0.0, 0.0);
        let cube_max = Vec3::new(1.0, 1.0, 1.0);

        let t = calculate_ray_exit_t(origin, direction, cube_min, cube_max);
        let exit_point = origin + direction * t;

        // Should exit at y=0.0
        assert!(
            (exit_point.y - 0.0).abs() < 0.001,
            "Exit Y should be 0.0, got {}",
            exit_point.y
        );
        assert!((exit_point.x - 0.5).abs() < 0.001, "X should remain 0.5");
    }

    #[test]
    fn test_ray_exit_t_diagonal_ray() {
        // Test diagonal ray through cube
        let origin = Vec3::new(0.25, 0.25, 0.25);
        let direction = Vec3::new(1.0, 1.0, 1.0).normalize();
        let cube_min = Vec3::new(0.0, 0.0, 0.0);
        let cube_max = Vec3::new(1.0, 1.0, 1.0);

        let t = calculate_ray_exit_t(origin, direction, cube_min, cube_max);
        let exit_point = origin + direction * t;

        // One of the coordinates should be at the boundary
        let at_boundary = (exit_point.x - 1.0).abs() < 0.001
            || (exit_point.y - 1.0).abs() < 0.001
            || (exit_point.z - 1.0).abs() < 0.001
            || (exit_point.x - 0.0).abs() < 0.001
            || (exit_point.y - 0.0).abs() < 0.001
            || (exit_point.z - 0.0).abs() < 0.001;

        assert!(
            at_boundary,
            "Exit point {:?} should be at cube boundary",
            exit_point
        );
    }

    #[test]
    fn test_ray_at_corner_entry() {
        // Ray entering exactly at corner
        let origin = Vec3::new(0.0, 0.0, 0.0);
        let direction = Vec3::new(1.0, 1.0, 1.0).normalize();
        let cube_min = Vec3::new(0.0, 0.0, 0.0);
        let cube_max = Vec3::new(1.0, 1.0, 1.0);

        let t = calculate_ray_exit_t(origin, direction, cube_min, cube_max);
        assert!(t >= 0.0, "T should be non-negative for corner entry");
    }

    #[test]
    fn test_ray_parallel_to_face() {
        // Ray parallel to XY plane, should exit through Z faces
        let origin = Vec3::new(0.5, 0.5, 0.25);
        let direction = Vec3::new(1.0, 0.0, 0.0);
        let cube_min = Vec3::new(0.0, 0.0, 0.0);
        let cube_max = Vec3::new(1.0, 1.0, 1.0);

        let t = calculate_ray_exit_t(origin, direction, cube_min, cube_max);
        let exit_point = origin + direction * t;

        // Should exit through X face
        assert!(
            (exit_point.x - 1.0).abs() < 0.001 || (exit_point.x - 0.0).abs() < 0.001,
            "Should exit through X face"
        );
    }

    #[test]
    fn test_determine_exit_normal_all_faces() {
        let cube_min = Vec3::new(0.0, 0.0, 0.0);
        let cube_max = Vec3::new(1.0, 1.0, 1.0);
        let epsilon = 1.0;

        // Test all six faces
        assert_eq!(
            determine_exit_normal(Vec3::new(0.0, 0.5, 0.5), cube_min, cube_max, epsilon),
            Normal::NegX,
            "Point on min X face"
        );

        assert_eq!(
            determine_exit_normal(Vec3::new(1.0, 0.5, 0.5), cube_min, cube_max, epsilon),
            Normal::PosX,
            "Point on max X face"
        );

        assert_eq!(
            determine_exit_normal(Vec3::new(0.5, 0.0, 0.5), cube_min, cube_max, epsilon),
            Normal::NegY,
            "Point on min Y face"
        );

        assert_eq!(
            determine_exit_normal(Vec3::new(0.5, 1.0, 0.5), cube_min, cube_max, epsilon),
            Normal::PosY,
            "Point on max Y face"
        );

        assert_eq!(
            determine_exit_normal(Vec3::new(0.5, 0.5, 0.0), cube_min, cube_max, epsilon),
            Normal::NegZ,
            "Point on min Z face"
        );

        assert_eq!(
            determine_exit_normal(Vec3::new(0.5, 0.5, 1.0), cube_min, cube_max, epsilon),
            Normal::PosZ,
            "Point on max Z face"
        );
    }

    #[test]
    fn test_determine_exit_normal_corner_ambiguity() {
        // Point at corner - multiple normals could be valid
        let cube_min = Vec3::new(0.0, 0.0, 0.0);
        let cube_max = Vec3::new(1.0, 1.0, 1.0);
        let epsilon = 1.0;

        // At corner (0, 0, 0) - any of NegX, NegY, NegZ could be valid
        let normal = determine_exit_normal(Vec3::new(0.0, 0.0, 0.0), cube_min, cube_max, epsilon);
        assert!(
            matches!(normal, Normal::NegX | Normal::NegY | Normal::NegZ),
            "Corner should give one of the adjacent faces, got {:?}",
            normal
        );
    }

    #[test]
    fn test_calculate_entry_child_all_octants() {
        let cube_center = Vec3::new(0.5, 0.5, 0.5);
        let cube_size = 1.0;
        let entry_normal = Normal::NegX;

        // Test all 8 octants
        // Octant encoding: x*4 + y*2 + z

        // Octant 0: x<center, y<center, z<center
        // Ray enters from negative X into the lower corner
        let ray_origin = Vec3::new(0.0, 0.25, 0.25);
        let ray_dir = Vec3::new(1.0, 0.0, 0.0);
        let child_idx =
            calculate_entry_child(cube_center, cube_size, ray_origin, ray_dir, entry_normal);
        assert_eq!(child_idx, 0, "Should be octant 0 (---) ");

        // Octant 3: x<center (always, since entering from NegX), y>=center, z>=center
        let ray_origin = Vec3::new(0.0, 0.75, 0.75);
        let child_idx =
            calculate_entry_child(cube_center, cube_size, ray_origin, ray_dir, entry_normal);
        // Entry at (0, 0.75, 0.75), rel = (-0.5, 0.25, 0.25), octant = 011 = 3
        assert_eq!(
            child_idx, 3,
            "Entry at (0, 0.75, 0.75) should be octant 3 (x-, y+, z+)"
        );

        // Octant 2: x<center, y<center, z>=center
        let ray_origin = Vec3::new(0.0, 0.25, 0.75);
        let child_idx =
            calculate_entry_child(cube_center, cube_size, ray_origin, ray_dir, entry_normal);
        // Entry at (0, 0.25, 0.75), rel = (-0.5, -0.25, 0.25), octant = 001 = 1
        assert_eq!(
            child_idx, 1,
            "Entry at (0, 0.25, 0.75) should be octant 1 (x-, y-, z+)"
        );
    }

    #[test]
    fn test_calculate_entry_child_boundary() {
        let cube_center = Vec3::new(0.5, 0.5, 0.5);
        let cube_size = 1.0;
        let entry_normal = Normal::NegX;

        // Ray entering exactly at center on the face (Y=0.5, Z=0.5)
        // Entry point will be at (0.0, 0.5, 0.5) on the NegX face
        // Relative to center: (-0.5, 0.0, 0.0)
        // Y and Z are at center (0), so they go positive due to >= comparison
        // X is negative, so octant should be: x=0, y=1, z=1 = octant 3
        let ray_origin = Vec3::new(0.0, 0.5, 0.5);
        let ray_dir = Vec3::new(1.0, 0.0, 0.0);
        let child_idx =
            calculate_entry_child(cube_center, cube_size, ray_origin, ray_dir, entry_normal);
        assert_eq!(
            child_idx, 3,
            "Entry at (0, 0.5, 0.5) should be octant 3 (x-, y+, z+)"
        );
    }

    #[test]
    fn test_world_cube_coord_conversions() {
        // Test round-trip conversion at depth 1
        let depth = 1;
        let cube_size = 1.0 / (1u32 << depth) as f32; // 0.5

        let original_coord = CubeCoord::new(IVec3::new(0, 1, 0), depth);
        let world_center = cube_coord_to_world_center(original_coord, cube_size);
        let converted_back = world_to_cube_coord(world_center, depth);

        assert_eq!(
            original_coord.pos, converted_back.pos,
            "Round-trip conversion should preserve position"
        );
    }

    #[test]
    fn test_world_cube_coord_at_boundaries() {
        let depth = 2;

        // Test point at various boundaries
        let coords = [
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(0.25, 0.25, 0.25),
            Vec3::new(0.5, 0.5, 0.5),
            Vec3::new(0.99, 0.99, 0.99),
        ];

        for pos in coords.iter() {
            let coord = world_to_cube_coord(*pos, depth);
            assert!(
                coord.pos.x >= 0 && coord.pos.x < (1 << depth),
                "X coordinate {} should be in valid range for depth {}",
                coord.pos.x,
                depth
            );
            assert!(
                coord.pos.y >= 0 && coord.pos.y < (1 << depth),
                "Y coordinate should be in valid range"
            );
            assert!(
                coord.pos.z >= 0 && coord.pos.z < (1 << depth),
                "Z coordinate should be in valid range"
            );
        }
    }

    #[test]
    fn test_raycast_through_subdivided_cube_hit_first_child() {
        // Create subdivided cube where first child (octant 0) is solid
        let mut children_voxels = [0; 8];
        children_voxels[0] = 42; // Only first octant is solid

        let cube = create_subdivided_cube(children_voxels);
        let coord = CubeCoord::new(IVec3::ZERO, 1);

        // Ray entering from negative X into octant 0
        let origin = Vec3::new(0.0, 0.1, 0.1); // Lower corner area
        let direction = Vec3::new(1.0, 0.0, 0.0);

        let result = raycast(&cube, 0, coord, Normal::NegX as u8, origin, direction);

        assert_eq!(result.voxel, Some(42), "Should hit solid voxel in octant 0");
    }

    #[test]
    fn test_raycast_through_subdivided_cube_hit_different_octants() {
        // Create subdivided cube where octant 3 is solid
        // Octant 3 = x<center, y>=center, z>=center (binary 011)
        let mut children_voxels = [0; 8];
        children_voxels[3] = 99; // Octant 3 (upper corner on negative X side)

        let cube = create_subdivided_cube(children_voxels);
        let coord = CubeCoord::new(IVec3::ZERO, 1);

        // Ray entering from negative X into octant 3
        // Entry point at (0, 0.6, 0.6) maps to octant 3
        let origin = Vec3::new(0.0, 0.6, 0.6);
        let direction = Vec3::new(1.0, 0.0, 0.0);

        let result = raycast(&cube, 0, coord, Normal::NegX as u8, origin, direction);

        // This test reveals whether the entry child calculation is correct
        // Ray should enter octant 3 and hit the solid voxel
        assert_eq!(
            result.voxel,
            Some(99),
            "Should hit solid voxel in octant 3, coord: {:?}",
            result.coord
        );
    }

    #[test]
    fn test_raycast_through_subdivided_all_empty() {
        // All children empty
        let children_voxels = [0; 8];
        let cube = create_subdivided_cube(children_voxels);
        let coord = CubeCoord::new(IVec3::ZERO, 1);

        let origin = Vec3::new(0.0, 0.5, 0.5);
        let direction = Vec3::new(1.0, 0.0, 0.0);

        let result = raycast(&cube, 0, coord, Normal::NegX as u8, origin, direction);

        assert_eq!(
            result.voxel, None,
            "Should pass through empty subdivided cube"
        );
    }

    #[test]
    fn test_raycast_negative_ray_direction() {
        // Test with negative ray directions
        let cube = Cube::Solid(0);
        let coord = CubeCoord::new(IVec3::ZERO, 1);

        // Ray going in negative X direction
        let origin = Vec3::new(0.5, 0.5, 0.5);
        let direction = Vec3::new(-1.0, 0.0, 0.0);

        let result = raycast(&cube, 0, coord, Normal::PosX as u8, origin, direction);

        assert_eq!(result.voxel, None, "Should handle negative direction");
    }

    #[test]
    fn test_raycast_diagonal_through_subdivided() {
        // Diagonal ray through subdivided cube
        let mut children_voxels = [0; 8];
        children_voxels[7] = 55; // Far corner

        let cube = create_subdivided_cube(children_voxels);
        let coord = CubeCoord::new(IVec3::ZERO, 1);

        // Diagonal ray from origin
        let origin = Vec3::new(0.0, 0.0, 0.0);
        let direction = Vec3::new(1.0, 1.0, 1.0).normalize();

        let result = raycast(&cube, 0, coord, Normal::NegX as u8, origin, direction);

        // The ray should potentially hit octant 7, but this depends on correct traversal
        // This is a complex case that tests the overall algorithm
        println!(
            "Diagonal ray result: voxel={:?}, coord={:?}",
            result.voxel, result.coord
        );
    }

    #[test]
    fn test_calculate_exit_different_entry_normals() {
        let coord = CubeCoord::new(IVec3::ZERO, 1);

        // Test exiting with different entry normals
        let test_cases = [
            (
                Normal::NegX,
                Vec3::new(0.0, 0.5, 0.5),
                Vec3::new(1.0, 0.0, 0.0),
            ),
            (
                Normal::PosX,
                Vec3::new(1.0, 0.5, 0.5),
                Vec3::new(-1.0, 0.0, 0.0),
            ),
            (
                Normal::NegY,
                Vec3::new(0.5, 0.0, 0.5),
                Vec3::new(0.0, 1.0, 0.0),
            ),
            (
                Normal::PosY,
                Vec3::new(0.5, 1.0, 0.5),
                Vec3::new(0.0, -1.0, 0.0),
            ),
            (
                Normal::NegZ,
                Vec3::new(0.5, 0.5, 0.0),
                Vec3::new(0.0, 0.0, 1.0),
            ),
            (
                Normal::PosZ,
                Vec3::new(0.5, 0.5, 1.0),
                Vec3::new(0.0, 0.0, -1.0),
            ),
        ];

        for (entry_normal, origin, direction) in test_cases.iter() {
            let (exit_coord, exit_pos) = calculate_exit(coord, *entry_normal, *origin, *direction);

            assert!(
                exit_pos.x >= 0.0 && exit_pos.x <= 1.0,
                "Exit position X should be normalized [0,1] for normal {:?}",
                entry_normal
            );
            assert!(
                exit_pos.y >= 0.0 && exit_pos.y <= 1.0,
                "Exit position Y should be normalized [0,1] for normal {:?}",
                entry_normal
            );
            assert!(
                exit_pos.z >= 0.0 && exit_pos.z <= 1.0,
                "Exit position Z should be normalized [0,1] for normal {:?}",
                entry_normal
            );

            println!(
                "Entry normal: {:?}, Exit coord: {:?}, Exit pos: {:?}",
                entry_normal, exit_coord, exit_pos
            );
        }
    }

    #[test]
    fn test_multiple_depth_levels() {
        // Test at different octree depths
        // Note: Using depth + 1 as voxel ID since 0 represents empty space
        for depth in 0..4 {
            let voxel_id = (depth + 1) as i32;
            let cube = Cube::Solid(voxel_id);
            let coord = CubeCoord::new(IVec3::ZERO, depth);

            let origin = Vec3::new(0.0, 0.0, 0.0);
            let direction = Vec3::new(1.0, 0.0, 0.0);

            let result = raycast(&cube, 0, coord, Normal::NegX as u8, origin, direction);

            assert_eq!(
                result.voxel,
                Some(voxel_id),
                "Should hit voxel at depth {}",
                depth
            );

            // Verify position is within valid range
            assert!(
                result.position.x >= 0.0 && result.position.x <= 1.0,
                "Position should be normalized at depth {}",
                depth
            );
        }
    }

    #[test]
    fn test_grazing_ray_along_edge() {
        // Ray that grazes along an edge of the cube
        let cube = Cube::Solid(0);
        let coord = CubeCoord::new(IVec3::ZERO, 1);

        // Ray along the X-axis edge where Y=0, Z=0
        let origin = Vec3::new(0.0, 0.0, 0.0);
        let direction = Vec3::new(1.0, 0.0, 0.0);

        let result = raycast(&cube, 0, coord, Normal::NegX as u8, origin, direction);

        // Should handle edge case without panic
        assert_eq!(result.voxel, None);
    }

    #[test]
    fn test_nearly_zero_direction_component() {
        // Ray with one component very close to zero (but not exactly zero)
        let cube = Cube::Solid(0);
        let coord = CubeCoord::new(IVec3::ZERO, 1);

        let origin = Vec3::new(0.25, 0.5, 0.5);
        let direction = Vec3::new(1.0, 0.0001, 0.0); // Nearly axis-aligned

        let result = raycast(&cube, 0, coord, Normal::NegX as u8, origin, direction);

        // Should not panic or produce NaN
        assert!(!result.position.is_nan(), "Position should not be NaN");
        assert_eq!(result.voxel, None);
    }
}
