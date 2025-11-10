use crate::renderer::*;
use image::{ImageBuffer, Rgb};

/// Pure Rust CPU raytracer that renders to an image buffer
pub struct CpuCubeTracer {
    cube_bounds: CubeBounds,
    light_dir: glam::Vec3,
    background_color: glam::Vec3,
    image_buffer: Option<ImageBuffer<Rgb<u8>, Vec<u8>>>,
}

impl CpuCubeTracer {
    pub fn new() -> Self {
        Self {
            cube_bounds: CubeBounds::default(),
            light_dir: glam::Vec3::new(0.5, 1.0, 0.3).normalize(),
            background_color: glam::Vec3::new(0.2, 0.3, 0.4),
            image_buffer: None,
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

    /// Render a single pixel
    fn render_pixel(&self, x: u32, y: u32, width: u32, height: u32, time: f32) -> glam::Vec3 {
        // Normalized pixel coordinates (flip Y to match GL coordinate system)
        let uv = glam::Vec2::new(
            (x as f32 - 0.5 * width as f32) / height as f32,
            -((y as f32 - 0.5 * height as f32) / height as f32),
        );

        // Camera setup (same as GL version)
        let camera_pos = glam::Vec3::new(
            3.0 * (time * 0.3).cos(),
            2.0,
            3.0 * (time * 0.3).sin(),
        );
        let target = glam::Vec3::ZERO;
        let up = glam::Vec3::Y;

        // Create ray
        let ray = create_camera_ray(uv, camera_pos, target, up);

        // Intersect with cube
        let hit = intersect_box(ray, self.cube_bounds.min, self.cube_bounds.max);

        // Background color
        let mut color = self.background_color;

        if hit.hit {
            color = calculate_lighting(&hit, ray.direction, self.light_dir);
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

    fn name(&self) -> &str {
        "CpuCubeTracer"
    }
}
