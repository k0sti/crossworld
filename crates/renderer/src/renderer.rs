/// Common renderer trait for cube raytracers
pub trait Renderer {
    /// Render a single frame at the given time
    fn render(&mut self, width: u32, height: u32, time: f32);

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

    // Ambient
    let ambient = 0.2;

    // Base cube color with variation based on normal
    let mut base_color = glam::Vec3::new(0.8, 0.4, 0.2);
    base_color = base_color.lerp(glam::Vec3::new(1.0, 0.6, 0.3), hit.normal.x.abs());
    base_color = base_color.lerp(glam::Vec3::new(0.6, 0.8, 0.4), hit.normal.y.abs());
    base_color = base_color.lerp(glam::Vec3::new(0.4, 0.5, 0.9), hit.normal.z.abs());

    // Combine lighting
    let mut color = base_color * (ambient + diffuse * 0.8);

    // Fresnel effect
    let fresnel = (1.0 - (-ray_direction).dot(hit.normal).abs()).powf(3.0);
    color += glam::Vec3::splat(0.1 * fresnel);

    color
}
