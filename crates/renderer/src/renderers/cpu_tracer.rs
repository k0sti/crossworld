use crate::renderer::*;
use crate::scenes::create_octa_cube;
use cube::Cube;
use image::{ImageBuffer, Rgb};
use std::rc::Rc;

/// Pure Rust CPU raytracer that renders to an image buffer
pub struct CpuTracer {
    cube: Rc<Cube<u8>>,
    image_buffer: Option<ImageBuffer<Rgb<u8>, Vec<u8>>>,
    /// If true, disable lighting and output pure material colors
    disable_lighting: bool,
}

impl CpuTracer {
    pub fn new() -> Self {
        // Use octa cube scene (2x2x2 octree with 6 solid voxels and 2 empty spaces)
        let cube = create_octa_cube();

        Self {
            cube,
            image_buffer: None,
            disable_lighting: false,
        }
    }

    #[allow(dead_code)]
    pub fn new_with_cube(cube: Rc<Cube<u8>>) -> Self {
        Self {
            cube,
            image_buffer: None,
            disable_lighting: false,
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

    /// Set whether to disable lighting (output pure material colors)
    ///
    /// When disabled, renders pure material palette colors without any lighting calculations.
    /// Useful for debugging material system and color verification tests.
    pub fn set_disable_lighting(&mut self, disable: bool) {
        self.disable_lighting = disable;
    }

    /// Get the current lighting disable state
    pub fn is_lighting_disabled(&self) -> bool {
        self.disable_lighting
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
        // Background color from constants
        let mut color = BACKGROUND_COLOR;

        // Call cube raycast directly (new API)
        // The raycast expects coordinates in [-1, 1]³ space (origin-centered)
        // The raycast handles rays from outside the cube and computes entry point automatically
        // The raycast treats T::default() (0 for i32) as empty
        let raycast_result = cube::raycast(
            &self.cube,
            ray.origin,
            ray.direction.normalize(),
            None, // No debug state
        );

        // DEBUG: Track raycast success rate
        #[cfg(test)]
        {
            static HIT_COUNT: std::sync::atomic::AtomicUsize =
                std::sync::atomic::AtomicUsize::new(0);
            static MISS_COUNT: std::sync::atomic::AtomicUsize =
                std::sync::atomic::AtomicUsize::new(0);
            static ONCE: std::sync::Once = std::sync::Once::new();

            match &raycast_result {
                Some(_) => {
                    HIT_COUNT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                }
                None => {
                    MISS_COUNT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                }
            }

            // Print stats after first pixel is rendered
            if HIT_COUNT.load(std::sync::atomic::Ordering::Relaxed)
                + MISS_COUNT.load(std::sync::atomic::Ordering::Relaxed)
                == 1
            {
                println!("\n=== Raycast Debug (first pixel) ===");
                println!("  Position: {:?}", ray.origin);
                println!("  Direction: {:?}", ray.direction.normalize());
                println!("  Hit: {}", raycast_result.is_some());
            }

            // Print final stats at end
            ONCE.call_once(|| {
                // Register cleanup to print stats when test ends
                let _ = std::panic::catch_unwind(|| {});
            });
        }

        match raycast_result {
            Some(cube_hit) => {
                // Successful octree raycast
                // The hit position is already in [-1, 1]³ world space (same as bounds)
                let world_hit_point = cube_hit.pos;

                // Calculate distance from ray origin
                let t = (world_hit_point - ray.origin).length();

                // Create HitInfo for lighting calculation
                let hit_info = HitInfo {
                    hit: true,
                    t,
                    point: world_hit_point,
                    normal: cube_hit.normal.as_vec3(),
                };

                // Get material color from voxel value
                let material_color = cube::material::get_material_color(cube_hit.value as i32);

                // Apply lighting (or output pure color if disabled)
                color = if self.disable_lighting {
                    material_color
                } else {
                    calculate_lighting(&hit_info, material_color)
                };
            }
            None => {
                // Miss - do nothing (color remains background)
            }
        }

        // Gamma correction
        color = color.powf(1.0 / 2.2);

        color
    }
}

impl Default for CpuTracer {
    fn default() -> Self {
        Self::new()
    }
}

impl Renderer for CpuTracer {
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
        "CPU Tracer"
    }

    fn supports_image_output(&self) -> bool {
        true
    }

    fn image_buffer(&self) -> Option<&ImageBuffer<Rgb<u8>, Vec<u8>>> {
        self.image_buffer.as_ref()
    }
}
