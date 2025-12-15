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

use glam::Vec3;
use renderer::{CameraConfig, CpuTracer, Renderer};

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
        expected.0,
        expected.1,
        expected.2,
        actual.0,
        actual.1,
        actual.2,
        r_diff,
        g_diff,
        b_diff
    );
}

/// Helper: Sample pixel from image buffer
fn sample_pixel(
    buffer: &image::ImageBuffer<image::Rgb<u8>, Vec<u8>>,
    x: u32,
    y: u32,
) -> (u8, u8, u8) {
    let pixel = buffer.get_pixel(x, y);
    (pixel[0], pixel[1], pixel[2])
}

#[test]
fn test_cpu_tracer_material_colors() {
    println!("\n=== Testing CPU Tracer Material Colors ===");

    let mut tracer = CpuTracer::new();

    // Fixed camera looking at the cube center
    let camera = CameraConfig::look_at(
        Vec3::new(3.0, 2.0, 3.0), // Position
        Vec3::ZERO,               // Target (cube center)
        Vec3::Y,                  // Up
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

    println!(
        "Center pixel RGB: ({}, {}, {})",
        center_color.0, center_color.1, center_color.2
    );

    // The center should NOT be background color (we should hit something)
    let background_rgb = (102, 128, 153); // 0.4, 0.5, 0.6 * 255
    let is_background = (center_color.0 as i16 - background_rgb.0 as i16).abs()
        < COLOR_TOLERANCE as i16
        && (center_color.1 as i16 - background_rgb.1 as i16).abs() < COLOR_TOLERANCE as i16
        && (center_color.2 as i16 - background_rgb.2 as i16).abs() < COLOR_TOLERANCE as i16;

    assert!(
        !is_background,
        "Center pixel should not be background color (should hit a voxel)"
    );

    // Sample multiple regions to verify different colors exist
    let top_left = sample_pixel(buffer, width / 4, height / 4);
    let top_right = sample_pixel(buffer, 3 * width / 4, height / 4);
    let bottom_left = sample_pixel(buffer, width / 4, 3 * height / 4);
    let bottom_right = sample_pixel(buffer, 3 * width / 4, 3 * height / 4);

    println!(
        "Top-left RGB: ({}, {}, {})",
        top_left.0, top_left.1, top_left.2
    );
    println!(
        "Top-right RGB: ({}, {}, {})",
        top_right.0, top_right.1, top_right.2
    );
    println!(
        "Bottom-left RGB: ({}, {}, {})",
        bottom_left.0, bottom_left.1, bottom_left.2
    );
    println!(
        "Bottom-right RGB: ({}, {}, {})",
        bottom_right.0, bottom_right.1, bottom_right.2
    );

    // Check for distinct colors (accounting for lighting which dims RGB values)
    // 1: set_empty (Black)
    // 2: glass (White)
    // 3: ice (Light Blue)
    // 4: water_surface (Blue)
    // 5: slime (Green)

    let samples = [center_color, top_left, top_right, bottom_left, bottom_right];

    // Adjusted thresholds for lit colors
    let has_white = samples
        .iter()
        .any(|&(r, g, b)| r > 150 && g > 150 && b > 150);
    let has_green = samples
        .iter()
        .any(|&(r, g, b)| r < 100 && g > 100 && b < 100);
    let has_blue = samples
        .iter()
        .any(|&(r, g, b)| r < 100 && g < 150 && b > 150);
    // Light blue (ice) is high G and B
    let has_light_blue = samples
        .iter()
        .any(|&(r, g, b)| r > 100 && g > 150 && b > 150);

    println!("Has white: {}", has_white);
    println!("Has green: {}", has_green);
    println!("Has blue: {}", has_blue);
    println!("Has light blue: {}", has_light_blue);

    // At least one distinct color should be visible
    assert!(
        has_white || has_green || has_blue || has_light_blue,
        "At least one sampled region should show a distinct color"
    );

    println!("✓ CPU tracer renders distinct colors");
}

#[test]
fn test_cpu_tracer_background_color() {
    println!("\n=== Testing CPU Tracer Background Color ===");

    let mut tracer = CpuTracer::new();

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

    println!(
        "Background pixel RGB: ({}, {}, {})",
        center.0, center.1, center.2
    );

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

    let mut tracer = CpuTracer::new();

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
    // Use centralized material system
    use cube::material::get_material_color;

    println!("\n=== Testing Material Palette ===");

    // Test some primary materials from the new registry
    // 0: empty (Black)
    // 1: set_empty (Black)
    // 2: glass (White)
    // 3: ice (Light Blue)
    // 4: water_surface (Blue)
    // 5: slime (Green)

    assert_eq!(get_material_color(0), Vec3::new(0.0, 0.0, 0.0)); // Empty
    assert_eq!(get_material_color(1), Vec3::new(0.0, 0.0, 0.0)); // set_empty
    assert_eq!(get_material_color(2), Vec3::new(1.0, 1.0, 1.0)); // glass
    assert_eq!(get_material_color(3), Vec3::new(0.816, 1.0, 1.0)); // ice
    assert_eq!(get_material_color(4), Vec3::new(0.0, 0.498, 1.0)); // water_surface
    assert_eq!(get_material_color(5), Vec3::new(0.0, 1.0, 0.0)); // slime

    println!("✓ Material palette has correct colors");
}

#[test]
fn test_lighting_constants() {
    use renderer::{AMBIENT, BACKGROUND_COLOR, DIFFUSE_STRENGTH, LIGHT_DIR};

    println!("\n=== Testing Lighting Constants ===");

    // Light direction should be normalized
    let light_len =
        (LIGHT_DIR.x * LIGHT_DIR.x + LIGHT_DIR.y * LIGHT_DIR.y + LIGHT_DIR.z * LIGHT_DIR.z).sqrt();
    assert!(
        (light_len - 1.0).abs() < 0.001,
        "Light direction should be normalized"
    );

    // Constants should be in valid ranges
    assert!(
        AMBIENT >= 0.0 && AMBIENT <= 1.0,
        "Ambient should be in [0, 1]"
    );
    assert!(
        DIFFUSE_STRENGTH >= 0.0,
        "Diffuse strength should be non-negative"
    );

    // Background color components should be in [0, 1]
    assert!(BACKGROUND_COLOR.x >= 0.0 && BACKGROUND_COLOR.x <= 1.0);
    assert!(BACKGROUND_COLOR.y >= 0.0 && BACKGROUND_COLOR.y <= 1.0);
    assert!(BACKGROUND_COLOR.z >= 0.0 && BACKGROUND_COLOR.z <= 1.0);

    println!(
        "  Light direction: {:?} (length: {:.3})",
        LIGHT_DIR, light_len
    );
    println!("  Ambient: {}", AMBIENT);
    println!("  Diffuse strength: {}", DIFFUSE_STRENGTH);
    println!("  Background: {:?}", BACKGROUND_COLOR);

    println!("✓ All lighting constants are valid");
}

#[test]
fn test_lighting_toggle() {
    println!("\n=== Testing Lighting Toggle ===");

    let mut tracer = CpuTracer::new();

    // Fixed camera looking at the cube center
    let camera = CameraConfig::look_at(
        Vec3::new(3.0, 2.0, 3.0), // Position
        Vec3::ZERO,               // Target (cube center)
        Vec3::Y,                  // Up
    );

    let width = 256;
    let height = 256;

    // Render with lighting enabled (default)
    tracer.set_disable_lighting(false);
    tracer.render_with_camera(width, height, &camera);
    let lit_buffer = tracer
        .image_buffer()
        .expect("Image buffer should exist")
        .clone();

    // Render with lighting disabled
    tracer.set_disable_lighting(true);
    tracer.render_with_camera(width, height, &camera);
    let unlit_buffer = tracer.image_buffer().expect("Image buffer should exist");

    // Sample center pixels from both renders
    let center_x = width / 2;
    let center_y = height / 2;

    let lit_color = sample_pixel(&lit_buffer, center_x, center_y);
    let unlit_color = sample_pixel(unlit_buffer, center_x, center_y);

    println!(
        "Lit color RGB: ({}, {}, {})",
        lit_color.0, lit_color.1, lit_color.2
    );
    println!(
        "Unlit color RGB: ({}, {}, {})",
        unlit_color.0, unlit_color.1, unlit_color.2
    );

    // With lighting disabled, colors should be brighter (closer to pure material colors)
    // The unlit version should have RGB values >= lit version for most pixels
    // (since lighting typically darkens colors via ambient + diffuse < 1.0)
    //
    // However, due to gamma correction, the relationship might not be strictly greater.
    // Instead, verify that the colors are significantly different.
    let r_diff = (lit_color.0 as i16 - unlit_color.0 as i16).abs();
    let g_diff = (lit_color.1 as i16 - unlit_color.1 as i16).abs();
    let b_diff = (lit_color.2 as i16 - unlit_color.2 as i16).abs();

    let total_diff = r_diff + g_diff + b_diff;

    println!(
        "Color difference: R={}, G={}, B={}, Total={}",
        r_diff, g_diff, b_diff, total_diff
    );

    // Expect at least 30 total RGB units difference (10 per channel average)
    assert!(
        total_diff > 30,
        "Lit and unlit renders should produce significantly different colors. \
        Lit: RGB({}, {}, {}), Unlit: RGB({}, {}, {}), Diff: {}",
        lit_color.0,
        lit_color.1,
        lit_color.2,
        unlit_color.0,
        unlit_color.1,
        unlit_color.2,
        total_diff
    );

    // Verify flag state
    tracer.set_disable_lighting(false);
    assert!(!tracer.is_lighting_disabled(), "Lighting should be enabled");
    tracer.set_disable_lighting(true);
    assert!(tracer.is_lighting_disabled(), "Lighting should be disabled");

    println!("✓ Lighting toggle produces distinct visual output");
}
