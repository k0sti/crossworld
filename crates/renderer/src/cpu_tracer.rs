use crate::renderer::*;
use crate::scenes::create_octa_cube;
use cube::{Cube, parse_csm};
use image::{ImageBuffer, Rgb};
use std::rc::Rc;

/// Pure Rust CPU raytracer that renders to an image buffer
pub struct CpuCubeTracer {
    cube: Rc<Cube<i32>>,
    bounds: CubeBounds,
    image_buffer: Option<ImageBuffer<Rgb<u8>, Vec<u8>>>,
}

impl CpuCubeTracer {
    pub fn new() -> Self {
        // Use octa cube scene (2x2x2 octree with 6 solid voxels and 2 empty spaces)
        let cube = create_octa_cube();

        Self {
            cube,
            bounds: CubeBounds::default(),
            image_buffer: None,
        }
    }

    #[allow(dead_code)]
    pub fn new_with_cube(cube: Rc<Cube<i32>>) -> Self {
        Self {
            cube,
            bounds: CubeBounds::default(),
            image_buffer: None,
        }
    }

    #[allow(dead_code)]
    pub fn new_with_cubscript(cubscript: &str) -> Self {
        let cube = Self::parse_cube(cubscript);
        Self {
            cube,
            bounds: CubeBounds::default(),
            image_buffer: None,
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
        // Intersect with bounding box
        let hit_info = intersect_box(ray, self.bounds.min, self.bounds.max);

        // Background color from constants
        let mut color = BACKGROUND_COLOR;

        if hit_info.hit {
            // Get cube bounds for coordinate transformation
            let bounds = self.bounds;

            // CRITICAL FIX: Advance ray slightly into the cube before transforming to normalized space
            // When we hit the bounding box surface, we're exactly ON the boundary.
            // Starting a DDA raycast from a boundary position can cause traversal issues.
            // Advance the ray by a small epsilon to ensure we start INSIDE the cube.
            // Cube is 2 units wide, so 0.01 is 0.5% of the cube size - small but meaningful
            const SURFACE_EPSILON: f32 = 0.01;
            let advanced_hit_point = hit_info.point + ray.direction * SURFACE_EPSILON;

            // Transform advanced hit point from world space to normalized [0,1]³ cube space
            let mut normalized_pos = (advanced_hit_point - bounds.min) / (bounds.max - bounds.min);

            // Clamp to valid range to ensure we stay within the cube
            const EPSILON: f32 = 0.001;
            normalized_pos =
                normalized_pos.clamp(glam::Vec3::splat(EPSILON), glam::Vec3::splat(1.0 - EPSILON));

            // Define empty voxel predicate (value == 0)
            let is_empty = |v: &i32| *v == 0;

            // Set maximum raycast depth (octa cube is depth 1, not 8!)
            let max_depth = 1;

            // Call cube raycast directly
            let raycast_result = self.cube.raycast(
                normalized_pos,
                ray.direction.normalize(),
                max_depth,
                &is_empty,
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
                    println!("  Position: {:?}", normalized_pos);
                    println!("  Direction: {:?}", ray.direction.normalize());
                    println!(
                        "  Result: {:?}",
                        raycast_result.as_ref().map(|o| o.is_some())
                    );
                }

                // Print final stats at end
                ONCE.call_once(|| {
                    // Register cleanup to print stats when test ends
                    let _ = std::panic::catch_unwind(|| {});
                });
            }

            match raycast_result {
                Some(cube_hit) => {
                    // Successful octree raycast - use detailed voxel information
                    // Transform hit position back to world space
                    let world_hit_point =
                        cube_hit.hit_pos * (bounds.max - bounds.min) + bounds.min;

                    // Calculate distance from ray origin
                    let t = (world_hit_point - ray.origin).length();

                    // Create HitInfo for lighting calculation
                    let hit_info = HitInfo {
                        hit: true,
                        t,
                        point: world_hit_point,
                        normal: cube_hit.normal(),
                    };

                    // Get material color from voxel value
                    let material_color = cube::material::get_material_color(cube_hit.value);

                    // Apply lighting
                    color = calculate_lighting(&hit_info, material_color);
                }
                None => {
                    // Miss - do nothing (color remains background)
                }
            }
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
