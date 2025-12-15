// Import from the renderer library crate
use renderer::{ComputeTracer, CpuTracer, GlTracer, Renderer, create_octa_cube};

// Bin-specific modules
mod egui_app;

use egui_app::DualRendererApp;

use glow::*;
use glutin::config::ConfigTemplateBuilder;
use glutin::context::{ContextApi, ContextAttributesBuilder, Version};
use glutin::display::GetGlDisplay;
use glutin::prelude::*;
use glutin::surface::{SurfaceAttributesBuilder, WindowSurface};
use glutin_winit::DisplayBuilder;
use raw_window_handle::HasWindowHandle;
use std::error::Error;
use std::num::NonZeroU32;
use std::sync::Arc;
use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::window::{Window, WindowId};

#[cfg(target_os = "linux")]
#[allow(unused_imports)]
use winit::platform::x11::EventLoopBuilderExtX11;

#[derive(Default)]
struct App {
    window: Option<Window>,
    gl_context: Option<glutin::context::PossiblyCurrentContext>,
    gl_surface: Option<glutin::surface::Surface<WindowSurface>>,
    gl: Option<Arc<Context>>,
    egui_ctx: Option<egui::Context>,
    egui_state: Option<egui_winit::State>,
    painter: Option<egui_glow::Painter>,
    dual_renderer: Option<DualRendererApp>,
    single_frame_mode: bool,
    sync_mode: bool,
    frame_rendered: bool,
}

impl App {
    fn with_single_frame_mode(mut self, single_frame_mode: bool) -> Self {
        self.single_frame_mode = single_frame_mode;
        self
    }

    fn with_sync_mode(mut self, sync_mode: bool) -> Self {
        self.sync_mode = sync_mode;
        self
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }

        // Calculate window size based on content
        // 3x2 grid: each cell is 400x300 render + ~80px for text/controls per cell
        // Grid spacing: 10px between cells
        // Top panel: ~50px
        // Side padding: ~20px per side
        let content_width = 400 * 3 + 10 * 2 + 40; // 3 renders + spacing + padding
        let content_height = (300 + 80) * 2 + 10 + 50 + 20; // 2 rows + spacing + top panel + padding

        let window_attributes = Window::default_attributes()
            .with_title("Triple Cube Raytracer - CPU | GL | GPU")
            .with_inner_size(winit::dpi::LogicalSize::new(content_width, content_height));

        let template = ConfigTemplateBuilder::new()
            .with_alpha_size(8)
            .with_transparency(false);

        let display_builder = DisplayBuilder::new().with_window_attributes(Some(window_attributes));

