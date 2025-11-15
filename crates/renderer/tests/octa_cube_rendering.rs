//! Octa cube rendering test - validates CPU raytracer with sparse octrees

mod scenes;

use renderer::cpu_tracer::CpuCubeTracer;
use renderer::renderer::{CameraConfig, Renderer};
use scenes::octa_cube::create_octa_cube;
use std::path::PathBuf;

/// Test output directory
fn test_output_dir() -> PathBuf {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.push("tests");
    path.push("output");
    std::fs::create_dir_all(&path).expect("Failed to create test output directory");
    path
}

#[test]
fn test_octa_cube_cpu_rendering() {
    // Create octa cube test scene
    let cube = create_octa_cube();

    // Create CPU raytracer
    let mut tracer = CpuCubeTracer::new_with_cube(cube);

    // Setup camera to view the octree
    // Position camera to see the two empty corners (octants 3 and 7)
    let camera = CameraConfig::look_at(
        glam::Vec3::new(2.5, 2.0, 2.5), // position
        glam::Vec3::ZERO,               // target
        glam::Vec3::Y,                  // up
    );

    // Render at 512x512 resolution
    let width = 512;
    let height = 512;
    tracer.render_with_camera(width, height, &camera);

    // Save output image
    let mut output_path = test_output_dir();
    output_path.push("octa_cube_cpu.png");
    tracer
        .save_image(output_path.to_str().unwrap())
        .expect("Failed to save CPU render output");

    // Verify image buffer was created
    let buffer = tracer
        .image_buffer()
        .expect("CPU tracer should have image buffer");

    // Verify dimensions
    assert_eq!(buffer.width(), width);
    assert_eq!(buffer.height(), height);

    // Basic validation: check that not all pixels are the background color
    // (This ensures the octree was actually rendered)
    let mut non_background_pixels = 0;
    let background_color = [51, 76, 102]; // RGB(0.2, 0.3, 0.4) * 255

    for pixel in buffer.pixels() {
        let [r, g, b] = pixel.0;
        // Allow small tolerance for gamma correction and anti-aliasing
        if (r as i32 - background_color[0]).abs() > 5
            || (g as i32 - background_color[1]).abs() > 5
            || (b as i32 - background_color[2]).abs() > 5
        {
            non_background_pixels += 1;
        }
    }

    // At least 10% of pixels should be non-background (the octree should be visible)
    let min_visible_pixels = (width * height) / 10;
    assert!(
        non_background_pixels > min_visible_pixels,
        "Expected at least {} non-background pixels, got {}",
        min_visible_pixels,
        non_background_pixels
    );

    println!(
        "✓ Octa cube CPU rendering test passed: {} / {} pixels visible",
        non_background_pixels,
        width * height
    );
}

#[test]
fn test_octa_cube_multiple_angles() {
    // Create octa cube test scene
    let cube = create_octa_cube();

    // Test from multiple camera angles to ensure all solid voxels are rendered
    let camera_positions = vec![
        ("front", glam::Vec3::new(0.0, 0.0, 3.0)),
        ("corner", glam::Vec3::new(2.5, 2.0, 2.5)),
        ("top", glam::Vec3::new(0.0, 3.0, 0.0)),
    ];

    let width = 256;
    let height = 256;

    for (name, position) in camera_positions {
        let mut tracer = CpuCubeTracer::new_with_cube(cube.clone());

        let camera = CameraConfig::look_at(
            position,
            glam::Vec3::ZERO, // target
            glam::Vec3::Y,    // up
        );

        tracer.render_with_camera(width, height, &camera);

        // Save output for visual inspection
        let mut output_path = test_output_dir();
        output_path.push(format!("octa_cube_{}.png", name));
        tracer
            .save_image(output_path.to_str().unwrap())
            .expect("Failed to save render output");

        // Verify image buffer exists
        assert!(tracer.image_buffer().is_some());
    }

    println!("✓ Octa cube multi-angle rendering test passed");
}
