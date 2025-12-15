use glam::Vec3;

/// Lighting constants for standardized rendering across all tracers
///
/// These constants define a simple directional lighting model with ambient
/// and diffuse components. All tracers (CPU, GL, GPU) use these same values
/// to ensure consistent visual output.
/// Directional light direction (normalized)
///
/// Light comes from upper-right-front direction.
/// Pre-normalized: normalize(0.5, 1.0, 0.3) = (0.431934, 0.863868, 0.259161)
pub const LIGHT_DIR: Vec3 = Vec3::new(0.431934, 0.863868, 0.259161);

/// Ambient lighting term (0.0-1.0)
///
/// 30% ambient illumination ensures all surfaces are visible even when
/// facing away from the light.
pub const AMBIENT: f32 = 0.3;

/// Diffuse lighting strength multiplier
///
/// Applied to the diffuse term before adding to ambient.
pub const DIFFUSE_STRENGTH: f32 = 0.7;

/// Background color for empty space
///
/// Bluish-gray color rendered when rays miss all voxels.
pub const BACKGROUND_COLOR: Vec3 = Vec3::new(0.4, 0.5, 0.6);

/// Entity trait for objects in 3D space
///
/// Represents any object with position and rotation in the scene
pub trait Entity {
    /// Get the entity's position in world space
    fn position(&self) -> glam::Vec3;

    /// Get the entity's rotation as a quaternion
    fn rotation(&self) -> glam::Quat;

    /// Set the entity's position
    fn set_position(&mut self, position: glam::Vec3);

    /// Set the entity's rotation
    fn set_rotation(&mut self, rotation: glam::Quat);

    /// Create a camera looking at this entity
    fn make_camera(&self, offset: glam::Vec3, up: glam::Vec3) -> CameraConfig {
        let camera_position = self.position() + offset;
        CameraConfig::look_at(camera_position, self.position(), up)
    }
}

/// A simple cube object that can be positioned and rotated in space
#[derive(Debug, Clone, Copy)]
pub struct CubeObject {
    /// Position in world space
    pub position: glam::Vec3,
    /// Rotation as quaternion
    pub rotation: glam::Quat,
}

impl CubeObject {
    /// Create a new cube object at the origin with no rotation
    pub fn new() -> Self {
        Self {
            position: glam::Vec3::ZERO,
            rotation: glam::Quat::IDENTITY,
        }
    }

    /// Create a new cube object at a specific position
    pub fn at_position(position: glam::Vec3) -> Self {
        Self {
            position,
            rotation: glam::Quat::IDENTITY,
        }
    }

    /// Create a new cube object with position and rotation
    pub fn with_transform(position: glam::Vec3, rotation: glam::Quat) -> Self {
        Self { position, rotation }
    }
}

impl Default for CubeObject {
    fn default() -> Self {
        Self::new()
    }
}

impl Entity for CubeObject {
    fn position(&self) -> glam::Vec3 {
        self.position
    }

    fn rotation(&self) -> glam::Quat {
        self.rotation
    }

    fn set_position(&mut self, position: glam::Vec3) {
        self.position = position;
    }

    fn set_rotation(&mut self, rotation: glam::Quat) {
        self.rotation = rotation;
    }
}

/// Camera configuration for rendering
#[derive(Debug, Clone)]
pub struct CameraConfig {
    /// Camera position in world space
    pub position: glam::Vec3,
    /// Camera rotation (orientation)
    pub rotation: glam::Quat,
    /// Vertical field of view in radians
    pub vfov: f32,
    /// Pitch angle in radians (rotation around X axis)
    pub pitch: f32,
    /// Yaw angle in radians (rotation around Y axis)
    pub yaw: f32,
    /// Optional target position for look-at cameras
    pub target_position: Option<glam::Vec3>,
}

