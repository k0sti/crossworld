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
/// * `ray_origin` - Ray origin in world space
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
    let entry_child_idx =
        calculate_entry_child(cube_center, ray_origin, ray_direction, entry_normal);

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
    ray_origin: Vec3,
    _ray_direction: Vec3,
    _entry_normal: Normal,
) -> usize {
    // Determine entry point on cube face
    let entry_point = ray_origin;

    // Calculate octant based on which side of center the entry point is
    let rel = entry_point - cube_center;
    let x_bit = if rel.x >= 0.0 { 1 } else { 0 };
    let y_bit = if rel.y >= 0.0 { 1 } else { 0 };
    let z_bit = if rel.z >= 0.0 { 1 } else { 0 };

    // Octant index: x*4 + y*2 + z
    (x_bit << 2) | (y_bit << 1) | z_bit
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
    let inv_dir = 1.0 / ray_direction;
    let t0 = (cube_min - ray_origin) * inv_dir;
    let t1 = (cube_max - ray_origin) * inv_dir;

    let t_max = t0.max(t1);

    // Return the furthest intersection (exit point)
    t_max.x.max(t_max.y).max(t_max.z).max(0.0)
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
}
