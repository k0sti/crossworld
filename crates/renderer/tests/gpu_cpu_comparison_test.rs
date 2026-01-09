//! GPU vs CPU tracer comparison test
//!
//! This test validates that the GPU tracer (compute shader) produces output
//! that matches the CPU tracer for the same scene and camera configuration.

use glow::*;
use glutin::config::ConfigTemplateBuilder;
use glutin::context::{ContextApi, ContextAttributesBuilder, Version};
use glutin::display::GetGlDisplay;
use glutin::prelude::*;
use glutin::surface::{Surface, SurfaceAttributesBuilder, WindowSurface};
use glutin_winit::DisplayBuilder;
use image::{ImageBuffer, Rgb, Rgba};
use raw_window_handle::HasWindowHandle;
use renderer::Camera;
use renderer::gpu_tracer::ComputeTracer;
use renderer::scenes::create_octa_cube;
use renderer::{CpuTracer, Renderer};
use std::num::NonZeroU32;
use winit::event_loop::EventLoop;
use winit::window::Window;

#[cfg(target_os = "linux")]
use winit::platform::x11::EventLoopBuilderExtX11;

/// Color tolerance for GPU vs CPU comparison (RGB units)
/// Allow some tolerance for floating-point precision differences
const COLOR_TOLERANCE: u8 = 5;

/// Create OpenGL context for testing (requires compute shader support)
fn create_test_context() -> Option<(
    EventLoop<()>,
    Window,
    glutin::context::PossiblyCurrentContext,
    Surface<WindowSurface>,
    glow::Context,
)> {
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
        .ok()?;

    let window = window?;
    let window_handle = window.window_handle().ok().map(|h| h.as_raw());
    let gl_display = gl_config.display();

    // Request OpenGL 4.3 for compute shader support
    let context_attributes = ContextAttributesBuilder::new()
        .with_context_api(ContextApi::OpenGl(Some(Version::new(4, 3))))
        .build(window_handle);

    let gl_context = unsafe {
        gl_display
            .create_context(&gl_config, &context_attributes)
            .ok()?
    };

    let size = window.inner_size();
    let attrs = SurfaceAttributesBuilder::<WindowSurface>::new().build(
        window_handle?,
        NonZeroU32::new(size.width)?,
        NonZeroU32::new(size.height)?,
    );

    let gl_surface = unsafe { gl_display.create_window_surface(&gl_config, &attrs).ok()? };
    let gl_context = gl_context.make_current(&gl_surface).ok()?;

    let gl =
        unsafe { glow::Context::from_loader_function_cstr(|s| gl_display.get_proc_address(s)) };

    Some((event_loop, window, gl_context, gl_surface, gl))
}

/// Test output directory
fn test_output_dir() -> std::path::PathBuf {
    let mut path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.push("tests");
    path.push("output");
    std::fs::create_dir_all(&path).expect("Failed to create test output directory");
    path
}