impl Default for CameraConfig {
    fn default() -> Self {
        let position = glam::Vec3::new(3.0, 2.0, 3.0);
        let target = glam::Vec3::ZERO;
        let forward = (target - position).normalize();

        // Calculate pitch and yaw from forward vector
        let yaw = forward.z.atan2(forward.x);
        let pitch = forward.y.asin();

        let rotation = glam::Quat::from_rotation_arc(glam::Vec3::NEG_Z, forward);

        Self {
            position,
            rotation,
            vfov: 2.0 * 0.5_f32.atan(), // ~53.13 degrees - matches raytracer's implicit FOV
            pitch,
            yaw,
            target_position: Some(target),
        }
    }
}

impl Copy for CameraConfig {}

impl CameraConfig {
    /// Create camera configuration with position looking at target
    pub fn look_at(position: glam::Vec3, target: glam::Vec3, up: glam::Vec3) -> Self {
        // Build camera basis the same way as the raytracer does
        let forward = (target - position).normalize();
        let right = forward.cross(up).normalize();
        let cam_up = right.cross(forward);

        // Build rotation matrix from basis vectors and convert to quaternion
        // In camera space: right=+X, up=+Y, forward=-Z (OpenGL convention)
        let rotation_matrix = glam::Mat3::from_cols(right, cam_up, -forward);
        let rotation = glam::Quat::from_mat3(&rotation_matrix);

        // Calculate pitch and yaw from forward vector
        let yaw = forward.z.atan2(forward.x);
        let pitch = forward.y.asin();

        Self {
            position,
            rotation,
            vfov: 2.0 * 0.5_f32.atan(), // ~53.13 degrees - matches raytracer's implicit FOV
            pitch,
            yaw,
            target_position: Some(target),
        }
    }

    /// Set the camera to look at a specific target position
    pub fn set_look_at(&mut self, target: glam::Vec3) {
        // Build camera basis the same way as the raytracer does
        let up = glam::Vec3::Y; // Use world up
        let forward = (target - self.position).normalize();
        let right = forward.cross(up).normalize();
        let cam_up = right.cross(forward);

        // Build rotation matrix from basis vectors and convert to quaternion
        let rotation_matrix = glam::Mat3::from_cols(right, cam_up, -forward);
        self.rotation = glam::Quat::from_mat3(&rotation_matrix);

        // Update pitch and yaw
        self.yaw = forward.z.atan2(forward.x);
        self.pitch = forward.y.asin();
        self.target_position = Some(target);
    }

    /// Create camera from pitch and yaw angles
    pub fn from_pitch_yaw(position: glam::Vec3, pitch: f32, yaw: f32) -> Self {
        let rotation = glam::Quat::from_euler(glam::EulerRot::YXZ, yaw, pitch, 0.0);

        Self {
            position,
            rotation,
            vfov: 2.0 * 0.5_f32.atan(), // ~53.13 degrees - matches raytracer's implicit FOV
            pitch,
            yaw,
            target_position: None,
        }
    }

    /// Update camera rotation from pitch and yaw
    pub fn update_from_pitch_yaw(&mut self) {
        self.rotation = glam::Quat::from_euler(glam::EulerRot::YXZ, self.yaw, self.pitch, 0.0);
        // Clear target when manually rotating
        self.target_position = None;
    }

    /// Get the forward direction vector
    pub fn forward(&self) -> glam::Vec3 {
        self.rotation * glam::Vec3::NEG_Z
    }

    /// Get the right direction vector
    pub fn right(&self) -> glam::Vec3 {
        self.rotation * glam::Vec3::X
    }

    /// Get the up direction vector
    pub fn up(&self) -> glam::Vec3 {
        self.rotation * glam::Vec3::Y
    }

    /// Get the target point the camera is looking at (1 unit forward)
    pub fn target(&self) -> glam::Vec3 {
        self.position + self.forward()
    }

    /// Rotate camera by yaw (around Y axis) and pitch (around local X axis)
    /// This rotates the camera in place (first-person style)
    #[allow(dead_code)]
    pub fn rotate(&mut self, yaw_delta: f32, pitch_delta: f32) {
        // Update pitch and yaw
        self.yaw += yaw_delta;
        self.pitch += pitch_delta;

        // Clamp pitch to prevent gimbal lock
        const MAX_PITCH: f32 = 89.0 * std::f32::consts::PI / 180.0;
        self.pitch = self.pitch.clamp(-MAX_PITCH, MAX_PITCH);

        // Update rotation from pitch and yaw
        self.update_from_pitch_yaw();
    }