        let (window, gl_config) = display_builder
            .build(event_loop, template, |configs| {
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

        let gl = Arc::new(unsafe {
            Context::from_loader_function_cstr(|s| gl_display.get_proc_address(s))
        });

        // Initialize egui
        let egui_ctx = egui::Context::default();
        let egui_state = egui_winit::State::new(
            egui_ctx.clone(),
            egui::ViewportId::ROOT,
            &window,
            None,
            None,
            None,
        );
        let painter = egui_glow::Painter::new(gl.clone(), "", None, false).unwrap();

        // Initialize dual renderer
        let dual_renderer = unsafe { DualRendererApp::new_with_sync(&gl, self.sync_mode).unwrap() };

        println!("Dual renderer initialized!");
        println!("  - GPU: GlTracer");
        println!("  - CPU: CpuTracer");
        if self.sync_mode {
            println!("  - Sync mode: Enabled (CPU blocks until complete each frame)");
        }

        self.window = Some(window);
        self.gl_context = Some(gl_context);
        self.gl_surface = Some(gl_surface);
        self.gl = Some(gl);
        self.egui_ctx = Some(egui_ctx);
        self.egui_state = Some(egui_state);
        self.painter = Some(painter);
        self.dual_renderer = Some(dual_renderer);
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        event: WindowEvent,
    ) {
        if let Some(egui_state) = &mut self.egui_state {
            let _ = egui_state.on_window_event(self.window.as_ref().unwrap(), &event);
        }

        match event {
            WindowEvent::CloseRequested => {
                event_loop.exit();
            }
            WindowEvent::Resized(size) => {
                if let (Some(gl_surface), Some(gl_context)) =
                    (self.gl_surface.as_ref(), self.gl_context.as_ref())
                {
                    gl_surface.resize(
                        gl_context,
                        NonZeroU32::new(size.width).unwrap(),
                        NonZeroU32::new(size.height).unwrap(),
                    );
                }
                if let Some(window) = &self.window {
                    window.request_redraw();
                }
            }
            WindowEvent::RedrawRequested => {
                if let (
                    Some(window),
                    Some(gl),
                    Some(egui_ctx),
                    Some(egui_state),
                    Some(painter),
                    Some(dual_renderer),
                    Some(gl_context),
                    Some(gl_surface),
                ) = (
                    self.window.as_ref(),
                    self.gl.as_ref(),
                    self.egui_ctx.as_ref(),
                    self.egui_state.as_mut(),
                    self.painter.as_mut(),
                    self.dual_renderer.as_mut(),
                    self.gl_context.as_ref(),
                    self.gl_surface.as_ref(),
                ) {
                    let size = window.inner_size();

                    unsafe {
                        gl.viewport(0, 0, size.width as i32, size.height as i32);
                        gl.clear_color(0.1, 0.1, 0.1, 1.0);
                        gl.clear(COLOR_BUFFER_BIT);
                    }

                    // Run egui
                    let raw_input = egui_state.take_egui_input(window);
                    let full_output = egui_ctx.run(raw_input, |ctx| {
                        dual_renderer.show_ui(ctx, gl);
                    });

                    egui_state.handle_platform_output(window, full_output.platform_output);

                    // Paint egui
                    let clipped_primitives =
                        egui_ctx.tessellate(full_output.shapes, full_output.pixels_per_point);
                    let size_in_pixels = [size.width, size.height];
                    painter.paint_and_update_textures(
                        size_in_pixels,
                        full_output.pixels_per_point,
                        &clipped_primitives,
                        &full_output.textures_delta,
                    );

                    gl_surface.swap_buffers(gl_context).unwrap();

                    // Check if single frame mode
                    if self.single_frame_mode {
                        if !self.frame_rendered {
                            self.frame_rendered = true;
                            println!("\n=== Single frame rendered, exiting ===");
                            event_loop.exit();
                        }
                    } else {
                        window.request_redraw();
                    }
                }
            }
            _ => (),
        }
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        if !self.single_frame_mode
            && let Some(window) = self.window.as_ref()
        {
            window.request_redraw();
        }
    }
}

impl Drop for App {
    fn drop(&mut self) {
        if let (Some(gl), Some(dual_renderer)) = (self.gl.as_ref(), self.dual_renderer.as_mut()) {
            unsafe {
                dual_renderer.destroy(gl);
            }
        }
        if let Some(mut painter) = self.painter.take() {
            painter.destroy();
        }
    }
}

/// Valid renderer names for diff comparison
const VALID_RENDERERS: [&str; 5] = ["cpu", "gl", "bcf", "compute", "mesh"];

/// Parse a renderer name from command line argument
fn parse_renderer_name(name: &str) -> Result<&'static str, String> {
    match name.to_lowercase().as_str() {
        "cpu" => Ok("cpu"),
        "gl" => Ok("gl"),
        "bcf" => Ok("bcf"),
        "compute" | "gpu" => Ok("compute"), // Allow "gpu" as alias for compute
        "mesh" => Ok("mesh"),
        _ => Err(format!("Unknown renderer: '{}'. Valid: cpu, gl, bcf, compute, mesh", name)),
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = std::env::args().collect();

    // Parse command line arguments
    let mut console_mode = false;
    let mut sync_mode = false;
    let mut single_mode = false;
    let mut force_headless = false;
    let mut diff_left = String::from("cpu");
    let mut diff_right = String::from("gl");

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--console" => console_mode = true,
            "--sync" => sync_mode = true,
            "--single" => {
                sync_mode = true;
                single_mode = true;
            }
            "--headless" => force_headless = true,
            "--diff" => {
                // Parse diff arguments: --diff <left> <right> OR --diff all
                if i + 1 >= args.len() {
                    eprintln!("Error: --diff requires arguments");
                    eprintln!("Usage: --diff <left> <right>  (e.g., --diff cpu gl)");
                    eprintln!("       --diff all             (compare all pairs)");
                    eprintln!("Valid renderers: {}", VALID_RENDERERS.join(", "));
                    return Ok(());
                }

                let first_arg = &args[i + 1];

                if first_arg == "all" {
                    diff_left = "all".to_string();
                    diff_right = "all".to_string();
                    i += 1;
                } else {
                    // Parse as <left> <right>
                    match parse_renderer_name(first_arg) {
                        Ok(left) => diff_left = left.to_string(),
                        Err(e) => {
                            eprintln!("Error: {}", e);
                            return Ok(());
                        }
                    }

                    if i + 2 >= args.len() {
                        eprintln!("Error: --diff requires two renderer names");
                        eprintln!("Usage: --diff <left> <right>  (e.g., --diff cpu gl)");
                        eprintln!("Valid renderers: {}", VALID_RENDERERS.join(", "));
                        return Ok(());
                    }

                    let second_arg = &args[i + 2];
                    match parse_renderer_name(second_arg) {
                        Ok(right) => diff_right = right.to_string(),
                        Err(e) => {
                            eprintln!("Error: {}", e);
                            return Ok(());
                        }
                    }

                    if diff_left == diff_right {
                        eprintln!("Error: Cannot diff a renderer with itself");
                        return Ok(());
                    }

                    i += 2;
                }
            }
            "--cpu" => {
                eprintln!("Note: --cpu is deprecated, use --console instead");
                console_mode = true;
            }
            _ => {
                eprintln!("Unknown argument: {}", args[i]);
                eprintln!("Usage: renderer [OPTIONS]");
                eprintln!();
                eprintln!("Options:");
                eprintln!("  --console          Run in console mode (no GUI, batch CPU rendering)");
                eprintln!("  --sync             GUI with synchronized rendering (all tracers use same time/camera)");
                eprintln!("  --single           Render once and exit. Uses GUI if available, headless if not.");
                eprintln!("  --headless         Force headless mode (combine with --single for batch processing)");
                eprintln!("  --diff <L> <R>     Diff comparison between two renderers (default: cpu gl)");
                eprintln!("  --diff all         Compare all renderer pairs");
                eprintln!();
                eprintln!("Renderers: cpu, gl, bcf, compute (or gpu), mesh");
                eprintln!();
                eprintln!("Default: Opens GUI with triple renderer (CPU + GL + Compute side-by-side)");
                eprintln!();
                eprintln!("Examples:");
                eprintln!("  renderer --single                         # GUI: render one frame and exit");
                eprintln!("  renderer --single --headless              # Headless: render, save cpu-gl diff, exit");
                eprintln!("  renderer --single --headless --diff cpu compute  # Headless: render cpu vs compute diff");
                eprintln!("  renderer --single --headless --diff all   # Headless: render all diff pairs");
                eprintln!("  renderer --sync                           # GUI: synchronized continuous rendering");
                return Ok(());
            }
        }
        i += 1;
    }

    if console_mode {
        run_console_renderer()?;
    } else if single_mode {
        if force_headless {
            // --single --headless: batch mode with stats and diffs
            run_sync_renderer_headless(&diff_left, &diff_right)?;
        } else {
            // --single: GUI mode, render once and exit
            run_dual_renderer_sync(true)?;
        }
    } else if sync_mode {
        // --sync: GUI mode with synchronized rendering
        run_dual_renderer_sync(false)?;
    } else {
        run_dual_renderer()?;
    }

    Ok(())
}

