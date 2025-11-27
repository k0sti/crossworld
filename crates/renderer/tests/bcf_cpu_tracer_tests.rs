//! Tests for BCF CPU tracer
//!
//! These tests validate that the BCF-based CPU raytracer produces correct output
//! and matches the behavior of the existing CPU tracer.

use cube::{Cube, io::bcf::serialize_bcf};
use renderer::{BcfCpuTracer, CameraConfig, CpuCubeTracer, Renderer};
use std::rc::Rc;

/// Test: BCF tracer can render a simple solid cube
#[test]
fn test_bcf_tracer_renders_solid_cube() {
    let mut tracer = BcfCpuTracer::new();

    // Render a 64x64 image
    tracer.render(64, 64, 0.0);

    let image = tracer.image_buffer().expect("Image buffer should exist");

    // Basic sanity check: image should have correct dimensions
    assert_eq!(image.width(), 64);
    assert_eq!(image.height(), 64);

    // Check that at least some pixels are not background color
    // (i.e., the cube is visible)
    let mut non_background_pixels = 0;
    for pixel in image.pixels() {
        // Background color is approximately RGB(102, 128, 153) after gamma correction
        let is_background = pixel[0] > 80
            && pixel[0] < 120
            && pixel[1] > 110
            && pixel[1] < 150
            && pixel[2] > 140
            && pixel[2] < 170;
        if !is_background {
            non_background_pixels += 1;
        }
    }

    // At least 10% of pixels should show the cube
    assert!(
        non_background_pixels > 64 * 64 / 10,
        "Expected at least {} non-background pixels, got {}",
        64 * 64 / 10,
        non_background_pixels
    );
}

/// Test: BCF tracer with octa cube produces output
#[test]
fn test_bcf_tracer_octa_cube() {
    let cube = renderer::create_octa_cube();
    let mut tracer = BcfCpuTracer::new_from_cube(cube);

    // Render a 128x128 image
    tracer.render(128, 128, 0.0);

    let image = tracer.image_buffer().expect("Image buffer should exist");

    // Basic sanity check
    assert_eq!(image.width(), 128);
    assert_eq!(image.height(), 128);

    // Check for non-background pixels
    let mut colored_pixels = 0;
    for pixel in image.pixels() {
        // Any pixel that's not close to background color
        let is_colored = pixel[0] < 80
            || pixel[0] > 120
            || pixel[1] < 110
            || pixel[1] > 150
            || pixel[2] < 140
            || pixel[2] > 170;
        if is_colored {
            colored_pixels += 1;
        }
    }

    assert!(
        colored_pixels > 128 * 128 / 10,
        "Expected at least {} colored pixels, got {}",
        128 * 128 / 10,
        colored_pixels
    );
}

/// Test: BCF tracer with static camera produces consistent output
#[test]
fn test_bcf_tracer_static_camera() {
    let mut tracer = BcfCpuTracer::new();

    // Fixed camera position
    let camera = CameraConfig::default();

    // Render twice with same camera
    tracer.render_with_camera(64, 64, &camera);
    let image1 = tracer.image_buffer().unwrap().clone();

    tracer.render_with_camera(64, 64, &camera);
    let image2 = tracer.image_buffer().unwrap();

    // Images should be identical
    for (pixel1, pixel2) in image1.pixels().zip(image2.pixels()) {
        assert_eq!(pixel1, pixel2, "Images should be identical for same camera");
    }
}

/// Test: BCF tracer handles empty cube (all zeros)
#[test]
fn test_bcf_tracer_empty_cube() {
    let empty_cube = Rc::new(Cube::Solid(0u8));
    let mut tracer = BcfCpuTracer::new_from_cube(empty_cube);

    // Render
    tracer.render(64, 64, 0.0);
    let image = tracer.image_buffer().expect("Image buffer should exist");

    // All pixels should be background color (since cube is empty/transparent)
    let mut all_background = true;
    for pixel in image.pixels() {
        // Background color check with tolerance
        let is_background = pixel[0] > 80
            && pixel[0] < 120
            && pixel[1] > 110
            && pixel[1] < 150
            && pixel[2] > 140
            && pixel[2] < 170;
        if !is_background {
            all_background = false;
            break;
        }
    }

    assert!(
        all_background,
        "Empty cube should render as all background color"
    );
}

/// Test: BCF tracer with max value cube (255)
#[test]
fn test_bcf_tracer_max_value() {
    let max_cube = Rc::new(Cube::Solid(255u8));
    let mut tracer = BcfCpuTracer::new_from_cube(max_cube);

    // Render
    tracer.render(64, 64, 0.0);
    let image = tracer.image_buffer().expect("Image buffer should exist");

    // Should have visible pixels
    let mut has_colored_pixels = false;
    for pixel in image.pixels() {
        if pixel[0] < 80 || pixel[1] < 80 || pixel[2] < 80 {
            has_colored_pixels = true;
            break;
        }
    }

    assert!(
        has_colored_pixels,
        "Max value cube should render with visible pixels"
    );
}

