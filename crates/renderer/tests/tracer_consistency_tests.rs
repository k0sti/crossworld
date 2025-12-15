//! Tracer Consistency Tests
//!
//! Verifies that all tracer implementations (CPU, GL, GPU) produce consistent
//! rendering output when given the same scene and camera configuration.
//!
//! These tests focus on cross-tracer comparison, not raycast algorithm correctness.
//! For raycast logic tests, see crates/cube/tests/raycast_table_tests.rs

use glam::Vec3;
use renderer::{CameraConfig, CpuTracer, Renderer};

/// Color tolerance for cross-tracer comparisons (±10 RGB units)
/// Higher than single-tracer tests due to rounding differences
const CROSS_TRACER_COLOR_TOLERANCE: u8 = 10;

/// Helper: Calculate pixel difference percentage between two image buffers
fn calculate_pixel_difference(
    img1: &image::ImageBuffer<image::Rgb<u8>, Vec<u8>>,
    img2: &image::ImageBuffer<image::Rgb<u8>, Vec<u8>>,
) -> f32 {
    assert_eq!(
        img1.dimensions(),
        img2.dimensions(),
        "Images must have same dimensions"
    );

    let (width, height) = img1.dimensions();
    let total_pixels = (width * height) as usize;
    let mut different_pixels = 0;

    for y in 0..height {
        for x in 0..width {
            let p1 = img1.get_pixel(x, y);
            let p2 = img2.get_pixel(x, y);

            // Consider pixel different if any channel differs by more than tolerance
            if (p1[0] as i16 - p2[0] as i16).abs() > CROSS_TRACER_COLOR_TOLERANCE as i16
                || (p1[1] as i16 - p2[1] as i16).abs() > CROSS_TRACER_COLOR_TOLERANCE as i16
                || (p1[2] as i16 - p2[2] as i16).abs() > CROSS_TRACER_COLOR_TOLERANCE as i16
            {
                different_pixels += 1;
            }
        }
    }

    (different_pixels as f32 / total_pixels as f32) * 100.0
}

/// Helper: Get pixel RGB at specific coordinates
fn get_pixel_at(
    buffer: &image::ImageBuffer<image::Rgb<u8>, Vec<u8>>,
    x: u32,
    y: u32,
) -> (u8, u8, u8) {
    let pixel = buffer.get_pixel(x, y);
    (pixel[0], pixel[1], pixel[2])
}

#[test]
fn test_cpu_tracer_basic_rendering() {
    // Basic smoke test: CPU tracer can render without crashing
    let camera = CameraConfig::look_at(Vec3::new(-2.0, 1.5, -2.0), Vec3::ZERO, Vec3::Y);

    let mut tracer = CpuTracer::new();
    tracer.render_with_camera(256, 256, &camera);

    let output = tracer.image_buffer().expect("Should have image buffer");

    // Verify output dimensions
    assert_eq!(output.dimensions(), (256, 256), "Output should be 256x256");

    // Verify output has non-background pixels
    let background = (102u8, 128u8, 153u8); // From materials.rs
    let mut has_content = false;

    for y in 0..256 {
        for x in 0..256 {
            let pixel = get_pixel_at(output, x, y);
            if pixel != background {
                has_content = true;
                break;
            }
        }
        if has_content {
            break;
        }
    }

    assert!(
        has_content,
        "Rendered image should contain non-background pixels"
    );
}

