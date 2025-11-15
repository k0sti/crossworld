use crate::gpu_tracer::GpuTracer;
use crate::renderer::*;
use cube::raycast::RaycastHit as CubeRaycastHit;
use cube::{Cube, parse_csm};
use image::{ImageBuffer, Rgb};
use std::rc::Rc;

/// Pure Rust CPU raytracer that renders to an image buffer
pub struct CpuCubeTracer {
    light_dir: glam::Vec3,
    background_color: glam::Vec3,
    image_buffer: Option<ImageBuffer<Rgb<u8>, Vec<u8>>>,
    gpu_tracer: GpuTracer,
}

impl CpuCubeTracer {
    pub fn new() -> Self {
        // Use a simple solid cube for testing
        let cube = Rc::new(Cube::Solid(1i32));

        Self {
            light_dir: glam::Vec3::new(0.5, 1.0, 0.3).normalize(),
            background_color: glam::Vec3::new(0.2, 0.3, 0.4),
            image_buffer: None,
            gpu_tracer: GpuTracer::new(cube),
        }
    }

    #[allow(dead_code)]
    pub fn new_with_cube(cube: Rc<Cube<i32>>) -> Self {
        Self {
            light_dir: glam::Vec3::new(0.5, 1.0, 0.3).normalize(),
            background_color: glam::Vec3::new(0.2, 0.3, 0.4),
            image_buffer: None,
            gpu_tracer: GpuTracer::new(cube),
        }
    }

    #[allow(dead_code)]
    pub fn new_with_cubscript(cubscript: &str) -> Self {
        let cube = Self::parse_cube(cubscript);
        Self {
            light_dir: glam::Vec3::new(0.5, 1.0, 0.3).normalize(),
            background_color: glam::Vec3::new(0.2, 0.3, 0.4),
            image_buffer: None,
            gpu_tracer: GpuTracer::new(cube),
        }
    }

    /// Parse cubscript code and return a Cube instance
    fn parse_cube(cubscript: &str) -> Rc<Cube<i32>> {
        match parse_csm(cubscript) {
            Ok(octree) => Rc::new(octree.root),
            Err(e) => {
                eprintln!("Failed to parse cubscript: {}", e);
                eprintln!("Using default solid cube");
                Rc::new(Cube::Solid(1))
            }
        }
    }

    /// Get a reference to the image buffer
    pub fn image_buffer(&self) -> Option<&ImageBuffer<Rgb<u8>, Vec<u8>>> {
        self.image_buffer.as_ref()
    }

    /// Get a mutable reference to the image buffer
    #[allow(dead_code)]
    pub fn image_buffer_mut(&mut self) -> Option<&mut ImageBuffer<Rgb<u8>, Vec<u8>>> {
        self.image_buffer.as_mut()
    }

    /// Save the rendered image to a file
    pub fn save_image(&self, path: &str) -> Result<(), image::ImageError> {
        if let Some(buffer) = &self.image_buffer {
            buffer.save(path)?;
        }
        Ok(())
    }

    /// Render a single pixel with time-based camera
    fn render_pixel(&self, x: u32, y: u32, width: u32, height: u32, time: f32) -> glam::Vec3 {
        // Normalized pixel coordinates (flip Y to match GL coordinate system)
        let uv = glam::Vec2::new(
            (x as f32 - 0.5 * width as f32) / height as f32,
            -((y as f32 - 0.5 * height as f32) / height as f32),
        );

        // Camera setup (same as GL version)
        let camera_pos = glam::Vec3::new(3.0 * (time * 0.3).cos(), 2.0, 3.0 * (time * 0.3).sin());
        let target = glam::Vec3::ZERO;
        let up = glam::Vec3::Y;

        // Create ray
        let ray = create_camera_ray(uv, camera_pos, target, up);

        self.render_ray(ray)
    }