fn run_console_renderer() -> Result<(), Box<dyn Error>> {
    println!("Running console renderer (headless, no GUI)...");
    println!("Single-threaded CPU raytracer with synchronized timestamps");

    let mut cpu_renderer = CpuTracer::new();
    println!("Renderer: {}", cpu_renderer.name());

    let width = 800;
    let height = 600;
    let num_frames = 10;

    // Create output directory if it doesn't exist
    std::fs::create_dir_all("output")?;

    for frame in 0..num_frames {
        let time = frame as f32 * 0.1;
        println!("Frame {} (time={:.2}): rendering...", frame, time);

        cpu_renderer.render(width, height, time);

        let filename = format!("output/console_frame_{:03}.png", frame);
        cpu_renderer.save_image(&filename)?;
        println!("  Saved {}", filename);
    }

    println!(
        "\nConsole rendering complete! Generated {} frames",
        num_frames
    );
    println!("Output: output/console_frame_*.png");
    Ok(())
}

fn run_sync_renderer_headless(diff_left: &str, diff_right: &str) -> Result<(), Box<dyn Error>> {
    use ComputeTracer;
    use GlTracer;
    use renderer::CameraConfig;
    // Note: BcfTracer, MeshRenderer not yet supported in headless diff mode

    println!("Running synchronized single-shot renderer...");
    println!("All tracers use same timestamp and camera configuration");
    if diff_left == "all" {
        println!("Diff mode: all pairs");
    } else {
        println!("Diff mode: {} vs {}", diff_left, diff_right);
    }

    // Create camera configuration
    let camera = CameraConfig::look_at(
        glam::Vec3::new(3.0, 2.0, 3.0),
        glam::Vec3::ZERO,
        glam::Vec3::Y,
    );
    let time = 0.0f32;
    let width = 800u32;
    let height = 600u32;

    println!(
        "Camera: pos={:?}, target={:?}",
        camera.position,
        camera.target()
    );
    println!("Resolution: {}x{}", width, height);
    println!("Time: {:.2}s", time);
    println!();

    // === CPU Renderer ===
    println!("[1/3] CPU Tracer (Pure Rust)");
    let mut cpu_renderer = CpuTracer::new();
    let cpu_start = std::time::Instant::now();
    cpu_renderer.render_with_camera(width, height, &camera);
    let cpu_time = cpu_start.elapsed();
    println!("  Render time: {:.2}ms", cpu_time.as_secs_f32() * 1000.0);

    cpu_renderer.save_image("output/sync_cpu.png")?;
    println!("  Saved: output/sync_cpu.png");
    println!();

    // Initialize headless GL context for GL and GPU renderers
    {
        // Initialize headless GL context
        use glutin::config::ConfigTemplateBuilder;
        use glutin::context::{ContextApi, ContextAttributesBuilder, Version};
        use glutin::display::GetGlDisplay;
        use glutin::prelude::*;
        use glutin::surface::{SurfaceAttributesBuilder, WindowSurface};

        // We need to create a minimal window for GL context
        // For true headless, we'd use EGL or OSMesa, but let's use a hidden window
        println!("Note: GL and GPU tracers require OpenGL context (creating minimal window)");

        #[cfg(target_os = "linux")]
        let event_loop = {
            use winit::platform::x11::EventLoopBuilderExtX11;
            let mut builder = EventLoop::builder();
            builder.with_x11();
            builder.build()?
        };

        #[cfg(not(target_os = "linux"))]
        let event_loop = EventLoop::new()?;

        let window_attributes = Window::default_attributes()
            .with_title("Sync Renderer (Hidden)")
            .with_inner_size(winit::dpi::LogicalSize::new(width, height))
            .with_visible(false); // Hide the window

        let template = ConfigTemplateBuilder::new()
            .with_alpha_size(8)
            .with_transparency(false);

        let display_builder =
            glutin_winit::DisplayBuilder::new().with_window_attributes(Some(window_attributes));

        let (window, gl_config) = display_builder.build(&event_loop, template, |configs| {
            configs
                .reduce(|accum, config| {
                    if config.num_samples() > accum.num_samples() {
                        config
                    } else {
                        accum
                    }
                })
                .unwrap()
        })?;

        let window = window.ok_or("Failed to create window")?;
        let window_handle = window.window_handle().ok().map(|h| h.as_raw());
        let gl_display = gl_config.display();

        let context_attributes = ContextAttributesBuilder::new()
            .with_context_api(ContextApi::Gles(Some(Version::new(3, 0))))
            .build(window_handle);

        let gl_context = unsafe { gl_display.create_context(&gl_config, &context_attributes)? };

        let size = window.inner_size();
        let attrs = SurfaceAttributesBuilder::<WindowSurface>::new().build(
            window_handle.unwrap(),
            NonZeroU32::new(size.width).unwrap(),
            NonZeroU32::new(size.height).unwrap(),
        );

        let gl_surface = unsafe { gl_display.create_window_surface(&gl_config, &attrs)? };

        let _gl_context = gl_context.make_current(&gl_surface)?;

        let gl = Arc::new(unsafe {
            glow::Context::from_loader_function_cstr(|s| gl_display.get_proc_address(s))
        });

        // === GL Renderer ===
        println!("[2/3] GL Tracer (WebGL 2.0 Fragment Shader)");
        let mut gl_renderer = GlTracer::new(create_octa_cube());
        unsafe {
            gl_renderer.init_gl(&gl)?;
        }

        // Create framebuffer for GL rendering
        let (gl_fb, gl_tex) = unsafe {
            let texture = gl.create_texture()?;
            gl.bind_texture(glow::TEXTURE_2D, Some(texture));
            gl.tex_image_2d(
                glow::TEXTURE_2D,
                0,
                glow::RGBA as i32,
                width as i32,
                height as i32,
                0,
                glow::RGBA,
                glow::UNSIGNED_BYTE,
                None,
            );
            gl.tex_parameter_i32(
                glow::TEXTURE_2D,
                glow::TEXTURE_MIN_FILTER,
                glow::LINEAR as i32,
            );
            gl.tex_parameter_i32(
                glow::TEXTURE_2D,
                glow::TEXTURE_MAG_FILTER,
                glow::LINEAR as i32,
            );

            let framebuffer = gl.create_framebuffer()?;
            gl.bind_framebuffer(glow::FRAMEBUFFER, Some(framebuffer));
            gl.framebuffer_texture_2d(
                glow::FRAMEBUFFER,
                glow::COLOR_ATTACHMENT0,
                glow::TEXTURE_2D,
                Some(texture),
                0,
            );
            (framebuffer, texture)
        };

        let gl_start = std::time::Instant::now();
        unsafe {
            gl.bind_framebuffer(glow::FRAMEBUFFER, Some(gl_fb));
            gl.viewport(0, 0, width as i32, height as i32);
            gl_renderer.render_to_gl_with_camera(&gl, width as i32, height as i32, &camera);
            gl.finish();
        }
        let gl_time = gl_start.elapsed();
        println!("  Render time: {:.2}ms", gl_time.as_secs_f32() * 1000.0);

        // Read back GL framebuffer
        let gl_pixels = unsafe {
            let mut pixels = vec![0u8; (width * height * 4) as usize];
            gl.bind_framebuffer(glow::FRAMEBUFFER, Some(gl_fb));
            gl.read_pixels(
                0,
                0,
                width as i32,
                height as i32,
                glow::RGBA,
                glow::UNSIGNED_BYTE,
                glow::PixelPackData::Slice(&mut pixels),
            );
            pixels
        };

        // Convert RGBA to RGB and flip Y
        let gl_image = image::ImageBuffer::from_fn(width, height, |x, y| {
            let y_flipped = height - 1 - y;
            let idx = ((y_flipped * width + x) * 4) as usize;
            image::Rgb([gl_pixels[idx], gl_pixels[idx + 1], gl_pixels[idx + 2]])
        });
        gl_image.save("output/sync_gl.png")?;
        println!("  Saved: output/sync_gl.png");
        println!();

        // === GPU Renderer ===
        println!("[3/3] Compute Tracer (Compute Shader)");
        let mut gpu_renderer = ComputeTracer::new(create_octa_cube());
        let mut gpu_time = std::time::Duration::ZERO;
        let gpu_available = unsafe {
            match gpu_renderer.init_gl(&gl) {
                Ok(_) => true,
                Err(e) => {
                    println!("  GPU compute shader not available: {}", e);
                    println!("  Skipping GPU tracer");
                    false
                }
            }
        };

        if gpu_available {
            // Create framebuffer for GPU rendering
            let (gpu_fb, gpu_tex) = unsafe {
                let texture = gl.create_texture()?;
                gl.bind_texture(glow::TEXTURE_2D, Some(texture));
                gl.tex_image_2d(
                    glow::TEXTURE_2D,
                    0,
                    glow::RGBA as i32,
                    width as i32,
                    height as i32,
                    0,
                    glow::RGBA,
                    glow::UNSIGNED_BYTE,
                    None,
                );
                gl.tex_parameter_i32(
                    glow::TEXTURE_2D,
                    glow::TEXTURE_MIN_FILTER,
                    glow::LINEAR as i32,
                );
                gl.tex_parameter_i32(
                    glow::TEXTURE_2D,
                    glow::TEXTURE_MAG_FILTER,
                    glow::LINEAR as i32,
                );

                let framebuffer = gl.create_framebuffer()?;
                gl.bind_framebuffer(glow::FRAMEBUFFER, Some(framebuffer));
                gl.framebuffer_texture_2d(
                    glow::FRAMEBUFFER,
                    glow::COLOR_ATTACHMENT0,
                    glow::TEXTURE_2D,
                    Some(texture),
                    0,
                );
                (framebuffer, texture)
            };

            let gpu_start = std::time::Instant::now();
            unsafe {
                gl.bind_framebuffer(glow::FRAMEBUFFER, Some(gpu_fb));
                gl.viewport(0, 0, width as i32, height as i32);
                gpu_renderer.render_to_gl_with_camera(&gl, width as i32, height as i32, &camera);
                gl.finish();
            }
            gpu_time = gpu_start.elapsed();
            println!("  Render time: {:.2}ms", gpu_time.as_secs_f32() * 1000.0);

            // Read back GPU framebuffer
            let gpu_pixels = unsafe {
                let mut pixels = vec![0u8; (width * height * 4) as usize];
                gl.bind_framebuffer(glow::FRAMEBUFFER, Some(gpu_fb));
                gl.read_pixels(
                    0,
                    0,
                    width as i32,
                    height as i32,
                    glow::RGBA,
                    glow::UNSIGNED_BYTE,
                    glow::PixelPackData::Slice(&mut pixels),
                );
                pixels
            };

            // Convert RGBA to RGB and flip Y
            let gpu_image = image::ImageBuffer::from_fn(width, height, |x, y| {
                let y_flipped = height - 1 - y;
                let idx = ((y_flipped * width + x) * 4) as usize;
                image::Rgb([gpu_pixels[idx], gpu_pixels[idx + 1], gpu_pixels[idx + 2]])
            });
            gpu_image.save("output/sync_gpu.png")?;
            println!("  Saved: output/sync_gpu.png");
            println!();

            // === Diff Image Calculation ===
            println!("[Diff] Calculating difference images...");

            if let Some(cpu_buf) = cpu_renderer.image_buffer() {
                // Helper to generate and save a single diff
                let generate_diff = |name: &str, img1: &image::ImageBuffer<image::Rgb<u8>, Vec<u8>>, img2: &image::ImageBuffer<image::Rgb<u8>, Vec<u8>>| -> Result<(), Box<dyn Error>> {
                    let diff_img = compute_diff_image(img1, img2);
                    let filename = format!("output/diff_{}.png", name);
                    diff_img.save(&filename)?;
                    let (max_diff, avg_diff, pixel_diff_count) = analyze_diff(img1, img2);
                    println!("  {}:", name.to_uppercase().replace('-', " vs "));
                    println!("    Max difference: {} (out of 255)", max_diff);
                    println!("    Avg difference: {:.2}", avg_diff);
                    println!(
                        "    Differing pixels: {} ({:.2}%)",
                        pixel_diff_count,
                        (pixel_diff_count as f32 / (width * height) as f32) * 100.0
                    );
                    println!("    Saved: {}", filename);
                    Ok(())
                };

                // Helper to get image by renderer name
                let get_image = |name: &str| -> Option<&image::ImageBuffer<image::Rgb<u8>, Vec<u8>>> {
                    match name {
                        "cpu" => Some(cpu_buf),
                        "gl" => Some(&gl_image),
                        "compute" => Some(&gpu_image),
                        _ => None,
                    }
                };

                // Generate requested diffs
                if diff_left == "all" {
                    // Generate all pairs
                    generate_diff("cpu-gl", cpu_buf, &gl_image)?;
                    generate_diff("cpu-compute", cpu_buf, &gpu_image)?;
                    generate_diff("gl-compute", &gl_image, &gpu_image)?;
                } else {
                    // Generate single diff
                    let left_img = get_image(diff_left);
                    let right_img = get_image(diff_right);

                    match (left_img, right_img) {
                        (Some(l), Some(r)) => {
                            let name = format!("{}-{}", diff_left, diff_right);
                            generate_diff(&name, l, r)?;
                        }
                        (None, _) => {
                            eprintln!("Warning: Renderer '{}' not available for diff", diff_left);
                        }
                        (_, None) => {
                            eprintln!("Warning: Renderer '{}' not available for diff", diff_right);
                        }
                    }
                }
            }
            println!();

            // Cleanup GPU
            unsafe {
                gl.delete_framebuffer(gpu_fb);
                gl.delete_texture(gpu_tex);
                gpu_renderer.destroy_gl(&gl);
            }
        }

        // Cleanup GL
        unsafe {
            gl.delete_framebuffer(gl_fb);
            gl.delete_texture(gl_tex);
            gl_renderer.destroy_gl(&gl);
        }

        // === Statistics Summary ===
        println!("=== Performance Summary ===");
        println!("CPU Tracer:     {:.2}ms", cpu_time.as_secs_f32() * 1000.0);
        println!("GL Tracer:      {:.2}ms", gl_time.as_secs_f32() * 1000.0);
        if gpu_available {
            println!("Compute Tracer: {:.2}ms", gpu_time.as_secs_f32() * 1000.0);
        } else {
            println!("Compute Tracer: N/A");
        }
        println!();
        println!("All outputs saved to output/ directory");
    }

    Ok(())
}