#[test]
fn test_cpu_tracer_renders_expected_colors() {
    // Test that CPU tracer renders octa-cube with correct material colors
    let camera = CameraConfig::look_at(Vec3::new(-2.0, 1.5, -2.0), Vec3::ZERO, Vec3::Y);

    let mut tracer = CpuTracer::new();
    tracer.render_with_camera(256, 256, &camera);

    let output = tracer.image_buffer().expect("Should have image buffer");

    // Count pixels of each expected color (with tolerance)
    let mut color_counts = std::collections::HashMap::new();

    for y in 0..256 {
        for x in 0..256 {
            let (r, g, b) = get_pixel_at(output, x, y);

            // Categorize by dominant color
            let category = if r > 200 && g < 50 && b < 50 {
                "red"
            } else if r < 50 && g > 200 && b < 50 {
                "green"
            } else if r < 50 && g < 50 && b > 200 {
                "blue"
            } else if r > 200 && g > 200 && b < 50 {
                "yellow"
            } else if r > 200 && g < 50 && b > 200 {
                "magenta"
            } else {
                "other"
            };

            *color_counts.entry(category).or_insert(0) += 1;
        }
    }

    println!("Color distribution: {:?}", color_counts);

    // At least some material colors should be present
    let material_pixel_count = color_counts.keys().filter(|&&k| k != "other").count();
    assert!(
        material_pixel_count >= 2,
        "Should render at least 2 different material colors, found {}",
        material_pixel_count
    );
}

#[test]
fn test_cpu_tracer_consistency_across_renders() {
    // Test that CPU tracer produces identical output for same input
    let camera = CameraConfig::look_at(Vec3::new(-2.0, 1.5, -2.0), Vec3::ZERO, Vec3::Y);

    let mut tracer1 = CpuTracer::new();
    tracer1.render_with_camera(128, 128, &camera);
    let output1 = tracer1.image_buffer().expect("Should have image buffer");

    let mut tracer2 = CpuTracer::new();
    tracer2.render_with_camera(128, 128, &camera);
    let output2 = tracer2.image_buffer().expect("Should have image buffer");

    // Outputs should be identical (deterministic rendering)
    let diff_percent = calculate_pixel_difference(output1, output2);
    assert_eq!(
        diff_percent, 0.0,
        "CPU tracer should produce identical output for same input, found {}% difference",
        diff_percent
    );
}

#[test]
fn test_cpu_tracer_different_viewing_angles() {
    // Test that different camera angles produce different outputs
    let camera_front = CameraConfig::look_at(Vec3::new(0.0, 0.0, -2.0), Vec3::ZERO, Vec3::Y);

    let camera_side = CameraConfig::look_at(Vec3::new(-2.0, 0.0, 0.0), Vec3::ZERO, Vec3::Y);

    let mut tracer1 = CpuTracer::new();
    tracer1.render_with_camera(128, 128, &camera_front);
    let output_front = tracer1.image_buffer().expect("Should have image buffer");

    let mut tracer2 = CpuTracer::new();
    tracer2.render_with_camera(128, 128, &camera_side);
    let output_side = tracer2.image_buffer().expect("Should have image buffer");

    // Outputs should be different
    let diff_percent = calculate_pixel_difference(output_front, output_side);
    assert!(
        diff_percent > 10.0,
        "Different viewing angles should produce different outputs, found only {}% difference",
        diff_percent
    );
}

#[test]
fn test_cpu_tracer_center_pixel_hits_geometry() {
    // Test that center pixel hits the cube geometry (not background)
    let camera = CameraConfig::look_at(Vec3::new(-2.0, 1.5, -2.0), Vec3::ZERO, Vec3::Y);

    let mut tracer = CpuTracer::new();
    tracer.render_with_camera(128, 128, &camera);
    let output = tracer.image_buffer().expect("Should have image buffer");

    // Get center pixel
    let center_pixel = get_pixel_at(output, 64, 64);
    let background = (102u8, 128u8, 153u8);

    // Check if center pixel is significantly different from background
    let is_background = (center_pixel.0 as i16 - background.0 as i16).abs() < 5
        && (center_pixel.1 as i16 - background.1 as i16).abs() < 5
        && (center_pixel.2 as i16 - background.2 as i16).abs() < 5;

    // Center pixel should not be background (camera is looking at cube center)
    assert!(
        !is_background,
        "Center pixel should hit geometry, not background. Got RGB({}, {}, {})",
        center_pixel.0, center_pixel.1, center_pixel.2
    );
}