    /// Orbit camera around a target point
    /// yaw_delta: rotation around world Y-axis (horizontal mouse movement)
    /// pitch_delta: rotation around camera's local right axis (vertical mouse movement)
    pub fn orbit(&mut self, target: glam::Vec3, yaw_delta: f32, pitch_delta: f32) {
        // Calculate vector from target to camera
        let mut offset = self.position - target;
        let distance = offset.length();

        // Step 1: Apply pitch rotation around camera's CURRENT local right axis (vertical angle)
        // This must happen FIRST, using the original camera orientation
        if pitch_delta.abs() > 0.0001 {
            // Get the current right vector (perpendicular to both up and forward)
            let forward = -offset.normalize();
            let right = forward.cross(glam::Vec3::Y).normalize();

            // Only apply pitch if right vector is valid (not looking straight up/down)
            if right.length_squared() > 0.0001 {
                let pitch_rotation = glam::Quat::from_axis_angle(right, pitch_delta);
                offset = pitch_rotation * offset;

                // Clamp to prevent flipping over the poles
                let new_y = offset.y;
                let xz_length = (offset.x * offset.x + offset.z * offset.z).sqrt();
                let angle_from_horizontal = new_y.atan2(xz_length);

                // Clamp angle to [-85°, 85°] to prevent gimbal lock
                const MAX_ANGLE: f32 = 85.0 * std::f32::consts::PI / 180.0;
                if angle_from_horizontal.abs() > MAX_ANGLE {
                    let clamped_angle = angle_from_horizontal.clamp(-MAX_ANGLE, MAX_ANGLE);
                    let new_y = distance * clamped_angle.sin();
                    let new_xz = distance * clamped_angle.cos();
                    let xz_ratio = new_xz / xz_length;
                    offset = glam::Vec3::new(offset.x * xz_ratio, new_y, offset.z * xz_ratio);
                }
            }
        }

        // Step 2: Apply yaw rotation around world Y-axis (horizontal orbit)
        // This happens AFTER pitch, so horizontal orbit is always around world Y
        if yaw_delta.abs() > 0.0001 {
            let yaw_rotation = glam::Quat::from_axis_angle(glam::Vec3::Y, yaw_delta);
            offset = yaw_rotation * offset;
        }

        // Ensure we maintain the same distance
        offset = offset.normalize() * distance;

        // Update position
        self.position = target + offset;

        // Update rotation to look at target
        let forward = (target - self.position).normalize();
        self.rotation = glam::Quat::from_rotation_arc(glam::Vec3::NEG_Z, forward);

        // Update pitch and yaw from the new rotation
        self.yaw = forward.z.atan2(forward.x);
        self.pitch = forward.y.asin();
        self.target_position = Some(target);
    }

    /// Move camera relative to its current orientation
    #[allow(dead_code)]
    pub fn translate_local(&mut self, offset: glam::Vec3) {
        self.position += self.right() * offset.x;
        self.position += self.up() * offset.y;
        self.position += self.forward() * offset.z;
    }

    /// Zoom by moving camera forward/backward along view direction
    /// Note: For orbit cameras, consider zooming toward/away from target instead
    #[allow(dead_code)]
    pub fn zoom(&mut self, delta: f32) {
        self.position += self.forward() * delta;
    }
}

/// Common renderer trait for cube raytracers
pub trait Renderer {
    /// Render a single frame at the given time
    fn render(&mut self, width: u32, height: u32, time: f32);

    /// Render with explicit camera configuration
    fn render_with_camera(&mut self, width: u32, height: u32, camera: &CameraConfig);

    /// Get the name of the renderer
    fn name(&self) -> &str;
}

/// Cube bounds for raytracing
#[derive(Debug, Clone, Copy)]
pub struct CubeBounds {
    pub min: glam::Vec3,
    pub max: glam::Vec3,
}

