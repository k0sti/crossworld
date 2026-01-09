//! Raycast system for voxel editing
//!
//! Handles mouse-to-world ray calculation and cube raycast integration
//! for determining where the cursor should be placed and what voxels
//! are being targeted.

use cube::{raycast_with_options, Axis, Cube, CubeCoord, Hit, RaycastOptions};
use glam::{IVec3, Vec2, Vec3};
use renderer::Camera;

/// Result of a raycast operation in the editor
#[derive(Debug, Clone)]
pub struct EditorHit {
    /// World position of the hit point
    pub world_pos: Vec3,
    /// Voxel coordinate of the hit voxel
    pub voxel_coord: IVec3,
    /// Normal of the face that was hit
    pub normal: Axis,
    /// Material/color value at the hit voxel
    pub value: u8,
    /// Octree coordinate of the hit
    pub cube_coord: CubeCoord,
}

impl EditorHit {
    /// Create from cube raycast hit
    ///
    /// # Arguments
    /// * `hit` - The hit result from cube raycast
    /// * `cube_position` - World position of the cube's center
    /// * `cube_scale` - Scale factor of the cube in world space
    pub fn from_cube_hit(hit: Hit<u8>, cube_position: Vec3, cube_scale: f32) -> Self {
        // Convert hit position from cube's [-1,1]³ space to world space
        let world_pos = cube_position + hit.pos * cube_scale;

        // Voxel coordinate comes directly from the cube coordinate
        let voxel_coord = hit.coord.pos;

        Self {
            world_pos,
            voxel_coord,
            normal: hit.normal,
            value: hit.value,
            cube_coord: hit.coord,
        }
    }

    /// Get the face normal as a Vec3
    pub fn normal_vec3(&self) -> Vec3 {
        self.normal.to_vec3()
    }

    /// Get the position adjacent to hit face (for voxel placement)
    pub fn adjacent_voxel_coord(&self) -> IVec3 {
        self.voxel_coord + self.normal.to_ivec3()
    }

    /// Compute voxel coordinate at a specific depth from hit position
    ///
    /// Uses the precise hit position to calculate the voxel coordinate
    /// at any depth level, regardless of the hit's actual depth.
    ///
    /// # Arguments
    /// * `depth` - Target depth level (e.g., 4 for 16x16x16)
    /// * `cube_position` - World position of the cube's center
    /// * `cube_scale` - Scale factor of the cube in world space
    pub fn voxel_at_depth(&self, depth: u32, cube_position: Vec3, cube_scale: f32) -> IVec3 {
        // Convert world position back to cube's [-1,1]³ space
        let cube_pos = (self.world_pos - cube_position) / cube_scale;

        // Convert to [0, 2^depth] range and floor to get voxel index
        // cube space: [-1, 1] → voxel space: [0, 2^depth]
        let scale = (1 << depth) as f32 / 2.0;
        IVec3::new(
            ((cube_pos.x + 1.0) * scale).floor() as i32,
            ((cube_pos.y + 1.0) * scale).floor() as i32,
            ((cube_pos.z + 1.0) * scale).floor() as i32,
        )
    }

    /// Compute placement position at a specific depth (adjacent to hit face)
    ///
    /// # Arguments
    /// * `depth` - Target depth level for placement
    /// * `cube_position` - World position of the cube's center
    /// * `cube_scale` - Scale factor of the cube in world space
    pub fn placement_at_depth(&self, depth: u32, cube_position: Vec3, cube_scale: f32) -> IVec3 {
        // Get the hit voxel at target depth
        let hit_voxel = self.voxel_at_depth(depth, cube_position, cube_scale);
        // Add normal to get adjacent position
        hit_voxel + self.normal.to_ivec3()
    }
}

/// A ray in 3D space defined by origin and direction
#[derive(Debug, Clone, Copy)]
pub struct Ray {
    /// Origin point of the ray
    pub origin: Vec3,
    /// Normalized direction vector
    pub direction: Vec3,
}

impl Ray {
    /// Create a new ray with the given origin and direction
    pub fn new(origin: Vec3, direction: Vec3) -> Self {
        Self {
            origin,
            direction: direction.normalize(),
        }
    }

    /// Get a point along the ray at parameter t
    pub fn at(&self, t: f32) -> Vec3 {
        self.origin + self.direction * t
    }
}

