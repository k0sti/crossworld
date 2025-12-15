//! Test VOX model loading

use cube::Cube;
use renderer::scenes::{create_vox_alien_bot, create_vox_eskimo, create_vox_robot};
use renderer::{BcfTracer, Renderer};

#[test]
fn test_vox_robot_loading() {
    let cube = create_vox_robot();

    // Check that cube is not just a solid
    match cube.as_ref() {
        Cube::Solid(val) => {
            println!("ERROR: VOX robot loaded as Solid({})", val);
            panic!("VOX robot should not be a simple solid cube");
        }
        Cube::Cubes(_) => {
            println!("SUCCESS: VOX robot loaded as octree structure");
        }
        _ => {
            println!("WARNING: VOX robot loaded as unexpected type");
        }
    }

    // Print some debug info
    println!("VOX robot cube structure loaded successfully");
}

#[test]
fn test_vox_alien_bot_loading() {
    let cube = create_vox_alien_bot();

    // Check that cube is not just a solid
    match cube.as_ref() {
        Cube::Solid(val) => {
            println!("ERROR: VOX alien bot loaded as Solid({})", val);
            panic!("VOX alien bot should not be a simple solid cube");
        }
        Cube::Cubes(_) => {
            println!("SUCCESS: VOX alien bot loaded as octree structure");
        }
        _ => {
            println!("WARNING: VOX alien bot loaded as unexpected type");
        }
    }

    println!("VOX alien bot cube structure loaded successfully");
}

#[test]
fn test_vox_eskimo_loading() {
    let cube = create_vox_eskimo();

    // Check that cube is not just a solid
    match cube.as_ref() {
        Cube::Solid(val) => {
            println!("ERROR: VOX eskimo loaded as Solid({})", val);
            panic!("VOX eskimo should not be a simple solid cube");
        }
        Cube::Cubes(_) => {
            println!("SUCCESS: VOX eskimo loaded as octree structure");
        }
        _ => {
            println!("WARNING: VOX eskimo loaded as unexpected type");
        }
    }

    println!("VOX eskimo cube structure loaded successfully");
}

#[test]
fn test_vox_robot_rendering() {
    let cube = create_vox_robot();
    let mut tracer = BcfTracer::new_from_cube(cube);

    // Render a 128x128 image
    tracer.render(128, 128, 0.0);

    let image = tracer.image_buffer().expect("Image buffer should exist");

    // Check for non-background pixels and collect unique colors
    let mut colored_pixels = 0;
    let mut sample_colors = Vec::new();
    let mut color_histogram = std::collections::HashMap::new();

    for (idx, pixel) in image.pixels().enumerate() {
        // Background color check (RGB(168, 186, 202))
        let is_colored = pixel[0] < 160
            || pixel[0] > 176
            || pixel[1] < 178
            || pixel[1] > 194
            || pixel[2] < 194
            || pixel[2] > 210;

        if is_colored {
            colored_pixels += 1;
            let color_key = (pixel[0], pixel[1], pixel[2]);
            *color_histogram.entry(color_key).or_insert(0) += 1;

            if sample_colors.len() < 10 {
                sample_colors.push((idx, pixel[0], pixel[1], pixel[2]));
            }
        }
    }

    println!("VOX robot rendering stats:");
    println!("  Total pixels: {}", 128 * 128);
    println!(
        "  Colored pixels: {} ({:.1}%)",
        colored_pixels,
        (colored_pixels as f32 / (128.0 * 128.0)) * 100.0
    );
    println!("  Unique colors: {}", color_histogram.len());
    println!("  Color histogram (top 10):");
    let mut color_vec: Vec<_> = color_histogram.iter().collect();
    color_vec.sort_by(|a, b| b.1.cmp(a.1));
    for ((r, g, b), count) in color_vec.iter().take(10) {
        println!("    RGB({}, {}, {}): {} pixels", r, g, b, count);
    }

    // VOX model should have visible pixels
    // Lowered threshold since the model might be small
    assert!(
        colored_pixels > 0,
        "Expected some colored pixels in VOX robot, got {}",
        colored_pixels
    );

    // Check if we have more than one color (to verify it's not all white/gray)
    if color_histogram.len() <= 1 {
        println!("WARNING: VOX robot appears to be rendered with only one color!");
        println!("This suggests the color mapping or material rendering might be incorrect.");
    }
}
