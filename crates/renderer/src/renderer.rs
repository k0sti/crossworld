/// Camera configuration for rendering
#[derive(Debug, Clone, Copy)]
pub struct CameraConfig {
    /// Camera position in world space
    pub position: glam::Vec3,
    /// Camera rotation (orientation)
    pub rotation: glam::Quat,
    /// Vertical field of view in radians
    #[allow(dead_code)]
    pub vfov: f32,
}

impl Default for CameraConfig {
    fn default() -> Self {
        Self {
            position: glam::Vec3::new(3.0, 2.0, 3.0),
            rotation: glam::Quat::IDENTITY,
            vfov: 60.0_f32.to_radians(),
        }
    }
}

impl CameraConfig {
    /// Create camera configuration with position looking at target
    pub fn look_at(position: glam::Vec3, target: glam::Vec3, _up: glam::Vec3) -> Self {
        let forward = (target - position).normalize();
        let rotation = glam::Quat::from_rotation_arc(glam::Vec3::NEG_Z, forward);

        Self {
            position,
            rotation,
            vfov: 60.0_f32.to_radians(),
        }
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
        // Apply yaw rotation (around world Y axis)
        let yaw_rotation = glam::Quat::from_axis_angle(glam::Vec3::Y, yaw_delta);

        // Apply pitch rotation (around local right axis)
        let pitch_rotation = glam::Quat::from_axis_angle(self.right(), pitch_delta);

        self.rotation = yaw_rotation * pitch_rotation * self.rotation;
        self.rotation = self.rotation.normalize();
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

/// Calculate lighting for a hit point
pub fn calculate_lighting(
    hit: &HitInfo,
    ray_direction: glam::Vec3,
    light_dir: glam::Vec3,
) -> glam::Vec3 {
    // Diffuse lighting
    let diffuse = hit.normal.dot(light_dir).max(0.0);

    // Ambient - increased for brighter appearance
    let ambient = 0.5;

    // Base cube color with variation based on normal
    let mut base_color = glam::Vec3::new(0.8, 0.4, 0.2);
    base_color = base_color.lerp(glam::Vec3::new(1.0, 0.6, 0.3), hit.normal.x.abs());
    base_color = base_color.lerp(glam::Vec3::new(0.6, 0.8, 0.4), hit.normal.y.abs());
    base_color = base_color.lerp(glam::Vec3::new(0.4, 0.5, 0.9), hit.normal.z.abs());

    // Combine lighting - increased diffuse contribution
    let mut color = base_color * (ambient + diffuse * 1.2);

    // Fresnel effect
    let fresnel = (1.0 - (-ray_direction).dot(hit.normal).abs()).powf(3.0);
    color += glam::Vec3::splat(0.1 * fresnel);

    color
}