/// Calculate a ray from mouse position through the camera
///
/// # Arguments
/// * `mouse_pos` - Mouse position in screen coordinates (pixels, origin top-left)
/// * `screen_size` - Screen dimensions (width, height) in pixels
/// * `camera` - Camera to use for ray calculation
///
/// # Returns
/// A ray in world space originating from the camera position
pub fn mouse_to_ray(mouse_pos: Vec2, screen_size: Vec2, camera: &Camera) -> Ray {
    // Convert mouse position to normalized device coordinates (NDC)
    // NDC ranges from -1 to 1, with (0,0) at center
    // Screen coordinates have origin at top-left
    let ndc_x = (2.0 * mouse_pos.x / screen_size.x) - 1.0;
    let ndc_y = 1.0 - (2.0 * mouse_pos.y / screen_size.y); // Flip Y

    // Calculate aspect ratio
    let aspect = screen_size.x / screen_size.y;

    // Calculate the half-height and half-width of the near plane
    // using the camera's vertical field of view
    let half_height = (camera.vfov / 2.0).tan();
    let half_width = half_height * aspect;

    // Get camera basis vectors
    let forward = camera.forward();
    let right = camera.right();
    let up = camera.up();

    // Calculate ray direction in world space
    // The ray goes from the camera through the point on the virtual near plane
    let direction = forward + right * (ndc_x * half_width) + up * (ndc_y * half_height);

    Ray::new(camera.position, direction)
}

/// Perform a raycast against a cube
///
/// # Arguments
/// * `ray` - The ray to cast
/// * `cube` - The cube to raycast against
/// * `cube_position` - World position of the cube's center
/// * `cube_scale` - Scale factor of the cube in world space
/// * `max_depth` - Optional maximum depth to traverse (for LOD)
///
/// # Returns
/// The hit result if the ray intersects the cube
pub fn raycast_cube(
    ray: &Ray,
    cube: &Cube<u8>,
    cube_position: Vec3,
    cube_scale: f32,
    max_depth: Option<u32>,
) -> Option<EditorHit> {
    // Transform ray from world space to cube's [-1,1]³ space
    let cube_origin = (ray.origin - cube_position) / cube_scale;
    let cube_direction = ray.direction; // Direction doesn't need scaling

    // Set up raycast options
    let options = RaycastOptions { max_depth };

    // Perform raycast
    let hit = raycast_with_options(cube, cube_origin, cube_direction, None, &options)?;

    Some(EditorHit::from_cube_hit(hit, cube_position, cube_scale))
}