// Helper function to compute diff image
fn compute_diff_image(
    img1: &image::ImageBuffer<image::Rgb<u8>, Vec<u8>>,
    img2: &image::ImageBuffer<image::Rgb<u8>, Vec<u8>>,
) -> image::ImageBuffer<image::Rgb<u8>, Vec<u8>> {
    assert_eq!(img1.dimensions(), img2.dimensions());
    let (width, height) = img1.dimensions();

    image::ImageBuffer::from_fn(width, height, |x, y| {
        let p1 = img1.get_pixel(x, y);
        let p2 = img2.get_pixel(x, y);

        let r_diff = (p1[0] as i16 - p2[0] as i16).unsigned_abs();
        let g_diff = (p1[1] as i16 - p2[1] as i16).unsigned_abs();
        let b_diff = (p1[2] as i16 - p2[2] as i16).unsigned_abs();

        // Amplify differences for visibility (10x, capped at 255)
        let r_amp = (r_diff * 10).min(255) as u8;
        let g_amp = (g_diff * 10).min(255) as u8;
        let b_amp = (b_diff * 10).min(255) as u8;

        image::Rgb([r_amp, g_amp, b_amp])
    })
}

// Helper function to analyze diff statistics
fn analyze_diff(
    img1: &image::ImageBuffer<image::Rgb<u8>, Vec<u8>>,
    img2: &image::ImageBuffer<image::Rgb<u8>, Vec<u8>>,
) -> (u8, f32, u32) {
    assert_eq!(img1.dimensions(), img2.dimensions());
    let (width, height) = img1.dimensions();

    let mut max_diff = 0u8;
    let mut total_diff = 0u64;
    let mut pixel_diff_count = 0u32;

    for y in 0..height {
        for x in 0..width {
            let p1 = img1.get_pixel(x, y);
            let p2 = img2.get_pixel(x, y);

            let r_diff = (p1[0] as i16 - p2[0] as i16).unsigned_abs();
            let g_diff = (p1[1] as i16 - p2[1] as i16).unsigned_abs();
            let b_diff = (p1[2] as i16 - p2[2] as i16).unsigned_abs();

            let max_channel_diff = r_diff.max(g_diff).max(b_diff) as u8;
            max_diff = max_diff.max(max_channel_diff);
            total_diff += r_diff as u64 + g_diff as u64 + b_diff as u64;

            if r_diff > 0 || g_diff > 0 || b_diff > 0 {
                pixel_diff_count += 1;
            }
        }
    }

    let total_pixels = (width * height) as u64;
    let avg_diff = total_diff as f32 / (total_pixels * 3) as f32;

    (max_diff, avg_diff, pixel_diff_count)
}