fn save_debug_png_rgba(pixels: &[u8], width: i32, height: i32, filename: &str) {
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

fn save_debug_png_rgb(img: &ImageBuffer<Rgb<u8>, Vec<u8>>, filename: &str) {
    let output_path = test_output_dir().join(filename);
    if let Err(e) = img.save(&output_path) {
        eprintln!("Failed to save debug image: {}", e);
    } else {
        println!("  Saved to: {}", output_path.display());
    }
}

/// Calculate pixel difference statistics between CPU and GPU outputs
fn compare_images(
    cpu_img: &ImageBuffer<Rgb<u8>, Vec<u8>>,
    gpu_pixels: &[u8],
    width: i32,
    height: i32,
) -> (f32, usize, usize) {
    let total_pixels = (width * height) as usize;
    let mut matching_pixels = 0;
    let mut different_pixels = 0;
    let mut max_diff = 0u8;

    for y in 0..height as u32 {
        for x in 0..width as u32 {
            // CPU image has origin at top-left
            let cpu_pixel = cpu_img.get_pixel(x, y);

            // GPU framebuffer has origin at bottom-left, so flip Y
            let gpu_y = (height as u32 - 1 - y) as usize;
            let idx = (gpu_y * width as usize + x as usize) * 4;
            let gpu_r = gpu_pixels[idx];
            let gpu_g = gpu_pixels[idx + 1];
            let gpu_b = gpu_pixels[idx + 2];

            let diff_r = (cpu_pixel[0] as i16 - gpu_r as i16).unsigned_abs() as u8;
            let diff_g = (cpu_pixel[1] as i16 - gpu_g as i16).unsigned_abs() as u8;
            let diff_b = (cpu_pixel[2] as i16 - gpu_b as i16).unsigned_abs() as u8;

            let max_channel_diff = diff_r.max(diff_g).max(diff_b);
            max_diff = max_diff.max(max_channel_diff);

            if max_channel_diff <= COLOR_TOLERANCE {
                matching_pixels += 1;
            } else {
                different_pixels += 1;
            }
        }
    }

    let match_percent = (matching_pixels as f32 / total_pixels as f32) * 100.0;
    (match_percent, different_pixels, max_diff as usize)
}

#[test]
fn test_gpu_cpu_tracer_comparison() {
    println!("\n========================================");
    println!("GPU vs CPU TRACER COMPARISON TEST");
    println!("========================================\n");

    // Try to create OpenGL context with compute shader support
    let context_result = create_test_context();
    let Some((_event_loop, _window, _gl_context, _surface, gl)) = context_result else {
        println!("Could not create OpenGL 4.3 context with compute shader support.");
        println!("Skipping GPU vs CPU comparison test.");
        println!("========================================\n");
        return;
    };

    let version = unsafe { gl.get_parameter_string(VERSION) };
    let renderer_name = unsafe { gl.get_parameter_string(RENDERER) };
    println!("OpenGL version: {}", version);
    println!("OpenGL renderer: {}", renderer_name);
    println!();

    // Create test scene
    let cube = create_octa_cube();
    let camera = Camera::look_at(
        glam::Vec3::new(3.0, 2.0, 3.0),
        glam::Vec3::ZERO,
        glam::Vec3::Y,
    );
    let width = 256i32;
    let height = 256i32;

    // === Render with CPU tracer ===
    println!("--- Rendering with CPU tracer ---\n");
    let mut cpu_tracer = CpuTracer::new();
    cpu_tracer.render_with_camera(width as u32, height as u32, &camera);
    let cpu_img = cpu_tracer
        .image_buffer()
        .expect("CPU tracer should produce image");
    save_debug_png_rgb(cpu_img, "cpu_tracer_output.png");
    println!("  CPU tracer rendered successfully");
    println!();

    // === Initialize and render with GPU tracer ===
    println!("--- Rendering with GPU tracer ---\n");
    let mut gpu_tracer = ComputeTracer::new(cube);
    let init_result = unsafe { gpu_tracer.init_gl(&gl) };

    match init_result {
        Ok(()) => {
            println!("  GPU tracer initialized successfully");
        }
        Err(e) => {
            println!("  GPU tracer initialization failed: {}", e);
            println!("  This is expected if compute shaders are not supported.");
            println!("  Skipping comparison test.");
            println!("========================================\n");
            return;
        }
    }

    // Clear framebuffer
    unsafe {
        gl.viewport(0, 0, width, height);
        gl.clear_color(0.0, 0.0, 0.0, 1.0);
        gl.clear(COLOR_BUFFER_BIT);
    }

    // Render with GPU tracer
    unsafe {
        gpu_tracer.render_to_gl_with_camera(&gl, width, height, &camera);
        gl.finish();
    }

    // Read back GPU pixels
    let mut gpu_pixels = vec![0u8; (width * height * 4) as usize];
    unsafe {
        gl.read_pixels(
            0,
            0,
            width,
            height,
            RGBA,
            UNSIGNED_BYTE,
            PixelPackData::Slice(Some(&mut gpu_pixels)),
        );
    }
    save_debug_png_rgba(
        &gpu_pixels,
        width,
        height,
        "gpu_tracer_comparison_output.png",
    );
    println!("  GPU tracer rendered successfully");
    println!();

    // === Compare outputs ===
    println!("--- Comparing CPU vs GPU output ---\n");

    let (match_percent, different_pixels, max_diff) =
        compare_images(cpu_img, &gpu_pixels, width, height);

    println!("  Total pixels: {}", width * height);
    println!(
        "  Matching pixels (within tolerance {}): {:.1}%",
        COLOR_TOLERANCE, match_percent
    );
    println!("  Different pixels: {}", different_pixels);
    println!("  Maximum channel difference: {}", max_diff);
    println!();

    // Clean up
    unsafe {
        gpu_tracer.destroy_gl(&gl);
    }

    // Verify results
    // We expect at least 95% match (allowing for floating-point precision differences)
    let min_match_percent = 95.0;
    if match_percent >= min_match_percent {
        println!(
            "GPU tracer output matches CPU tracer ({:.1}% match)",
            match_percent
        );
    } else {
        println!(
            "GPU tracer output differs from CPU tracer ({:.1}% match, expected {}%)",
            match_percent, min_match_percent
        );
        println!("Check the saved images for visual comparison:");
        println!("  - tests/output/cpu_tracer_output.png");
        println!("  - tests/output/gpu_tracer_comparison_output.png");
    }

    println!();
    println!("========================================\n");

    assert!(
        match_percent >= min_match_percent,
        "GPU tracer output should match CPU tracer (got {:.1}%, expected {}%)",
        match_percent,
        min_match_percent
    );
}
