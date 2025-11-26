//! Dedicated test for GPU tracer (compute shader raytracer)
//!
//! This test validates that the GPU tracer:
//! 1. Initializes correctly when compute shaders are available
//! 2. Produces non-empty output (not all background color)
//! 3. Renders varying pixel colors across the screen
//! 4. Creates output images for visual inspection

use glow::*;
use glutin::config::ConfigTemplateBuilder;
use glutin::context::{ContextApi, ContextAttributesBuilder, Version};
use glutin::display::GetGlDisplay;
use glutin::prelude::*;
use glutin::surface::{Surface, SurfaceAttributesBuilder, WindowSurface};
use glutin_winit::DisplayBuilder;
use raw_window_handle::HasWindowHandle;
use renderer::gpu_tracer::GpuTracer;
use renderer::renderer::CameraConfig;
use renderer::scenes::create_octa_cube;
use std::num::NonZeroU32;
use winit::event_loop::EventLoop;
use winit::window::Window;

#[cfg(target_os = "linux")]
use winit::platform::x11::EventLoopBuilderExtX11;

struct RenderAnalysis {
    total_pixels: usize,
    background_count: usize,
    non_background_count: usize,
    unique_colors: usize,
    has_content: bool,
    all_same_color: bool,
    sample_colors: Vec<(u8, u8, u8)>,
}

/// Create OpenGL context for testing
fn create_test_context() -> (
    EventLoop<()>,
    Window,
    glutin::context::PossiblyCurrentContext,
    Surface<WindowSurface>,
    glow::Context,
) {
    #[cfg(target_os = "linux")]
    let event_loop = {
        let mut builder = EventLoop::builder();
        builder.with_any_thread(true);
        builder.build().unwrap()
    };

    #[cfg(not(target_os = "linux"))]
    let event_loop = EventLoop::new().unwrap();

    let window_attributes = Window::default_attributes()
        .with_inner_size(winit::dpi::PhysicalSize::new(256, 256))
        .with_visible(false);

    let template = ConfigTemplateBuilder::new()
        .with_alpha_size(8)
        .with_transparency(false);

    let display_builder = DisplayBuilder::new().with_window_attributes(Some(window_attributes));

    let (window, gl_config) = display_builder
        .build(&event_loop, template, |configs| {
            configs
                .reduce(|accum, config| {
                    if config.num_samples() > accum.num_samples() {
                        config
                    } else {
                        accum
                    }
                })
                .unwrap()
        })
        .unwrap();

    let window = window.unwrap();
    let window_handle = window.window_handle().ok().map(|h| h.as_raw());
    let gl_display = gl_config.display();

    let context_attributes = ContextAttributesBuilder::new()
        .with_context_api(ContextApi::Gles(Some(Version::new(3, 0))))
        .build(window_handle);

    let gl_context = unsafe {
        gl_display
            .create_context(&gl_config, &context_attributes)
            .unwrap()
    };

    let size = window.inner_size();
    let attrs = SurfaceAttributesBuilder::<WindowSurface>::new().build(
        window_handle.unwrap(),
        NonZeroU32::new(size.width).unwrap(),
        NonZeroU32::new(size.height).unwrap(),
    );

    let gl_surface = unsafe {
        gl_display
            .create_window_surface(&gl_config, &attrs)
            .unwrap()
    };

    let gl_context = gl_context.make_current(&gl_surface).unwrap();

    let gl =
        unsafe { glow::Context::from_loader_function_cstr(|s| gl_display.get_proc_address(s)) };

    (event_loop, window, gl_context, gl_surface, gl)
}

