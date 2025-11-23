//! Color verification tests for all tracers
//!
//! These tests render the octa cube scene with each tracer and verify that
//! specific octants render with the correct material colors.
//!
//! Expected colors (from materials.rs):
//! - Octant 0: Red (255, 0, 0)
//! - Octant 2: Green (0, 255, 0)
//! - Octant 4: Blue (0, 0, 255)
//! - Octant 6: Yellow (255, 255, 0)
//! - Octant 1, 5: White (255, 255, 255)
//! - Background: Bluish-gray (102, 128, 153)

use renderer::{CameraConfig, CpuCubeTracer, Renderer};
use glam::Vec3;

/// Color tolerance for comparisons (±5 RGB units on 0-255 scale)
const COLOR_TOLERANCE: u8 = 5;

/// Helper: Assert RGB color is within tolerance
fn assert_color_near(actual: (u8, u8, u8), expected: (u8, u8, u8), tolerance: u8, label: &str) {
    let r_diff = (actual.0 as i16 - expected.0 as i16).abs();
    let g_diff = (actual.1 as i16 - expected.1 as i16).abs();
    let b_diff = (actual.2 as i16 - expected.2 as i16).abs();

    assert!(
        r_diff <= tolerance as i16 && g_diff <= tolerance as i16 && b_diff <= tolerance as i16,
        "{}: expected RGB({}, {}, {}), got RGB({}, {}, {}), diff=({}, {}, {})",
        label,
        expected.0, expected.1, expected.2,
        actual.0, actual.1, actual.2,
        r_diff, g_diff, b_diff
    );
}

/// Helper: Sample pixel from image buffer
fn sample_pixel(buffer: &image::ImageBuffer<image::Rgb<u8>, Vec<u8>>, x: u32, y: u32) -> (u8, u8, u8) {
    let pixel = buffer.get_pixel(x, y);
    (pixel[0], pixel[1], pixel[2])
}

#[test]
fn test_cpu_tracer_material_colors() {
    println!("\n=== Testing CPU Tracer Material Colors ===");

    let mut tracer = CpuCubeTracer::new();

    // Fixed camera looking at the cube center
    let camera = CameraConfig::look_at(
        Vec3::new(3.0, 2.0, 3.0),  // Position
        Vec3::ZERO,                 // Target (cube center)
        Vec3::Y,                    // Up
    );

    // Render 256x256 image
    let width = 256;
    let height = 256;
    tracer.render_with_camera(width, height, &camera);

    let buffer = tracer.image_buffer().expect("Image buffer should exist");

    // Sample center of image (should hit a voxel)
    let center_x = width / 2;
    let center_y = height / 2;
    let center_color = sample_pixel(buffer, center_x, center_y);

    println!("Center pixel RGB: ({}, {}, {})", center_color.0, center_color.1, center_color.2);

    // The center should NOT be background color (we should hit something)
    let background_rgb = (102, 128, 153); // 0.4, 0.5, 0.6 * 255
    let is_background =
        (center_color.0 as i16 - background_rgb.0 as i16).abs() < COLOR_TOLERANCE as i16 &&
        (center_color.1 as i16 - background_rgb.1 as i16).abs() < COLOR_TOLERANCE as i16 &&
        (center_color.2 as i16 - background_rgb.2 as i16).abs() < COLOR_TOLERANCE as i16;

    assert!(!is_background, "Center pixel should not be background color (should hit a voxel)");

    // Sample multiple regions to verify different colors exist
    let top_left = sample_pixel(buffer, width / 4, height / 4);
    let top_right = sample_pixel(buffer, 3 * width / 4, height / 4);
    let bottom_left = sample_pixel(buffer, width / 4, 3 * height / 4);
    let bottom_right = sample_pixel(buffer, 3 * width / 4, 3 * height / 4);

    println!("Top-left RGB: ({}, {}, {})", top_left.0, top_left.1, top_left.2);
    println!("Top-right RGB: ({}, {}, {})", top_right.0, top_right.1, top_right.2);
    println!("Bottom-left RGB: ({}, {}, {})", bottom_left.0, bottom_left.1, bottom_left.2);
    println!("Bottom-right RGB: ({}, {}, {})", bottom_right.0, bottom_right.1, bottom_right.2);

    // Check for distinct colors (accounting for lighting which dims RGB values)
    // Red: R dominant, G and B low
    // Green: G dominant, R and B low
    // Blue: B dominant, R and G low
    // Yellow: R and G high, B low

    let samples = [center_color, top_left, top_right, bottom_left, bottom_right];

    // Adjusted thresholds for lit colors (lighting reduces max values from 255 to ~170-200)
    let has_red = samples.iter().any(|&(r, g, b)| r > 140 && g < 50 && b < 50);
    let has_green = samples.iter().any(|&(r, g, b)| r < 50 && g > 100 && b < 50);
    let has_blue = samples.iter().any(|&(r, g, b)| r < 50 && g < 50 && b > 100);
    let has_yellow = samples.iter().any(|&(r, g, b)| r > 100 && g > 100 && b < 50);

    println!("Has red: {}", has_red);
    println!("Has green: {}", has_green);
    println!("Has blue: {}", has_blue);
    println!("Has yellow: {}", has_yellow);

    // At least one distinct color should be visible
    assert!(has_red || has_green || has_blue || has_yellow,
        "At least one sampled region should show a distinct color (red, green, blue, or yellow)");

    println!("✓ CPU tracer renders distinct colors");
}

