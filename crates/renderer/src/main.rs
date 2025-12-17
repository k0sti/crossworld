// Import from the renderer library crate
use renderer::{CpuTracer, Renderer};

// Bin-specific modules
mod egui_app;

use egui_app::CubeRendererApp;

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
    cube_renderer: Option<CubeRendererApp>,
    single_frame_mode: bool,
    sync_mode: bool,
    frame_rendered: bool,
    diff_left: String,
    diff_right: String,
    model_name: Option<String>,
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

    fn with_diff_sources(mut self, diff_left: &str, diff_right: &str) -> Self {
        self.diff_left = diff_left.to_string();
        self.diff_right = diff_right.to_string();
        self
    }

    fn with_model(mut self, model_name: Option<String>) -> Self {
        self.model_name = model_name;
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
            .with_title("Cube Renderer - CPU | GL | BCF | Compute | Mesh")
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

        // Request OpenGL 4.3 for compute shader support (GPU tracer requires it)
        // Falls back to lower versions if 4.3 is not available
        let context_attributes = ContextAttributesBuilder::new()
            .with_context_api(ContextApi::OpenGl(Some(Version::new(4, 3))))
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

        // Initialize cube renderer
        let mut cube_renderer =
            unsafe { CubeRendererApp::new_with_sync(&gl, self.sync_mode, self.model_name.as_deref()).unwrap() };

        // Set diff sources from CLI arguments
        if !self.diff_left.is_empty() && !self.diff_right.is_empty() {
            cube_renderer.set_diff_sources(&self.diff_left, &self.diff_right);
        }

        println!("Cube renderer initialized!");
        println!("  - CPU: CpuTracer");
        println!("  - GL: GlTracer");
        println!("  - BCF: BcfTracer");
        println!("  - Compute: ComputeTracer");
        println!("  - Mesh: MeshRenderer");
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
        self.cube_renderer = Some(cube_renderer);
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
                    Some(cube_renderer),
                    Some(gl_context),
                    Some(gl_surface),
                ) = (
                    self.window.as_ref(),
                    self.gl.as_ref(),
                    self.egui_ctx.as_ref(),
                    self.egui_state.as_mut(),
                    self.painter.as_mut(),
                    self.cube_renderer.as_mut(),
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
                        cube_renderer.show_ui(ctx, gl);
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

                            // Save diff image output
                            if let Some(ref cube_renderer) = self.cube_renderer {
                                // Create output directory
                                if let Err(e) = std::fs::create_dir_all("output") {
                                    eprintln!("Warning: Failed to create output directory: {}", e);
                                }

                                // Save all individual frames for debugging
                                if let Err(e) = cube_renderer.save_all_frames("output") {
                                    eprintln!("Warning: Failed to save all frames: {}", e);
                                }

                                match cube_renderer.save_diff_image("output") {
                                    Ok(path) => {
                                        println!("\n=== Diff image saved ===");
                                        println!("Output: {}", path);
                                    }
                                    Err(e) => {
                                        eprintln!("Warning: Failed to save diff image: {}", e);
                                    }
                                }
                            }

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
        if let (Some(gl), Some(cube_renderer)) = (self.gl.as_ref(), self.cube_renderer.as_mut()) {
            unsafe {
                cube_renderer.destroy(gl);
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
        _ => Err(format!(
            "Unknown renderer: '{}'. Valid: cpu, gl, bcf, compute, mesh",
            name
        )),
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = std::env::args().collect();

    // Parse command line arguments
    let mut console_mode = false;
    let mut sync_mode = false;
    let mut single_mode = false;
    let mut diff_left = String::from("cpu");
    let mut diff_right = String::from("gl");
    let mut model_name: Option<String> = None;

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--console" => console_mode = true,
            "--sync" => sync_mode = true,
            "--single" => {
                sync_mode = true;
                single_mode = true;
            }
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
            "--model" => {
                if i + 1 >= args.len() {
                    eprintln!("Error: --model requires a model name");
                    eprintln!("Usage: --model <name>  (e.g., --model octa)");
                    eprintln!("Available models: single, octa, extended, depth3, quad, layer, sdf, generated, test_expansion, vox_robot, vox_alien_bot, vox_eskimo");
                    return Ok(());
                }
                model_name = Some(args[i + 1].clone());
                i += 1;
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
                eprintln!(
                    "  --sync             GUI with synchronized rendering (all tracers use same time/camera)"
                );
                eprintln!("  --single           Render one frame and exit");
                eprintln!(
                    "  --diff <L> <R>     Set initial diff comparison renderers (default: cpu gl)"
                );
                eprintln!("  --diff all         Compare all renderer pairs in diff view");
                eprintln!("  --model <name>     Select test model to render (default: octa)");
                eprintln!();
                eprintln!("Renderers: cpu, gl, bcf, compute (or gpu), mesh");
                eprintln!("Models: single, octa, extended, depth3, quad, layer, sdf, generated, test_expansion, vox_robot, vox_alien_bot, vox_eskimo");
                eprintln!();
                eprintln!(
                    "Default: Opens GUI with 5 renderers (CPU + GL + BCF + Compute + Mesh)"
                );
                eprintln!();
                eprintln!("Examples:");
                eprintln!(
                    "  renderer --single                    # Render one frame and exit"
                );
                eprintln!(
                    "  renderer --single --diff cpu mesh    # Render one frame with cpu vs mesh diff"
                );
                eprintln!(
                    "  renderer --sync                      # Synchronized continuous rendering"
                );
                return Ok(());
            }
        }
        i += 1;
    }

    if console_mode {
        run_console_renderer()?;
    } else if single_mode {
        // --single: GUI mode, render once and exit
        run_cube_renderer_sync(true, &diff_left, &diff_right, model_name.as_deref())?;
    } else if sync_mode {
        // --sync: GUI mode with synchronized rendering
        run_cube_renderer_sync(false, &diff_left, &diff_right, model_name.as_deref())?;
    } else {
        run_cube_renderer(model_name.as_deref())?;
    }

    Ok(())
}

fn run_console_renderer() -> Result<(), Box<dyn Error>> {
    println!("Running console renderer (CPU-only, batch mode)...");
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

fn run_cube_renderer_sync(
    single_frame_mode: bool,
    diff_left: &str,
    diff_right: &str,
    model_name: Option<&str>,
) -> Result<(), Box<dyn Error>> {
    if single_frame_mode {
        println!("Running cube renderer (single frame mode)...");
    } else {
        println!("Running cube renderer (all tracers use same timestamp/camera)...");
        println!("CPU renderer will block until complete each frame for true synchronization.");
    }

    run_cube_renderer_with_mode(single_frame_mode, true, diff_left, diff_right, model_name)
}

fn run_cube_renderer_with_mode(
    single_frame_mode: bool,
    sync_mode: bool,
    diff_left: &str,
    diff_right: &str,
    model_name: Option<&str>,
) -> Result<(), Box<dyn Error>> {
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
        .with_sync_mode(sync_mode)
        .with_diff_sources(diff_left, diff_right)
        .with_model(model_name.map(String::from));
    event_loop.run_app(&mut app)?;

    Ok(())
}

fn run_cube_renderer(model_name: Option<&str>) -> Result<(), Box<dyn Error>> {
    println!("Running cube renderer (5 tracers side-by-side)...");
    run_cube_renderer_with_mode(false, false, "cpu", "gl", model_name)
}
