//! Tests for BCF CPU tracer
//!
//! These tests validate that the BCF-based CPU raytracer produces correct output
//! and matches the behavior of the existing CPU tracer.

use cube::{Cube, io::bcf::serialize_bcf};
use renderer::{BcfTracer, Camera, CpuTracer, Renderer};
use std::rc::Rc;

/// Test: BCF tracer can render a simple solid cube
#[test]
fn test_bcf_tracer_renders_solid_cube() {
    let mut tracer = BcfTracer::new();

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
        // Background color is RGB(168, 186, 202) after gamma correction
        let is_background = pixel[0] > 160
            && pixel[0] < 176
            && pixel[1] > 178
            && pixel[1] < 194
            && pixel[2] > 194
            && pixel[2] < 210;
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
    let mut tracer = BcfTracer::new_from_cube(cube);

    // Render a 128x128 image
    tracer.render(128, 128, 0.0);

    let image = tracer.image_buffer().expect("Image buffer should exist");

    // Basic sanity check
    assert_eq!(image.width(), 128);
    assert_eq!(image.height(), 128);

    // Check for non-background pixels
    let mut colored_pixels = 0;
    for pixel in image.pixels() {
        // Any pixel that's not close to background color (RGB(168, 186, 202))
        let is_colored = pixel[0] < 160
            || pixel[0] > 176
            || pixel[1] < 178
            || pixel[1] > 194
            || pixel[2] < 194
            || pixel[2] > 210;
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
    let mut tracer = BcfTracer::new();

    // Fixed camera position
    let camera = Camera::default();

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
    let mut tracer = BcfTracer::new_from_cube(empty_cube);

    // Render
    tracer.render(64, 64, 0.0);
    let image = tracer.image_buffer().expect("Image buffer should exist");

    // Count background and non-background pixels for debugging
    let mut background_count = 0;
    let mut non_background_pixels = Vec::new();

    for (idx, pixel) in image.pixels().enumerate() {
        // Background color check with tolerance
        // Background is RGB(168, 186, 202) after gamma correction
        let is_background = pixel[0] > 160
            && pixel[0] < 176
            && pixel[1] > 178
            && pixel[1] < 194
            && pixel[2] > 194
            && pixel[2] < 210;
        if is_background {
            background_count += 1;
        } else if non_background_pixels.len() < 5 {
            non_background_pixels.push((idx, *pixel));
        }
    }

    if !non_background_pixels.is_empty() {
        eprintln!("Empty cube rendering stats:");
        eprintln!("  Background pixels: {}", background_count);
        eprintln!("  Non-background pixels: {}", 64 * 64 - background_count);
        eprintln!("  Sample non-background pixels:");
        for (idx, pixel) in &non_background_pixels {
            eprintln!(
                "    Pixel {}: RGB({}, {}, {})",
                idx, pixel[0], pixel[1], pixel[2]
            );
        }
    }

    assert!(
        non_background_pixels.is_empty(),
        "Empty cube should render as all background color, but found {} non-background pixels",
        64 * 64 - background_count
    );
}

/// Test: BCF tracer with max value cube (255)
#[test]
fn test_bcf_tracer_max_value() {
    let max_cube = Rc::new(Cube::Solid(255u8));
    let mut tracer = BcfTracer::new_from_cube(max_cube);

    // Render
    tracer.render(64, 64, 0.0);
    let image = tracer.image_buffer().expect("Image buffer should exist");

    // Count pixels that are not background
    let mut non_background_count = 0;
    let mut sample_pixels = Vec::new();

    for (idx, pixel) in image.pixels().enumerate() {
        // Check if pixel is NOT background (RGB(168, 186, 202))
        let is_not_background = pixel[0] < 160
            || pixel[0] > 176
            || pixel[1] < 178
            || pixel[1] > 194
            || pixel[2] < 194
            || pixel[2] > 210;

        if is_not_background {
            non_background_count += 1;
            if sample_pixels.len() < 5 {
                sample_pixels.push((idx, *pixel));
            }
        }
    }

    if non_background_count == 0 {
        eprintln!("Max value cube (255) rendered as all background!");
        eprintln!("This suggests the cube is not being hit by rays.");
    } else {
        eprintln!("Found {} non-background pixels", non_background_count);
        for (idx, pixel) in &sample_pixels {
            eprintln!(
                "  Pixel {}: RGB({}, {}, {})",
                idx, pixel[0], pixel[1], pixel[2]
            );
        }
    }

    assert!(
        non_background_count > 0,
        "Max value cube should render with visible pixels, but all {} pixels are background",
        64 * 64
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
    let mut tracer = BcfTracer::new_from_cube(Rc::new(parsed_cube));

    // Should still render correctly
    tracer.render(64, 64, 0.0);
    let image = tracer.image_buffer().expect("Image buffer should exist");

    // Check for colored pixels
    let mut colored_count = 0;
    for pixel in image.pixels() {
        let is_colored = pixel[0] < 160
            || pixel[0] > 176
            || pixel[1] < 178
            || pixel[1] > 194
            || pixel[2] < 194
            || pixel[2] > 210;
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
    let mut tracer = BcfTracer::new_from_cube(cube);

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
    let mut bcf_tracer = BcfTracer::new_from_cube(cube.clone());
    let mut cpu_tracer = CpuTracer::new_with_cube(cube);

    // Fixed camera for consistency
    let camera = Camera::default();

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

/// Test: BCF tracer handles depth 2 extended octa cube
#[test]
fn test_bcf_tracer_depth_2_extended_octa() {
    use renderer::scenes::create_cube_from_id;

    let cube = create_cube_from_id("extended").expect("Failed to load extended model");
    let mut tracer = BcfTracer::new_from_cube(cube);

    // Render a 128x128 image
    tracer.render(128, 128, 0.0);

    let image = tracer.image_buffer().expect("Image buffer should exist");

    // Basic sanity check
    assert_eq!(image.width(), 128);
    assert_eq!(image.height(), 128);

    // Check for non-background pixels
    let mut colored_pixels = 0;
    for pixel in image.pixels() {
        // Any pixel that's not close to background color (RGB(168, 186, 202))
        let is_colored = pixel[0] < 160
            || pixel[0] > 176
            || pixel[1] < 178
            || pixel[1] > 194
            || pixel[2] < 194
            || pixel[2] > 210;
        if is_colored {
            colored_pixels += 1;
        }
    }

    // Extended octa cube has more voxels than basic octa cube
    // We expect at least 10% of pixels to show the cube
    assert!(
        colored_pixels > 128 * 128 / 10,
        "Expected at least {} colored pixels in depth 2 cube, got {}",
        128 * 128 / 10,
        colored_pixels
    );

    eprintln!(
        "Depth 2 extended octa cube rendered with {} colored pixels ({:.1}%)",
        colored_pixels,
        (colored_pixels as f32 / (128.0 * 128.0)) * 100.0
    );
}

/// Test: Error material colors are rendered correctly
#[test]
fn test_bcf_tracer_error_materials() {
    // Create a cube with error material values (1-7)
    // Material 1 should render as hot pink, not black
    let error_cube = Rc::new(Cube::Solid(1u8)); // Error material 1
    let mut tracer = BcfTracer::new_from_cube(error_cube);

    // Render
    tracer.render(64, 64, 0.0);
    let image = tracer.image_buffer().expect("Image buffer should exist");

    // Find a pixel that hits the cube (not background)
    let mut found_error_color = false;
    for pixel in image.pixels() {
        // Background color check
        let is_background = pixel[0] > 160
            && pixel[0] < 176
            && pixel[1] > 178
            && pixel[1] < 194
            && pixel[2] > 194
            && pixel[2] < 210;

        if !is_background {
            // Error material 1 is hot pink (1.0, 0.0, 0.3)
            // After gamma correction: (255, 0, ~155)
            // Should be RED heavy, not black
            assert!(
                pixel[0] > 200,
                "Error material 1 should be hot pink (red component high), got RGB({}, {}, {})",
                pixel[0],
                pixel[1],
                pixel[2]
            );
            found_error_color = true;
            break;
        }
    }

    assert!(
        found_error_color,
        "Should find at least one error material pixel"
    );
}

/// Performance benchmark (disabled by default, run with --ignored to measure)
#[test]
#[ignore]
fn bench_bcf_tracer_render_time() {
    use std::time::Instant;

    let mut tracer = BcfTracer::new();

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

/// Test: BCF tracer handles depth 3 cube with random subdivisions
#[test]
fn test_bcf_tracer_depth_3_cube() {
    use renderer::scenes::create_cube_from_id;

    let cube = create_cube_from_id("depth3").expect("Failed to load depth3 model");
    let mut tracer = BcfTracer::new_from_cube(cube);

    // Render a 128x128 image
    tracer.render(128, 128, 0.0);

    let image = tracer.image_buffer().expect("Image buffer should exist");

    // Basic sanity check
    assert_eq!(image.width(), 128);
    assert_eq!(image.height(), 128);

    // Check for non-background pixels
    let mut colored_pixels = 0;
    for pixel in image.pixels() {
        // Any pixel that's not close to background color (RGB(168, 186, 202))
        let is_colored = pixel[0] < 160
            || pixel[0] > 176
            || pixel[1] < 178
            || pixel[1] > 194
            || pixel[2] < 194
            || pixel[2] > 210;
        if is_colored {
            colored_pixels += 1;
        }
    }

    // Depth 3 cube has complex structure with random cubes
    // We expect at least 10% of pixels to show the cube
    assert!(
        colored_pixels > 128 * 128 / 10,
        "Expected at least {} colored pixels in depth 3 cube, got {}",
        128 * 128 / 10,
        colored_pixels
    );

    eprintln!(
        "Depth 3 cube rendered with {} colored pixels ({:.1}%)",
        colored_pixels,
        (colored_pixels as f32 / (128.0 * 128.0)) * 100.0
    );
}

/// Test: Debug BCF serialization for depth 3 cube
#[test]
fn test_bcf_depth_3_serialization() {
    use renderer::scenes::create_cube_from_id;

    let cube = create_cube_from_id("depth3").expect("Failed to load depth3 model");

    // Serialize to BCF
    let bcf_data = serialize_bcf(&cube);

    eprintln!("Depth 3 cube BCF data:");
    eprintln!("  Total size: {} bytes", bcf_data.len());
    eprintln!(
        "  First 64 bytes: {:?}",
        &bcf_data[..64.min(bcf_data.len())]
    );

    // Check that we have a reasonable amount of data
    // Depth 3 should have more data than depth 2
    assert!(
        bcf_data.len() > 100,
        "BCF data seems too small for depth 3: {} bytes",
        bcf_data.len()
    );
}

/// Test: Compare BCF tracer with Pure Rust tracer for depth 3 cube
#[test]
fn test_bcf_vs_pure_rust_depth_3() {
    use renderer::scenes::create_cube_from_id;

    let cube = create_cube_from_id("depth3").expect("Failed to load depth3 model");

    // Create BCF tracer
    let mut bcf_tracer = BcfTracer::new_from_cube(cube.clone());
    bcf_tracer.set_disable_lighting(true); // Disable lighting for exact color comparison

    // Create Pure Rust tracer
    let mut rust_tracer = CpuTracer::new_with_cube(cube.clone());
    rust_tracer.set_disable_lighting(true); // Disable lighting for exact color comparison

    // Render with both tracers
    let width = 64;
    let height = 64;
    bcf_tracer.render(width, height, 0.0);
    rust_tracer.render(width, height, 0.0);

    let bcf_image = bcf_tracer.image_buffer().expect("BCF image should exist");
    let rust_image = rust_tracer.image_buffer().expect("Rust image should exist");

    // Compare pixel-by-pixel
    let mut different_pixels = 0;
    let mut total_non_background = 0;

    for y in 0..height {
        for x in 0..width {
            let bcf_pixel = bcf_image.get_pixel(x, y);
            let rust_pixel = rust_image.get_pixel(x, y);

            // Skip background pixels (both tracers should agree on background)
            let is_background = rust_pixel[0] > 160
                && rust_pixel[0] < 176
                && rust_pixel[1] > 178
                && rust_pixel[1] < 194
                && rust_pixel[2] > 194
                && rust_pixel[2] < 210;

            if !is_background {
                total_non_background += 1;

                // Check if pixels are significantly different
                let r_diff = (bcf_pixel[0] as i32 - rust_pixel[0] as i32).abs();
                let g_diff = (bcf_pixel[1] as i32 - rust_pixel[1] as i32).abs();
                let b_diff = (bcf_pixel[2] as i32 - rust_pixel[2] as i32).abs();

                // Allow small differences due to lighting/rounding (threshold: 10)
                if r_diff > 10 || g_diff > 10 || b_diff > 10 {
                    different_pixels += 1;

                    // Print first few differences for debugging
                    if different_pixels <= 5 {
                        eprintln!(
                            "Pixel ({}, {}): BCF={:?}, Rust={:?}, diff=({}, {}, {})",
                            x, y, bcf_pixel, rust_pixel, r_diff, g_diff, b_diff
                        );
                    }
                }
            }
        }
    }

    let difference_percent = if total_non_background > 0 {
        (different_pixels as f32 / total_non_background as f32) * 100.0
    } else {
        0.0
    };

    eprintln!(
        "Depth 3 comparison: {}/{} non-background pixels differ ({:.1}%)",
        different_pixels, total_non_background, difference_percent
    );

    // BCF and Pure Rust tracers should match closely
    // Allow up to 5% difference due to implementation variations
    assert!(
        difference_percent < 5.0,
        "BCF tracer differs from Pure Rust tracer by {:.1}% (expected < 5%)",
        difference_percent
    );
}

/// Test: BCF tracer handles test expansion cube
#[test]
fn test_bcf_tracer_test_expansion_cube() {
    use renderer::scenes::create_cube_from_id;

    let cube = create_cube_from_id("test_expansion").expect("Failed to load test_expansion model");
    let mut tracer = BcfTracer::new_from_cube(cube);

    // Render a 128x128 image
    tracer.render(128, 128, 0.0);

    let image = tracer.image_buffer().expect("Image buffer should exist");

    // Basic sanity check
    assert_eq!(image.width(), 128);
    assert_eq!(image.height(), 128);

    // Check for non-background pixels
    let mut colored_pixels = 0;
    for pixel in image.pixels() {
        // Any pixel that's not close to background color (RGB(168, 186, 202))
        let is_colored = pixel[0] < 160
            || pixel[0] > 176
            || pixel[1] < 178
            || pixel[1] > 194
            || pixel[2] < 194
            || pixel[2] > 210;
        if is_colored {
            colored_pixels += 1;
        }
    }

    // Test expansion cube should have visible pixels
    // We expect at least 10% of pixels to show the cube
    assert!(
        colored_pixels > 128 * 128 / 10,
        "Expected at least {} colored pixels in test expansion cube, got {}",
        128 * 128 / 10,
        colored_pixels
    );

    eprintln!(
        "Test expansion cube rendered with {} colored pixels ({:.1}%)",
        colored_pixels,
        (colored_pixels as f32 / (128.0 * 128.0)) * 100.0
    );
}