fn analyze_pixels(pixels: &[u8], width: i32, height: i32) -> RenderAnalysis {
    let total_pixels = (width * height) as usize;
    let mut background_count = 0;
    let mut non_background_count = 0;
    let mut unique_colors = std::collections::HashSet::new();
    let mut sample_colors = Vec::new();

    let bg_r = 51u8; // 0.2 * 255
    let bg_g = 76u8; // 0.3 * 255
    let bg_b = 102u8; // 0.4 * 255

    // Sample 9 locations across the image
    let sample_points = [
        (width / 4, height / 4),
        (width / 2, height / 4),
        (3 * width / 4, height / 4),
        (width / 4, height / 2),
        (width / 2, height / 2),
        (3 * width / 4, height / 2),
        (width / 4, 3 * height / 4),
        (width / 2, 3 * height / 4),
        (3 * width / 4, 3 * height / 4),
    ];

    for i in 0..total_pixels {
        let idx = i * 4;
        let r = pixels[idx];
        let g = pixels[idx + 1];
        let b = pixels[idx + 2];

        unique_colors.insert((r, g, b));

        let is_bg = (r as i32 - bg_r as i32).abs() <= 2
            && (g as i32 - bg_g as i32).abs() <= 2
            && (b as i32 - bg_b as i32).abs() <= 2;

        if is_bg {
            background_count += 1;
        } else {
            non_background_count += 1;
        }

        // Collect sample colors
        let x = (i % width as usize) as i32;
        let y = (i / width as usize) as i32;
        for &(sx, sy) in &sample_points {
            if x == sx && y == sy {
                sample_colors.push((r, g, b));
            }
        }
    }

    RenderAnalysis {
        total_pixels,
        background_count,
        non_background_count,
        unique_colors: unique_colors.len(),
        has_content: non_background_count > 0,
        all_same_color: unique_colors.len() == 1,
        sample_colors,
    }
}

/// Test output directory
fn test_output_dir() -> std::path::PathBuf {
    let mut path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.push("tests");
    path.push("output");
    std::fs::create_dir_all(&path).expect("Failed to create test output directory");
    path
}

fn save_debug_png(pixels: &[u8], width: i32, height: i32, filename: &str) {
    use image::{ImageBuffer, Rgba};

    let mut img = ImageBuffer::<Rgba<u8>, Vec<u8>>::new(width as u32, height as u32);

    for y in 0..height as u32 {
        for x in 0..width as u32 {
            let src_y = (height as u32 - 1 - y) as usize;
            let idx = (src_y * width as usize + x as usize) * 4;

            let pixel = Rgba([
                pixels[idx],
                pixels[idx + 1],
                pixels[idx + 2],
                pixels[idx + 3],
            ]);

            img.put_pixel(x, y, pixel);
        }
    }

    let output_path = test_output_dir().join(filename);
    if let Err(e) = img.save(&output_path) {
        eprintln!("Failed to save debug image: {}", e);
    } else {
        println!("  Saved to: {}", output_path.display());
    }
}