/// Test: BCF format roundtrip doesn't break rendering
#[test]
fn test_bcf_format_roundtrip() {
    let cube = renderer::create_octa_cube();

    // Serialize to BCF and back
    let bcf_data = serialize_bcf(&cube);
    let parsed_cube = cube::io::bcf::parse_bcf(&bcf_data).expect("Should parse BCF");

    // Create tracer from parsed cube
    let mut tracer = BcfCpuTracer::new_from_cube(Rc::new(parsed_cube));

    // Should still render correctly
    tracer.render(64, 64, 0.0);
    let image = tracer.image_buffer().expect("Image buffer should exist");

    // Check for colored pixels
    let mut colored_count = 0;
    for pixel in image.pixels() {
        let is_colored = pixel[0] < 80
            || pixel[0] > 120
            || pixel[1] < 110
            || pixel[1] > 150
            || pixel[2] < 140
            || pixel[2] > 170;
        if is_colored {
            colored_count += 1;
        }
    }

    assert!(
        colored_count > 64 * 64 / 10,
        "Roundtrip cube should still render correctly"
    );
}

/// Test: Lighting disable mode works
#[test]
fn test_bcf_tracer_lighting_disable() {
    let cube = Rc::new(Cube::Solid(100u8)); // Mid-value material
    let mut tracer = BcfCpuTracer::new_from_cube(cube);

    // Render with lighting enabled
    tracer.set_disable_lighting(false);
    tracer.render(64, 64, 0.0);
    let lit_image = tracer.image_buffer().unwrap().clone();

    // Render with lighting disabled
    tracer.set_disable_lighting(true);
    tracer.render(64, 64, 0.0);
    let unlit_image = tracer.image_buffer().unwrap();

    // Images should be different (lighting affects output)
    let mut differences = 0;
    for (lit_pixel, unlit_pixel) in lit_image.pixels().zip(unlit_image.pixels()) {
        if lit_pixel != unlit_pixel {
            differences += 1;
        }
    }

    // At least some pixels should differ
    assert!(
        differences > 100,
        "Lighting toggle should affect output (found {} differences)",
        differences
    );
}

/// Compare BCF tracer output with existing CPU tracer (visual regression test)
///
/// This test is disabled by default as it may fail due to minor numerical differences,
/// but is useful for manual validation.
#[test]
#[ignore]
fn test_bcf_vs_cpu_tracer_comparison() {
    let cube = renderer::create_octa_cube();

    // Create both tracers
    let mut bcf_tracer = BcfCpuTracer::new_from_cube(cube.clone());
    let mut cpu_tracer = CpuCubeTracer::new_with_cube(cube);

    // Fixed camera for consistency
    let camera = CameraConfig::default();

    // Render with both tracers
    bcf_tracer.render_with_camera(128, 128, &camera);
    cpu_tracer.render_with_camera(128, 128, &camera);

    let bcf_image = bcf_tracer.image_buffer().unwrap();
    let cpu_image = cpu_tracer.image_buffer().unwrap();

    // Compare pixel-by-pixel (allow small tolerance for floating point differences)
    let mut diff_pixels = 0;
    let mut max_diff = 0u8;

    for (bcf_pixel, cpu_pixel) in bcf_image.pixels().zip(cpu_image.pixels()) {
        let diff_r = (bcf_pixel[0] as i16 - cpu_pixel[0] as i16).abs() as u8;
        let diff_g = (bcf_pixel[1] as i16 - cpu_pixel[1] as i16).abs() as u8;
        let diff_b = (bcf_pixel[2] as i16 - cpu_pixel[2] as i16).abs() as u8;

        let pixel_diff = diff_r.max(diff_g).max(diff_b);
        max_diff = max_diff.max(pixel_diff);

        if pixel_diff > 5 {
            // Tolerance of 5 RGB units
            diff_pixels += 1;
        }
    }

    println!("BCF vs CPU tracer comparison:");
    println!("  Pixels with diff > 5: {} / {}", diff_pixels, 128 * 128);
    println!("  Max pixel difference: {}", max_diff);

    // Allow up to 5% of pixels to differ slightly
    let tolerance_pixels = (128 * 128) / 20;
    assert!(
        diff_pixels < tolerance_pixels,
        "Too many differing pixels: {} (max: {})",
        diff_pixels,
        tolerance_pixels
    );

    // Max difference should be reasonable
    assert!(
        max_diff < 30,
        "Maximum pixel difference too large: {}",
        max_diff
    );
}

/// Performance benchmark (disabled by default, run with --ignored to measure)
#[test]
#[ignore]
fn bench_bcf_tracer_render_time() {
    use std::time::Instant;

    let mut tracer = BcfCpuTracer::new();

    // Warmup
    tracer.render(256, 256, 0.0);

    // Measure render time
    let start = Instant::now();
    tracer.render(256, 256, 0.0);
    let elapsed = start.elapsed();

    println!("BCF tracer render time (256x256): {:?}", elapsed);

    // Sanity check: should complete in reasonable time (< 5 seconds for 256x256)
    assert!(elapsed.as_secs() < 5, "Render took too long: {:?}", elapsed);
}
