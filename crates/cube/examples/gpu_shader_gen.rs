//! Example: GPU Shader Generation
//!
//! Demonstrates how to compile expressions to WGSL shaders for GPU evaluation.
//! This example shows the shader generation API without requiring a GPU context.

use cube::function::compile_gpu;

fn main() {
    println!("=== GPU Shader Generation Examples ===\n");

    // Example 1: Simple terrain with noise
    println!("1. Simple Terrain (Noise-based)");
    let terrain = "if noise(x * 0.1, y * 0.1, z * 0.1) > 0.5 then STONE else AIR";
    demonstrate_shader(terrain);

    // Example 2: Layered terrain with height
    println!("\n2. Layered Terrain");
    let layered = "let height = noise(x * 0.1, z * 0.1, 0) * 10; if y < height then if y < height - 2 then STONE else GRASS else AIR";
    demonstrate_shader(layered);

    // Example 3: Animated wave pattern
    println!("\n3. Animated Wave Pattern (Time-based)");
    let wave = "sin(x * 3.14 + time) * 0.5 + 0.5";
    demonstrate_shader(wave);

    // Example 4: Complex FBM terrain
    println!("\n4. Complex FBM Terrain");
    let fbm_terrain = "if fbm(x * 0.05, y * 0.05, z * 0.05, 4) > 0.3 then STONE else AIR";
    demonstrate_shader(fbm_terrain);

    // Example 5: Match-based material selection
    println!("\n5. Vertical Material Layers");
    let layers = "match floor(y * 4) { 0 => BEDROCK, 1 => STONE, 2 => DIRT, _ => GRASS }";
    demonstrate_shader(layers);
}

fn demonstrate_shader(source: &str) {
    println!("Expression: {}", source);

    match compile_gpu(source) {
        Ok(gpu_fn) => {
            println!("✓ Compilation successful");
            println!("  - Uses time: {}", gpu_fn.uses_time);
            println!("  - Uses noise: {}", gpu_fn.uses_noise);
            println!("  - Complexity: {}", gpu_fn.complexity);

            // Show a snippet of the generated shader
            let shader = gpu_fn.shader_source();
            let lines: Vec<&str> = shader.lines().collect();

            println!("\n  Generated shader structure:");
            println!("    - Total lines: {}", lines.len());

            // Show compute entry point
            if let Some(compute_line) = lines.iter().find(|l| l.contains("@compute")) {
                println!("    - Compute shader: {}", compute_line.trim());
            }

            // Show expression evaluation line
            if let Some(result_line) = lines.iter().find(|l| l.contains("let result: f32 =")) {
                println!("    - Expression: {}", result_line.trim());
            }

            // Count noise functions if used
            if gpu_fn.uses_noise {
                let noise_funcs = ["fn noise3", "fn fbm", "fn turbulence"];
                let count = noise_funcs
                    .iter()
                    .filter(|&f| shader.contains(f))
                    .count();
                println!("    - Noise functions: {} defined", count);
            }
        }
        Err(e) => {
            println!("✗ Compilation failed: {}", e);
        }
    }
}