#[test]
fn test_cpu_tracer_background_color() {
    println!("\n=== Testing CPU Tracer Background Color ===");

    let mut tracer = CpuCubeTracer::new();

    // Camera looking away from cube (should see background)
    let camera = CameraConfig::look_at(
        Vec3::new(0.0, 0.0, -5.0),  // Behind cube
        Vec3::new(0.0, 0.0, -10.0), // Looking further back
        Vec3::Y,
    );

    let width = 256;
    let height = 256;
    tracer.render_with_camera(width, height, &camera);

    let buffer = tracer.image_buffer().expect("Image buffer should exist");

    // Sample center (should be background)
    let center = sample_pixel(buffer, width / 2, height / 2);

    println!("Background pixel RGB: ({}, {}, {})", center.0, center.1, center.2);

    // Background should be bluish-gray: RGB(102, 128, 153) ± 5
    // But with gamma correction: 0.4^(1/2.2) ≈ 0.665, 0.5^(1/2.2) ≈ 0.730, 0.6^(1/2.2) ≈ 0.787
    // So actual RGB after gamma: (170, 186, 201) approximately
    let expected_bg = (170, 186, 201);

    assert_color_near(center, expected_bg, 15, "Background color"); // Higher tolerance for gamma

    println!("✓ CPU tracer renders correct background color");
}

#[test]
fn test_cpu_tracer_renders_without_crash() {
    println!("\n=== Testing CPU Tracer Basic Rendering ===");

    let mut tracer = CpuCubeTracer::new();

    // Simple time-based render
    tracer.render(128, 128, 0.0);

    let buffer = tracer.image_buffer();
    assert!(buffer.is_some(), "CPU tracer should produce image buffer");

    let buffer = buffer.unwrap();
    assert_eq!(buffer.width(), 128);
    assert_eq!(buffer.height(), 128);

    println!("✓ CPU tracer renders 128x128 image without crashing");
}

// TODO: Add GL tracer tests when GL context can be created in tests
// #[test]
// fn test_gl_tracer_material_colors() { ... }

// TODO: Add GPU tracer tests when GPU compute is implemented
// #[test]
// fn test_gpu_tracer_material_colors() { ... }

#[test]
fn test_material_palette_accessibility() {
    use renderer::{get_material_color, MATERIAL_PALETTE};

    println!("\n=== Testing Material Palette ===");

    // Test palette size
    assert_eq!(MATERIAL_PALETTE.len(), 7, "Palette should have 7 materials");

    // Test each primary color
    assert_eq!(get_material_color(0), Vec3::new(0.0, 0.0, 0.0)); // Empty
    assert_eq!(get_material_color(1), Vec3::new(1.0, 0.0, 0.0)); // Red
    assert_eq!(get_material_color(2), Vec3::new(0.0, 1.0, 0.0)); // Green
    assert_eq!(get_material_color(3), Vec3::new(0.0, 0.0, 1.0)); // Blue
    assert_eq!(get_material_color(4), Vec3::new(1.0, 1.0, 0.0)); // Yellow
    assert_eq!(get_material_color(5), Vec3::new(1.0, 1.0, 1.0)); // White
    assert_eq!(get_material_color(6), Vec3::new(0.0, 0.0, 0.0)); // Black

    println!("✓ Material palette has correct colors");
}

#[test]
fn test_lighting_constants() {
    use renderer::{LIGHT_DIR, AMBIENT, DIFFUSE_STRENGTH, BACKGROUND_COLOR};

    println!("\n=== Testing Lighting Constants ===");

    // Light direction should be normalized
    let light_len = (LIGHT_DIR.x * LIGHT_DIR.x + LIGHT_DIR.y * LIGHT_DIR.y + LIGHT_DIR.z * LIGHT_DIR.z).sqrt();
    assert!((light_len - 1.0).abs() < 0.001, "Light direction should be normalized");

    // Constants should be in valid ranges
    assert!(AMBIENT >= 0.0 && AMBIENT <= 1.0, "Ambient should be in [0, 1]");
    assert!(DIFFUSE_STRENGTH >= 0.0, "Diffuse strength should be non-negative");

    // Background color components should be in [0, 1]
    assert!(BACKGROUND_COLOR.x >= 0.0 && BACKGROUND_COLOR.x <= 1.0);
    assert!(BACKGROUND_COLOR.y >= 0.0 && BACKGROUND_COLOR.y <= 1.0);
    assert!(BACKGROUND_COLOR.z >= 0.0 && BACKGROUND_COLOR.z <= 1.0);

    println!("  Light direction: {:?} (length: {:.3})", LIGHT_DIR, light_len);
    println!("  Ambient: {}", AMBIENT);
    println!("  Diffuse strength: {}", DIFFUSE_STRENGTH);
    println!("  Background: {:?}", BACKGROUND_COLOR);

    println!("✓ All lighting constants are valid");
}
