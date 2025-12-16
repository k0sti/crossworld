//! Combined test for both GL and GPU tracers
//!
//! This test creates a single OpenGL context and tests both tracers
//! sequentially to avoid event loop recreation issues.

use glow::*;
use glutin::config::ConfigTemplateBuilder;
use glutin::context::{ContextApi, ContextAttributesBuilder, Version};
use glutin::display::GetGlDisplay;
use glutin::prelude::*;
use glutin::surface::{Surface, SurfaceAttributesBuilder, WindowSurface};
use glutin_winit::DisplayBuilder;
use raw_window_handle::HasWindowHandle;
use renderer::gl_tracer::GlTracer;
use renderer::gpu_tracer::ComputeTracer;
use renderer::Camera;
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

    let bg_r = 51u8;
    let bg_g = 76u8;
    let bg_b = 102u8;

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
    }

    RenderAnalysis {
        total_pixels,
        background_count,
        non_background_count,
        unique_colors: unique_colors.len(),
        has_content: non_background_count > 0,
        all_same_color: unique_colors.len() == 1,
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
fn test_both_tracers() {
    println!("\n========================================");
    println!("COMBINED TRACER TEST");
    println!("========================================\n");

    let (_event_loop, _window, _gl_context, _surface, gl) = create_test_context();

    let version = unsafe { gl.get_parameter_string(VERSION) };
    let renderer = unsafe { gl.get_parameter_string(RENDERER) };
    println!("OpenGL version: {}", version);
    println!("OpenGL renderer: {}", renderer);
    println!();

    let cube = create_octa_cube();
    let camera = Camera::look_at(
        glam::Vec3::new(3.0, 2.0, 3.0),
        glam::Vec3::ZERO,
        glam::Vec3::Y,
    );
    let width = 256i32;
    let height = 256i32;

    // TEST 1: GL Tracer
    println!("=== TEST 1: GL TRACER (Fragment Shader) ===\n");

    let mut gl_tracer = GlTracer::new(cube.clone());

    unsafe {
        gl_tracer
            .init_gl(&gl)
            .expect("Failed to initialize GL tracer");
        println!("✓ GL tracer initialized");

        gl_tracer.render_to_gl_with_camera(&gl, width, height, &camera);
        gl.finish();
        println!("✓ Rendering complete");
    }

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

    save_debug_png(&pixels, width, height, "gl_tracer_output.png");

    unsafe {
        gl_tracer.destroy_gl(&gl);
    }

    if analysis.all_same_color {
        let first = (pixels[0], pixels[1], pixels[2]);
        println!(
            "\n❌ GL TRACER FAILED: All pixels are RGB({}, {}, {})",
            first.0, first.1, first.2
        );
        println!("   Shader runs but raytracing logic is broken\n");
    } else if analysis.has_content {
        println!(
            "\n✅ GL TRACER PASSED: {} unique colors rendered\n",
            analysis.unique_colors
        );
    } else {
        println!("\n❌ GL TRACER FAILED: Only background rendered\n");
    }

    // TEST 2: GPU Tracer
    println!("=== TEST 2: GPU TRACER (Compute Shader) ===\n");

    let mut gpu_tracer = ComputeTracer::new(cube);
    let init_result = unsafe { gpu_tracer.init_gl(&gl) };

    match init_result {
        Ok(()) => {
            println!("✓ GPU tracer initialized");

            // Clear to magenta to detect if anything is rendered
            unsafe {
                gl.clear_color(1.0, 0.0, 1.0, 1.0);
                gl.clear(COLOR_BUFFER_BIT);
            }

            unsafe {
                gpu_tracer.render_to_gl_with_camera(&gl, width, height, &camera);
                gl.finish();
                println!("✓ Rendering complete");
            }

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

            // Sample some pixel values
            println!("\nSample pixels:");
            for i in [
                0,
                width / 2,
                width - 1,
                (height / 2) * width,
                ((height - 1) * width),
            ] {
                let idx = (i * 4) as usize;
                if idx + 2 < pixels.len() {
                    println!(
                        "  Pixel {}: RGB({}, {}, {})",
                        i,
                        pixels[idx],
                        pixels[idx + 1],
                        pixels[idx + 2]
                    );
                }
            }

            save_debug_png(&pixels, width, height, "gpu_tracer_output.png");

            unsafe {
                gpu_tracer.destroy_gl(&gl);
            }

            if analysis.all_same_color {
                let first = (pixels[0], pixels[1], pixels[2]);
                println!(
                    "\n❌ GPU TRACER FAILED: All pixels are RGB({}, {}, {})",
                    first.0, first.1, first.2
                );
                println!("   Shader runs but produces uniform output");

                // Check if it's magenta (clearcolor) - means nothing was rendered
                if first.0 == 255 && first.1 == 0 && first.2 == 255 {
                    println!(
                        "   ⚠️  All pixels are MAGENTA (clear color) - compute/blit shader not executing!\n"
                    );
                } else {
                    println!("   Compute shader executes but raytracing logic is broken\n");
                }

                panic!("GPU tracer test failed - see output above");
            } else if !analysis.has_content {
                println!("\n❌ GPU TRACER FAILED: Only background rendered\n");
                panic!("GPU tracer produces only background");
            } else if analysis.unique_colors < 5 {
                println!(
                    "\n⚠️  GPU TRACER WARNING: Only {} unique colors (expected more)\n",
                    analysis.unique_colors
                );
            } else {
                println!(
                    "\n✅ GPU TRACER PASSED: {} unique colors rendered\n",
                    analysis.unique_colors
                );
            }
        }
        Err(e) => {
            println!("⚠️  GPU tracer not available: {}", e);
            println!("   (This is expected on systems without compute shader support)\n");
        }
    }

    println!("========================================\n");

    // Fail the test if GL tracer has issues
    assert!(
        !analysis.all_same_color,
        "GL tracer produces uniform color - raytracing broken"
    );
}
