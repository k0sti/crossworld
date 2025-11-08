mod cpu_tracer;
mod gl_tracer;
mod renderer;

use cpu_tracer::CpuCubeTracer;
use gl_tracer::GlCubeTracer;
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
use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::window::{Window, WindowId};

#[cfg(target_os = "linux")]
use winit::platform::x11::EventLoopBuilderExtX11;

struct App {
    window: Option<Window>,
    gl_context: Option<glutin::context::PossiblyCurrentContext>,
    gl_surface: Option<glutin::surface::Surface<WindowSurface>>,
    gl: Option<Context>,
    gl_renderer: Option<GlCubeTracer>,
    start_time: std::time::Instant,
}

impl Default for App {
    fn default() -> Self {
        Self {
            window: None,
            gl_context: None,
            gl_surface: None,
            gl: None,
            gl_renderer: None,
            start_time: std::time::Instant::now(),
        }
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }

        let window_attributes = Window::default_attributes()
            .with_title("Cube Raytracer - GL & CPU")
            .with_inner_size(winit::dpi::LogicalSize::new(800, 600));

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

        let gl = unsafe {
            Context::from_loader_function_cstr(|s| gl_display.get_proc_address(s))
        };

        let gl_renderer = unsafe { GlCubeTracer::new(&gl).unwrap() };

        println!("Renderer initialized: {}", gl_renderer.name());

        self.window = Some(window);
        self.gl_context = Some(gl_context);
        self.gl_surface = Some(gl_surface);
        self.gl = Some(gl);
        self.gl_renderer = Some(gl_renderer);
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        event: WindowEvent,
    ) {
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
            }
            WindowEvent::RedrawRequested => {
                if let (Some(window), Some(gl), Some(renderer), Some(gl_context), Some(gl_surface)) = (
                    self.window.as_ref(),
                    self.gl.as_ref(),
                    self.gl_renderer.as_ref(),
                    self.gl_context.as_ref(),
                    self.gl_surface.as_ref(),
                ) {
                    let size = window.inner_size();
                    let time = self.start_time.elapsed().as_secs_f32();

                    unsafe {
                        gl.viewport(0, 0, size.width as i32, size.height as i32);
                        renderer.render_to_gl(gl, size.width as i32, size.height as i32, time);
                    }

                    gl_surface.swap_buffers(gl_context).unwrap();
                    window.request_redraw();
                }
            }
            _ => (),
        }
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        if let Some(window) = self.window.as_ref() {
            window.request_redraw();
        }
    }
}

impl Drop for App {
    fn drop(&mut self) {
        if let (Some(gl), Some(renderer)) = (self.gl.as_ref(), self.gl_renderer.as_ref()) {
            unsafe {
                renderer.destroy(gl);
            }
        }
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = std::env::args().collect();

    // Check if we should run CPU renderer
    if args.len() > 1 && args[1] == "--cpu" {
        run_cpu_renderer()?;
    } else {
        run_gl_renderer()?;
    }

    Ok(())
}

fn run_cpu_renderer() -> Result<(), Box<dyn Error>> {
    println!("Running CPU raytracer...");

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

fn run_gl_renderer() -> Result<(), Box<dyn Error>> {
    println!("Running GL raytracer...");

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

    let mut app = App::default();
    event_loop.run_app(&mut app)?;

    Ok(())
}
