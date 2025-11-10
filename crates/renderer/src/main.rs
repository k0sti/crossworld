mod cpu_tracer;
mod egui_app;
mod gl_tracer;
mod gpu_tracer;
mod renderer;

use cpu_tracer::CpuCubeTracer;
use egui_app::DualRendererApp;
use renderer::Renderer;

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
    single_frame: bool,
    frame_rendered: bool,
}

impl App {
    fn with_single_frame(mut self, single_frame: bool) -> Self {
        self.single_frame = single_frame;
        self
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }

        let window_attributes = Window::default_attributes()
            .with_title("Dual Cube Raytracer - GPU vs CPU")
            .with_inner_size(winit::dpi::LogicalSize::new(1000, 700));

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
        let dual_renderer = unsafe { DualRendererApp::new(&gl).unwrap() };

        println!("Dual renderer initialized!");
        println!("  - GPU: GlCubeTracer");
        println!("  - CPU: CpuCubeTracer");

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
                    if self.single_frame {
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
        if !self.single_frame
            && let Some(window) = self.window.as_ref()
        {
            window.request_redraw();
        }
    }
}

impl Drop for App {
    fn drop(&mut self) {
        if let (Some(gl), Some(dual_renderer)) = (self.gl.as_ref(), self.dual_renderer.as_ref()) {
            unsafe {
                dual_renderer.destroy(gl);
            }
        }
        if let Some(mut painter) = self.painter.take() {
            painter.destroy();
        }
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = std::env::args().collect();

    // Parse command line arguments
    let mut cpu_only = false;
    let mut single_frame = false;

    for arg in args.iter().skip(1) {
        match arg.as_str() {
            "--cpu" => cpu_only = true,
            "--single-frame" => single_frame = true,
            _ => {
                eprintln!("Unknown argument: {}", arg);
                eprintln!("Usage: renderer [--cpu] [--single-frame]");
                eprintln!("  --cpu          Run CPU renderer only (batch mode)");
                eprintln!("  --single-frame Render one frame and exit (for debugging)");
                return Ok(());
            }
        }
    }

    if cpu_only {
        run_cpu_renderer()?;
    } else {
        run_dual_renderer(single_frame)?;
    }

    Ok(())
}

fn run_cpu_renderer() -> Result<(), Box<dyn Error>> {
    println!("Running CPU raytracer (batch mode)...");

    let mut cpu_renderer = CpuCubeTracer::new();
    println!("Renderer: {}", cpu_renderer.name());

    let width = 800;
    let height = 600;
    let num_frames = 10;

    for frame in 0..num_frames {
        let time = frame as f32 * 0.1;
        println!("Rendering frame {} at time {:.2}...", frame, time);

        cpu_renderer.render(width, height, time);

        // Save the frame
        let filename = format!("output_frame_{:03}.png", frame);
        cpu_renderer.save_image(&filename)?;
        println!("Saved {}", filename);
    }

    println!("CPU rendering complete! Generated {} frames", num_frames);
    Ok(())
}

fn run_dual_renderer(single_frame: bool) -> Result<(), Box<dyn Error>> {
    if single_frame {
        println!("Running dual raytracer (single frame mode for debugging)...");
    } else {
        println!("Running dual raytracer (GPU + CPU side-by-side)...");
    }

    #[cfg(target_os = "linux")]
    let event_loop = {
        let mut builder = EventLoop::builder();
        // Force X11 backend on Linux (fallback from Wayland)
        builder.with_x11();
        builder.build()?
    };

    #[cfg(not(target_os = "linux"))]
    let event_loop = EventLoop::new()?;

    event_loop.set_control_flow(ControlFlow::Poll);

    let mut app = App::default().with_single_frame(single_frame);
    event_loop.run_app(&mut app)?;

    Ok(())
}