    /// Render a single pixel with explicit camera configuration
    fn render_pixel_with_camera(
        &self,
        x: u32,
        y: u32,
        width: u32,
        height: u32,
        camera: &CameraConfig,
    ) -> glam::Vec3 {
        // Normalized pixel coordinates (flip Y to match GL coordinate system)
        let uv = glam::Vec2::new(
            (x as f32 - 0.5 * width as f32) / height as f32,
            -((y as f32 - 0.5 * height as f32) / height as f32),
        );

        // Create ray from camera
        let ray = create_camera_ray(uv, camera.position, camera.target(), camera.up());

        self.render_ray(ray)
    }

    /// Render a ray and return the color
    fn render_ray(&self, ray: Ray) -> glam::Vec3 {
        // Use GPU tracer's bounding box intersection
        let hit = self.gpu_tracer.raycast(ray.origin, ray.direction);

        // Background color
        let mut color = self.background_color;

        if hit.hit {
            // Get cube bounds for coordinate transformation
            let bounds = CubeBounds::default();

            // Transform hit point from world space to normalized [0,1]³ cube space
            let normalized_pos = (hit.point - bounds.min) / (bounds.max - bounds.min);

            // Move slightly inside the cube to avoid boundary issues
            const EPSILON: f32 = 1e-6;
            let normalized_pos = normalized_pos + ray.direction.normalize() * EPSILON;

            // Define empty voxel predicate (value == 0)
            let is_empty = |v: &i32| *v == 0;

            // Set maximum raycast depth
            let max_depth = 8;

            // Call cube raycast directly
            let cube = self.gpu_tracer.cube();
            let cube_hit: Option<CubeRaycastHit<i32>> = cube.raycast(
                normalized_pos,
                ray.direction.normalize(),
                max_depth,
                &is_empty,
            );

            if let Some(cube_hit) = cube_hit {
                // Transform hit position back to world space
                let world_hit_point = cube_hit.position * (bounds.max - bounds.min) + bounds.min;

                // Calculate distance from ray origin
                let t = (world_hit_point - ray.origin).length();

                // Create HitInfo for lighting calculation
                let hit_info = HitInfo {
                    hit: true,
                    t,
                    point: world_hit_point,
                    normal: cube_hit.normal,
                };

                // Note: cube_hit.value contains voxel data for future material systems
                color = calculate_lighting(&hit_info, ray.direction, self.light_dir);
            }
            // If cube raycast misses, use background color (no fallback to bounding box)
        }

        // Gamma correction
        color = color.powf(1.0 / 2.2);

        color
    }
}

impl Default for CpuCubeTracer {
    fn default() -> Self {
        Self::new()
    }
}

impl Renderer for CpuCubeTracer {
    fn render(&mut self, width: u32, height: u32, time: f32) {
        // Create or resize image buffer
        let buffer = ImageBuffer::from_fn(width, height, |x, y| {
            let color = self.render_pixel(x, y, width, height, time);

            // Convert to RGB8
            let r = (color.x.clamp(0.0, 1.0) * 255.0) as u8;
            let g = (color.y.clamp(0.0, 1.0) * 255.0) as u8;
            let b = (color.z.clamp(0.0, 1.0) * 255.0) as u8;

            Rgb([r, g, b])
        });

        self.image_buffer = Some(buffer);
    }

    fn render_with_camera(&mut self, width: u32, height: u32, camera: &CameraConfig) {
        // Create or resize image buffer
        let buffer = ImageBuffer::from_fn(width, height, |x, y| {
            let color = self.render_pixel_with_camera(x, y, width, height, camera);

            // Convert to RGB8
            let r = (color.x.clamp(0.0, 1.0) * 255.0) as u8;
            let g = (color.y.clamp(0.0, 1.0) * 255.0) as u8;
            let b = (color.z.clamp(0.0, 1.0) * 255.0) as u8;

            Rgb([r, g, b])
        });

        self.image_buffer = Some(buffer);
    }

    fn name(&self) -> &str {
        "CpuCubeTracer"
    }
}
