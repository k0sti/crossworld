//! Validation tests to ensure renderers actually produce visible output

use renderer::cpu_tracer::CpuTracer;
use renderer::{Camera, Renderer};
use renderer::scenes::create_octa_cube;

#[test]
fn test_cpu_renderer_produces_visible_output() {
    // Create octa cube
    let cube = create_octa_cube();
    let mut tracer = CpuTracer::new_with_cube(cube);

    // Setup camera
    let camera = Camera::look_at(
        glam::Vec3::new(2.5, 2.0, 2.5),
        glam::Vec3::ZERO,
        glam::Vec3::Y,
    );

    // Render at small resolution for fast testing
    let width = 128;
    let height = 128;
    tracer.render_with_camera(width, height, &camera);

    // Get image buffer
    let buffer = tracer
        .image_buffer()
        .expect("Should have image buffer after rendering");

    // Count non-background pixels
    let background_color = [170, 186, 201]; // RGB(0.4, 0.5, 0.6) * 255 with gamma correction
    let mut non_background_count = 0;
    let mut sample_pixels = Vec::new();

    for (x, y, pixel) in buffer.enumerate_pixels() {
        let [r, g, b] = pixel.0;

        // Store some sample pixels for debugging
        if x % 32 == 0 && y % 32 == 0 {
            sample_pixels.push(((x, y), [r, g, b]));
        }

        // Check if pixel is different from background (with tolerance)
        if (r as i32 - background_color[0] as i32).abs() > 5
            || (g as i32 - background_color[1] as i32).abs() > 5
            || (b as i32 - background_color[2] as i32).abs() > 5
        {
            non_background_count += 1;
        }
    }

    // Print sample pixels for debugging
    println!("Sample pixels:");
    for ((x, y), rgb) in &sample_pixels {
        println!(
            "  ({:3}, {:3}): RGB({:3}, {:3}, {:3})",
            x, y, rgb[0], rgb[1], rgb[2]
        );
    }

    println!(
        "Non-background pixels: {} / {}",
        non_background_count,
        width * height
    );
    println!("Background color: {:?}", background_color);

    // Check if all pixels are the same color (which would indicate no actual octree rendering)
    let first_pixel = buffer.get_pixel(0, 0).0;
    let all_same = buffer.pixels().all(|p| p.0 == first_pixel);

    if all_same {
        println!(
            "WARNING: All pixels are the same color {:?}! Octree may not be rendering.",
            first_pixel
        );
        println!("This suggests bounding box hits but octree raycast fails.");
    } else {
        println!("Good: Image has varying colors, suggesting octree is being rendered.");
    }

    // Fail if all pixels are the same (octree not rendering)
    assert!(
        !all_same,
        "All pixels are identical! Octree is not being rendered. Got color {:?}",
        first_pixel
    );

    // Should have at least 5% non-background pixels
    let min_visible = (width * height) / 20;
    assert!(
        non_background_count >= min_visible,
        "Expected at least {} visible pixels, got {}. Rendering may be broken!",
        min_visible,
        non_background_count
    );
}

#[test]
fn test_default_cpu_renderer_produces_output() {
    // Test that the default constructor also works
    let mut tracer = CpuTracer::new();

    // Use default camera settings
    let camera = Camera::look_at(
        glam::Vec3::new(3.0, 2.0, 3.0),
        glam::Vec3::ZERO,
        glam::Vec3::Y,
    );

    let width = 128;
    let height = 128;
    tracer.render_with_camera(width, height, &camera);

    let buffer = tracer.image_buffer().expect("Should have image buffer");

    // Count non-background pixels
    let background_color = [170, 186, 201]; // RGB(0.4, 0.5, 0.6) * 255 with gamma correction
    let non_background = buffer
        .pixels()
        .filter(|p| {
            let [r, g, b] = p.0;
            (r as i32 - background_color[0] as i32).abs() > 5
                || (g as i32 - background_color[1] as i32).abs() > 5
                || (b as i32 - background_color[2] as i32).abs() > 5
        })
        .count();

    println!(
        "Default renderer non-background pixels: {} / {}",
        non_background,
        width * height
    );

    assert!(
        non_background > 0,
        "Default renderer produced completely empty image!"
    );
}