impl Default for CubeBounds {
    fn default() -> Self {
        Self {
            min: glam::Vec3::new(-1.0, -1.0, -1.0),
            max: glam::Vec3::new(1.0, 1.0, 1.0),
        }
    }
}

/// Ray for raytracing
#[derive(Debug, Clone, Copy)]
pub struct Ray {
    pub origin: glam::Vec3,
    pub direction: glam::Vec3,
}

/// Hit information from ray-cube intersection
#[derive(Debug, Clone, Copy)]
pub struct HitInfo {
    pub hit: bool,
    pub t: f32,
    pub point: glam::Vec3,
    pub normal: glam::Vec3,
}

impl Default for HitInfo {
    fn default() -> Self {
        Self {
            hit: false,
            t: f32::MAX,
            point: glam::Vec3::ZERO,
            normal: glam::Vec3::ZERO,
        }
    }
}

/// Ray-box intersection algorithm
pub fn intersect_box(ray: Ray, box_min: glam::Vec3, box_max: glam::Vec3) -> HitInfo {
    let mut hit_info = HitInfo::default();

    let inv_dir = 1.0 / ray.direction;
    let t_min = (box_min - ray.origin) * inv_dir;
    let t_max = (box_max - ray.origin) * inv_dir;

    let t1 = t_min.min(t_max);
    let t2 = t_min.max(t_max);

    let t_near = t1.x.max(t1.y).max(t1.z);
    let t_far = t2.x.min(t2.y).min(t2.z);

    if t_near > t_far || t_far < 0.0 {
        return hit_info;
    }

    hit_info.hit = true;
    hit_info.t = if t_near > 0.0 { t_near } else { t_far };
    hit_info.point = ray.origin + ray.direction * hit_info.t;

    // Calculate normal
    let center = (box_min + box_max) * 0.5;
    let local_point = hit_info.point - center;
    let size = (box_max - box_min) * 0.5;
    let d = (local_point / size).abs();

    let max_component = d.x.max(d.y).max(d.z);
    if (max_component - d.x).abs() < 0.0001 {
        hit_info.normal = glam::Vec3::new(local_point.x.signum(), 0.0, 0.0);
    } else if (max_component - d.y).abs() < 0.0001 {
        hit_info.normal = glam::Vec3::new(0.0, local_point.y.signum(), 0.0);
    } else {
        hit_info.normal = glam::Vec3::new(0.0, 0.0, local_point.z.signum());
    }

    hit_info
}

/// Create a camera ray for a given pixel coordinate
pub fn create_camera_ray(
    uv: glam::Vec2,
    camera_pos: glam::Vec3,
    target: glam::Vec3,
    up: glam::Vec3,
) -> Ray {
    let forward = (target - camera_pos).normalize();
    let right = forward.cross(up).normalize();
    let cam_up = right.cross(forward);

    Ray {
        origin: camera_pos,
        direction: (forward + uv.x * right + uv.y * cam_up).normalize(),
    }
}

/// Calculate lighting for a hit point with material color
///
/// Applies standardized lighting model: `materialColor * (AMBIENT + diffuse * DIFFUSE_STRENGTH)`
///
/// # Arguments
///
/// * `hit` - Hit information including position and normal
/// * `material_color` - Base material color from palette
///
/// # Returns
///
/// Final lit color (before gamma correction)
pub fn calculate_lighting(hit: &HitInfo, material_color: Vec3) -> Vec3 {
    // Diffuse lighting using Lambert's cosine law
    let diffuse = hit.normal.dot(LIGHT_DIR).max(0.0);

    // Combine lighting: material color * (ambient + diffuse)
    material_color * (AMBIENT + diffuse * DIFFUSE_STRENGTH)
}

/// Calculate unlit material color (for debug mode)
///
/// Returns the pure material color without any lighting applied.
///
/// # Arguments
///
/// * `material_color` - Base material color from palette
///
/// # Returns
///
/// Unmodified material color
pub fn calculate_lighting_unlit(material_color: Vec3) -> Vec3 {
    material_color
}