fn run_dual_renderer_sync(single_frame_mode: bool) -> Result<(), Box<dyn Error>> {
    if single_frame_mode {
        println!("Running synchronized raytracer (single frame mode)...");
    } else {
        println!("Running synchronized raytracer (all tracers use same timestamp/camera)...");
        println!("CPU renderer will block until complete each frame for true synchronization.");
    }

    run_dual_renderer_with_mode(single_frame_mode, true)
}

fn run_dual_renderer_with_mode(single_frame_mode: bool, sync_mode: bool) -> Result<(), Box<dyn Error>> {
    #[cfg(target_os = "linux")]
    let event_loop = {
        let mut builder = EventLoop::builder();
        // Force X11 backend on Linux (fallback from Wayland)
        use winit::platform::x11::EventLoopBuilderExtX11;
        builder.with_x11();
        builder.build()?
    };

    #[cfg(not(target_os = "linux"))]
    let event_loop = EventLoop::new()?;

    event_loop.set_control_flow(ControlFlow::Poll);

    let mut app = App::default()
        .with_single_frame_mode(single_frame_mode)
        .with_sync_mode(sync_mode);
    event_loop.run_app(&mut app)?;

    Ok(())
}

fn run_dual_renderer() -> Result<(), Box<dyn Error>> {
    println!("Running dual raytracer (GPU + CPU side-by-side)...");
    run_dual_renderer_with_mode(false, false)
}
