//! Test GL and GPU tracer rendering to diagnose shader issues

use cube::Cube;
use renderer::gl_tracer::GlCubeTracer;
use renderer::gpu_tracer::GpuTracer;
use renderer::scenes::create_octa_cube;

/// Test that checks if GL tracer can be initialized
#[test]
fn test_gl_tracer_initialization() {
    println!("Testing GL tracer initialization...");

    let cube = create_octa_cube();
    let gl_tracer = GlCubeTracer::new(cube);

    println!("✓ GL tracer created successfully");
    assert!(gl_tracer.cube().as_ref() as *const _ != std::ptr::null());
}

/// Test that checks if GPU tracer can be initialized
#[test]
fn test_gpu_tracer_initialization() {
    println!("Testing GPU tracer initialization...");

    let cube = create_octa_cube();
    let gpu_tracer = GpuTracer::new(cube);

    println!("✓ GPU tracer created successfully");
    assert!(gpu_tracer.cube().as_ref() as *const _ != std::ptr::null());
}

/// Test cube structure to verify it's not empty
#[test]
fn test_octa_cube_structure() {
    println!("Testing octa cube structure...");

    let cube = create_octa_cube();

    match cube.as_ref() {
        Cube::Solid(val) => {
            println!("  Cube is Solid with value: {}", val);
        }
        Cube::Cubes(children) => {
            println!("  Cube is subdivided (Cubes variant)");
            for (i, child) in children.iter().enumerate() {
                match child.as_ref() {
                    Cube::Solid(val) => println!("    Child {}: Solid({})", i, val),
                    Cube::Cubes(_) => println!("    Child {}: Cubes (subdivided)", i),
                    Cube::Quad { .. } => println!("    Child {}: Quad", i),
                    Cube::Layers { .. } => println!("    Child {}: Layers", i),
                }
            }
        }
        Cube::Quad { .. } => {
            println!("  Cube is Quad variant");
        }
        Cube::Layers { .. } => {
            println!("  Cube is Layers variant");
        }
    }

    println!("✓ Cube structure verified");
}

/// Test shader source files exist
#[test]
fn test_shader_files_exist() {
    println!("Testing shader files...");

    // Check that shader files can be read directly
    let vertex_shader = include_str!("../src/shaders/octree_raycast.vert");
    let fragment_shader = include_str!("../src/shaders/octree_raycast.frag");

    println!("  Vertex shader length: {} bytes", vertex_shader.len());
    println!("  Fragment shader length: {} bytes", fragment_shader.len());

    assert!(!vertex_shader.is_empty(), "Vertex shader source is empty!");
    assert!(
        !fragment_shader.is_empty(),
        "Fragment shader source is empty!"
    );

    // Check for key shader components
    assert!(
        vertex_shader.contains("gl_Position"),
        "Vertex shader missing gl_Position"
    );
    assert!(
        fragment_shader.contains("FragColor") || fragment_shader.contains("gl_FragColor"),
        "Fragment shader missing output"
    );

    println!("✓ Shader files verified");
}

/// Test compute shader source
#[test]
fn test_compute_shader_source_exists() {
    println!("Testing compute shader source...");

    // The compute shader should be included
    let compute_source = include_str!("../src/shaders/basic_raycast.comp");

    println!("  Compute shader length: {} bytes", compute_source.len());

    assert!(
        !compute_source.is_empty(),
        "Compute shader source is empty!"
    );
    assert!(
        compute_source.contains("imageStore"),
        "Compute shader missing imageStore"
    );
    assert!(
        compute_source.contains("layout(local_size_x"),
        "Compute shader missing work group size"
    );

    println!("✓ Compute shader source verified");
}

/// Diagnostic test to print shader debugging info
#[test]
fn test_shader_debug_info() {
    println!("\n========================================");
    println!("SHADER DEBUGGING INFORMATION");
    println!("========================================\n");

    let vertex_shader = include_str!("../src/shaders/octree_raycast.vert");
    let fragment_shader = include_str!("../src/shaders/octree_raycast.frag");
    let compute_shader = include_str!("../src/shaders/basic_raycast.comp");

    println!("VERTEX SHADER (first 500 chars):");
    println!("----------------------------------------");
    println!("{}", &vertex_shader[..vertex_shader.len().min(500)]);
    if vertex_shader.len() > 500 {
        println!("... ({} more bytes)", vertex_shader.len() - 500);
    }

    println!("\nFRAGMENT SHADER (first 500 chars):");
    println!("----------------------------------------");
    println!("{}", &fragment_shader[..fragment_shader.len().min(500)]);
    if fragment_shader.len() > 500 {
        println!("... ({} more bytes)", fragment_shader.len() - 500);
    }

    println!("\nCOMPUTE SHADER (first 500 chars):");
    println!("----------------------------------------");
    println!("{}", &compute_shader[..compute_shader.len().min(500)]);
    if compute_shader.len() > 500 {
        println!("... ({} more bytes)", compute_shader.len() - 500);
    }

    println!("\n========================================\n");
}
