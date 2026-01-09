//! Thumbnail generator for voxel models
//!
//! Provides CPU-based thumbnail rendering for voxel cubes

use crate::camera::Camera;
use crate::renderer::Renderer;
use crate::renderers::cpu_tracer::CpuTracer;
use cube::Cube;
use glam::Vec3;
use image::{ImageBuffer, Rgb};
use std::rc::Rc;

/// Default thumbnail size (square)
pub const DEFAULT_THUMBNAIL_SIZE: u32 = 64;

/// Generate a placeholder thumbnail (simple colored square)
pub fn generate_placeholder(size: u32, hue: u8) -> ImageBuffer<Rgb<u8>, Vec<u8>> {
    ImageBuffer::from_fn(size, size, |x, y| {
        // Create a simple pattern based on position
        let corner_dist = ((x as f32 - size as f32 / 2.0).powi(2)
            + (y as f32 - size as f32 / 2.0).powi(2))
        .sqrt();
        let max_dist = (size as f32 / 2.0) * 1.414;
        let brightness = (1.0 - corner_dist / max_dist).max(0.3);

        // Convert hue to RGB
        let h = (hue as f32 / 255.0) * 360.0;
        let s = 0.6;
        let v = 0.8 * brightness;

        let c = v * s;
        let x_val = c * (1.0 - ((h / 60.0) % 2.0 - 1.0).abs());
        let m = v - c;

        let (r, g, b) = if h < 60.0 {
            (c, x_val, 0.0)
        } else if h < 120.0 {
            (x_val, c, 0.0)
        } else if h < 180.0 {
            (0.0, c, x_val)
        } else if h < 240.0 {
            (0.0, x_val, c)
        } else if h < 300.0 {
            (x_val, 0.0, c)
        } else {
            (c, 0.0, x_val)
        };

        Rgb([
            ((r + m) * 255.0) as u8,
            ((g + m) * 255.0) as u8,
            ((b + m) * 255.0) as u8,
        ])
    })
}

/// Generate a thumbnail image for a voxel cube
///
/// # Arguments
/// * `cube` - The voxel cube to render
/// * `size` - Thumbnail size in pixels (width and height)
///
/// # Returns
/// An RGB image buffer containing the rendered thumbnail
pub fn generate_thumbnail(cube: Rc<Cube<u8>>, size: u32) -> ImageBuffer<Rgb<u8>, Vec<u8>> {
    let mut tracer = CpuTracer::new_with_cube(cube);

    // Setup camera to frame the cube nicely
    // Position camera at 45-degree angle to show multiple faces
    let distance = 2.5;
    let angle = std::f32::consts::PI / 4.0; // 45 degrees

    let position = Vec3::new(
        distance * angle.cos(),
        distance * 0.5,
        distance * angle.sin(),
    );
    let target = Vec3::ZERO;
    let forward = (target - position).normalize();
    let rotation = glam::Quat::from_rotation_arc(glam::Vec3::NEG_Z, forward);

    let camera = Camera {
        position,
        rotation,
        vfov: 45.0_f32.to_radians(),
        yaw: -angle,
        pitch: -15.0_f32.to_radians(),
        target_position: Some(target),
    };

    // Render thumbnail
    tracer.render_with_camera(size, size, &camera);

    // Extract image buffer
    tracer
        .image_buffer()
        .expect("Thumbnail render failed")
        .clone()
}

/// Generate a thumbnail with default size
pub fn generate_thumbnail_default(cube: Rc<Cube<u8>>) -> ImageBuffer<Rgb<u8>, Vec<u8>> {
    generate_thumbnail(cube, DEFAULT_THUMBNAIL_SIZE)
}

#[cfg(test)]
mod tests {
    use super::*;
    use cube::Cube;

    #[test]
    fn test_generate_thumbnail() {
        let cube = Rc::new(Cube::Solid(128u8)); // Simple solid cube
        let thumbnail = generate_thumbnail(cube, 32);

        assert_eq!(thumbnail.width(), 32);
        assert_eq!(thumbnail.height(), 32);
    }

    #[test]
    fn test_generate_thumbnail_default() {
        let cube = Rc::new(Cube::Solid(128u8));
        let thumbnail = generate_thumbnail_default(cube);

        assert_eq!(thumbnail.width(), DEFAULT_THUMBNAIL_SIZE);
        assert_eq!(thumbnail.height(), DEFAULT_THUMBNAIL_SIZE);
    }

    #[test]
    fn test_generate_placeholder() {
        let placeholder = generate_placeholder(32, 128);
        assert_eq!(placeholder.width(), 32);
        assert_eq!(placeholder.height(), 32);

        // Check that pixels are not all the same (has gradient)
        let pixel1 = placeholder.get_pixel(0, 0);
        let pixel2 = placeholder.get_pixel(16, 16);
        assert_ne!(pixel1, pixel2, "Placeholder should have gradient pattern");
    }
}