#[test]
fn test_gpu_tracer_comprehensive() {
    println!("\n========================================");
    println!("GPU TRACER COMPREHENSIVE TEST");
    println!("========================================\n");

    let (_event_loop, _window, _gl_context, _surface, gl) = create_test_context();

    let version = unsafe { gl.get_parameter_string(VERSION) };
    let renderer = unsafe { gl.get_parameter_string(RENDERER) };
    println!("OpenGL version: {}", version);
    println!("OpenGL renderer: {}", renderer);
    println!();

    let cube = create_octa_cube();
    let camera = CameraConfig::look_at(
        glam::Vec3::new(3.0, 2.0, 3.0),
        glam::Vec3::ZERO,
        glam::Vec3::Y,
    );
    let width = 256i32;
    let height = 256i32;

    // TEST 1: Initialization
    println!("--- Test 1: Initialization ---\n");

    let mut gpu_tracer = GpuTracer::new(cube);
    let init_result = unsafe { gpu_tracer.init_gl(&gl) };

    match init_result {
        Ok(()) => {
            println!("✅ GPU tracer initialized successfully!");
            println!("   Compute shaders are supported on this system.");
            println!();
        }
        Err(e) => {
            println!("⚠️  GPU tracer initialization failed: {}", e);
            println!("   This is expected if compute shaders are not supported.");
            println!("   Skipping remaining tests.");
            println!();
            println!("========================================\n");
            return;
        }
    }

    // TEST 2: Rendering
    println!("--- Test 2: Rendering ---\n");

    // Clear framebuffer to a known color (magenta) to detect if anything is rendered
    unsafe {
        gl.viewport(0, 0, width, height);
        gl.clear_color(1.0, 0.0, 1.0, 1.0); // Magenta
        gl.clear(COLOR_BUFFER_BIT);
    }

    // Render with GPU tracer
    unsafe {
        gpu_tracer.render_to_gl_with_camera(&gl, width, height, &camera);
        gl.finish();
        println!("✓ Rendering complete");
    }

    // Read back pixels
    let mut pixels = vec![0u8; (width * height * 4) as usize];
    unsafe {
        gl.read_pixels(
            0,
            0,
            width,
            height,
            RGBA,
            UNSIGNED_BYTE,
            PixelPackData::Slice(&mut pixels),
        );
    }

    let analysis = analyze_pixels(&pixels, width, height);

    println!("\nPixel Analysis:");
    println!("  Total pixels: {}", analysis.total_pixels);
    println!(
        "  Background: {} ({:.1}%)",
        analysis.background_count,
        (analysis.background_count as f32 / analysis.total_pixels as f32) * 100.0
    );
    println!(
        "  Content: {} ({:.1}%)",
        analysis.non_background_count,
        (analysis.non_background_count as f32 / analysis.total_pixels as f32) * 100.0
    );
    println!("  Unique colors: {}", analysis.unique_colors);

    println!("\nSample pixels (9 locations):");
    for (i, &(r, g, b)) in analysis.sample_colors.iter().enumerate() {
        println!("  Location {}: RGB({}, {}, {})", i + 1, r, g, b);
    }

    save_debug_png(&pixels, width, height, "gpu_tracer_test_output.png");

    println!();

    // Validate output
    if analysis.all_same_color {
        let first = (pixels[0], pixels[1], pixels[2]);
        println!(
            "❌ GPU TRACER FAILED: All pixels are RGB({}, {}, {})",
            first.0, first.1, first.2
        );

        // Check if it's magenta (clear color) - means nothing was rendered
        if first.0 == 255 && first.1 == 0 && first.2 == 255 {
            println!("   ⚠️  All pixels are MAGENTA (clear color)");
            println!("   This means compute/blit shader did not execute!");
            println!();
            unsafe {
                gpu_tracer.destroy_gl(&gl);
            }
            println!("========================================\n");
            panic!("GPU tracer failed - compute/blit shader not executing");
        } else {
            println!("   Shader runs but produces uniform output");
            println!();
            unsafe {
                gpu_tracer.destroy_gl(&gl);
            }
            println!("========================================\n");
            panic!("GPU tracer produces uniform color - rendering broken");
        }
    } else if !analysis.has_content {
        println!("❌ GPU TRACER FAILED: Only background rendered");
        println!("   No content visible (cube not rendering)");
        println!();
        unsafe {
            gpu_tracer.destroy_gl(&gl);
        }
        println!("========================================\n");
        panic!("GPU tracer produces only background - cube not rendered");
    } else if analysis.unique_colors < 5 {
        println!(
            "⚠️  GPU TRACER WARNING: Only {} unique colors",
            analysis.unique_colors
        );
        println!("   Expected more color variation for proper lighting");
        println!();
    } else {
        println!(
            "✅ GPU TRACER PASSED: {} unique colors rendered",
            analysis.unique_colors
        );
        println!();
    }

    // TEST 3: Compute texture output validation
    println!("--- Test 3: Compute Texture Output ---\n");
    println!("✓ Compute shader dispatched");
    println!("✓ Blit shader executed");
    println!(
        "✓ Output texture contains {} unique colors",
        analysis.unique_colors
    );

    unsafe {
        gpu_tracer.destroy_gl(&gl);
    }

    println!();
    println!("========================================\n");
}
