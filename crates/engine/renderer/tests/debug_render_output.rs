//! Debug test to render and inspect actual output

use renderer::cpu_tracer::CpuTracer;
use renderer::scenes::create_octa_cube;
use renderer::{Camera, Renderer};
use std::path::Path;

#[test]
fn test_render_and_save_debug_image() {
    println!("\n=== Rendering Debug Image ===");

    // Create octa cube
    let cube = create_octa_cube();
    let mut tracer = CpuTracer::new_with_cube(cube);

    // Setup camera looking at the cube from an angle
    let camera = Camera::look_at(
        glam::Vec3::new(3.0, 2.0, 3.0), // Camera position
        glam::Vec3::ZERO,               // Look at origin
        glam::Vec3::Y,                  // Up vector
    );

    // Render at moderate resolution
    let width = 256;
    let height = 256;

    println!("Rendering {}x{} image...", width, height);
    tracer.render_with_camera(width, height, &camera);

    // Get the buffer
    let buffer = tracer
        .image_buffer()
        .expect("Should have buffer after rendering");

    // Analyze the output
    println!("\n=== Image Analysis ===");

    // Count unique colors
    let mut color_histogram: std::collections::HashMap<[u8; 3], usize> =
        std::collections::HashMap::new();
    for pixel in buffer.pixels() {
        *color_histogram.entry(pixel.0).or_insert(0) += 1;
    }

    println!("Found {} unique colors:", color_histogram.len());
    let mut colors: Vec<_> = color_histogram.iter().collect();
    colors.sort_by_key(|(_, count)| std::cmp::Reverse(**count));

    for (color, count) in colors.iter().take(10) {
        let percentage = (**count as f32 / (width * height) as f32) * 100.0;
        println!(
            "  RGB({:3}, {:3}, {:3}): {:6} pixels ({:5.1}%)",
            color[0], color[1], color[2], count, percentage
        );
    }

    // Sample grid
    println!("\n=== Sample Grid (16x16) ===");
    for y in 0..16 {
        print!("  ");
        for x in 0..16 {
            let px = (x * width / 16) as u32;
            let py = (y * height / 16) as u32;
            let pixel = buffer.get_pixel(px, py);
            let [r, g, b] = pixel.0;

            // Simple ASCII visualization based on brightness
            let brightness = (r as u32 + g as u32 + b as u32) / 3;
            let char = match brightness {
                0..=50 => '█',
                51..=100 => '▓',
                101..=150 => '▒',
                151..=200 => '░',
                _ => ' ',
            };
            print!("{}", char);
        }
        println!();
    }

    // Save the image
    let output_dir = Path::new("tests/output");
    std::fs::create_dir_all(output_dir).ok();
    let output_path = output_dir.join("debug_cpu_render.png");

    println!("\n=== Saving Output ===");
    tracer
        .save_image(output_path.to_str().unwrap())
        .expect("Should save");
    println!("Saved to: {}", output_path.display());

    // Analyze raycast success
    println!("\n=== Raycast Analysis ===");
    // Background color after gamma correction: Vec3(0.2, 0.3, 0.4)^(1/2.2) ≈ (0.484, 0.576, 0.657)
    let background_rgb = [123, 147, 168]; // Gamma-corrected background color

    let background_pixels = color_histogram.get(&background_rgb).unwrap_or(&0);
    let total_pixels = (width * height) as usize;
    let octree_pixels = total_pixels - background_pixels;

    println!(
        "Background/miss pixels: {} ({:.1}%)",
        background_pixels,
        (*background_pixels as f32 / total_pixels as f32) * 100.0
    );
    println!(
        "Octree hit pixels: {} ({:.1}%)",
        octree_pixels,
        (octree_pixels as f32 / total_pixels as f32) * 100.0
    );

    // Basic validation
    assert!(
        color_histogram.len() > 1,
        "Should have multiple colors, got {}",
        color_histogram.len()
    );
    println!("\n✓ Test complete - check output image for visual inspection");
}

#[test]
fn test_render_multiple_angles() {
    println!("\n=== Rendering Multiple Angles ===");

    let cube = create_octa_cube();

    // Test different camera angles
    let angles = vec![
        ("front", glam::Vec3::new(0.0, 0.0, 3.0)),
        ("side", glam::Vec3::new(3.0, 0.0, 0.0)),
        ("top", glam::Vec3::new(0.0, 3.0, 0.0)),
        ("diagonal", glam::Vec3::new(2.0, 2.0, 2.0)),
    ];

    let output_dir = Path::new("tests/output");
    std::fs::create_dir_all(output_dir).ok();

    for (name, camera_pos) in angles {
        let mut tracer = CpuTracer::new_with_cube(cube.clone());
        let camera = Camera::look_at(camera_pos, glam::Vec3::ZERO, glam::Vec3::Y);

        tracer.render_with_camera(128, 128, &camera);

        let output_path = output_dir.join(format!("debug_cpu_{}.png", name));
        tracer.save_image(output_path.to_str().unwrap()).ok();

        println!("  Rendered {} view -> {}", name, output_path.display());
    }

    println!("✓ All angles rendered");
}