/// Perform a raycast from mouse position against a cube
///
/// This is a convenience function that combines mouse_to_ray and raycast_cube.
///
/// # Arguments
/// * `mouse_pos` - Mouse position in screen coordinates
/// * `screen_size` - Screen dimensions (width, height)
/// * `camera` - Camera for ray calculation
/// * `cube` - The cube to raycast against
/// * `cube_position` - World position of the cube's center
/// * `cube_scale` - Scale factor of the cube in world space
/// * `max_depth` - Optional maximum depth to traverse
pub fn raycast_from_mouse(
    mouse_pos: Vec2,
    screen_size: Vec2,
    camera: &Camera,
    cube: &Cube<u8>,
    cube_position: Vec3,
    cube_scale: f32,
    max_depth: Option<u32>,
) -> Option<EditorHit> {
    let ray = mouse_to_ray(mouse_pos, screen_size, camera);
    raycast_cube(&ray, cube, cube_position, cube_scale, max_depth)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::rc::Rc;

    #[test]
    fn test_ray_creation() {
        let ray = Ray::new(Vec3::ZERO, Vec3::new(1.0, 2.0, 3.0));
        assert_eq!(ray.origin, Vec3::ZERO);
        assert!((ray.direction.length() - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_ray_at() {
        let ray = Ray::new(Vec3::ZERO, Vec3::X);
        assert_eq!(ray.at(0.0), Vec3::ZERO);
        assert_eq!(ray.at(5.0), Vec3::new(5.0, 0.0, 0.0));
    }

    #[test]
    fn test_mouse_to_ray_center() {
        let camera = Camera::look_at(Vec3::new(0.0, 0.0, 5.0), Vec3::ZERO, Vec3::Y);
        let screen_size = Vec2::new(800.0, 600.0);
        let mouse_pos = Vec2::new(400.0, 300.0); // Center of screen

        let ray = mouse_to_ray(mouse_pos, screen_size, &camera);

        // Ray should originate from camera position
        assert_eq!(ray.origin, camera.position);

        // Ray direction should be forward (towards target)
        let expected_dir = (Vec3::ZERO - camera.position).normalize();
        let dot = ray.direction.dot(expected_dir);
        assert!(dot > 0.99, "Ray direction should be mostly forward, got dot={}", dot);
    }

    #[test]
    fn test_raycast_solid_cube() {
        let cube = Cube::Solid(42u8);
        let ray = Ray::new(Vec3::new(-5.0, 0.0, 0.0), Vec3::X);

        let hit = raycast_cube(&ray, &cube, Vec3::ZERO, 1.0, None);

        assert!(hit.is_some());
        let hit = hit.unwrap();
        assert_eq!(hit.value, 42);
        assert_eq!(hit.normal, Axis::NegX); // Hit from negative X side
    }

    #[test]
    fn test_raycast_empty_cube() {
        let cube = Cube::Solid(0u8); // Empty cube
        let ray = Ray::new(Vec3::new(-5.0, 0.0, 0.0), Vec3::X);

        let hit = raycast_cube(&ray, &cube, Vec3::ZERO, 1.0, None);

        assert!(hit.is_none());
    }

    #[test]
    fn test_raycast_miss() {
        let cube = Cube::Solid(42u8);
        // Ray pointing away from cube
        let ray = Ray::new(Vec3::new(-5.0, 0.0, 0.0), Vec3::NEG_X);

        let hit = raycast_cube(&ray, &cube, Vec3::ZERO, 1.0, None);

        assert!(hit.is_none());
    }

    #[test]
    fn test_raycast_with_scale() {
        let cube = Cube::Solid(42u8);
        let ray = Ray::new(Vec3::new(-5.0, 0.0, 0.0), Vec3::X);

        // With scale 2.0, cube extends from -2 to 2 in world space
        let hit = raycast_cube(&ray, &cube, Vec3::ZERO, 2.0, None);

        assert!(hit.is_some());
        let hit = hit.unwrap();
        // Hit should be at x = -2.0 (edge of scaled cube)
        assert!((hit.world_pos.x - (-2.0)).abs() < 0.01);
    }

    #[test]
    fn test_raycast_with_offset() {
        let cube = Cube::Solid(42u8);
        let cube_position = Vec3::new(10.0, 0.0, 0.0);
        let ray = Ray::new(Vec3::new(5.0, 0.0, 0.0), Vec3::X);

        // Cube at x=10, ray starts at x=5, should hit at x=9
        let hit = raycast_cube(&ray, &cube, cube_position, 1.0, None);

        assert!(hit.is_some());
        let hit = hit.unwrap();
        assert!((hit.world_pos.x - 9.0).abs() < 0.01);
    }

    #[test]
    fn test_editor_hit_adjacent_coord() {
        let cube = Cube::Solid(42u8);
        let ray = Ray::new(Vec3::new(-5.0, 0.0, 0.0), Vec3::X);

        let hit = raycast_cube(&ray, &cube, Vec3::ZERO, 1.0, None).unwrap();

        // When hit from -X side, adjacent voxel is one step in -X direction
        let adjacent = hit.adjacent_voxel_coord();
        assert_eq!(adjacent, hit.voxel_coord + IVec3::NEG_X);
    }

    #[test]
    fn test_raycast_nested_cube() {
        // Create a nested cube structure
        let inner = Cube::cubes([
            Rc::new(Cube::Solid(1u8)),
            Rc::new(Cube::Solid(2u8)),
            Rc::new(Cube::Solid(3u8)),
            Rc::new(Cube::Solid(4u8)),
            Rc::new(Cube::Solid(5u8)),
            Rc::new(Cube::Solid(6u8)),
            Rc::new(Cube::Solid(7u8)),
            Rc::new(Cube::Solid(8u8)),
        ]);

        let ray = Ray::new(Vec3::new(-5.0, 0.0, 0.0), Vec3::X);

        // Without max_depth, should hit deepest level
        let hit = raycast_cube(&ray, &inner, Vec3::ZERO, 1.0, None).unwrap();
        assert!(hit.cube_coord.depth >= 1);

        // With max_depth=0, should stop at root level
        let hit_limited = raycast_cube(&ray, &inner, Vec3::ZERO, 1.0, Some(0)).unwrap();
        assert_eq!(hit_limited.cube_coord.depth, 0);
    }
}
