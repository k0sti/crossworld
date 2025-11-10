use glow::*;
use glutin::config::ConfigTemplateBuilder;
use glutin::context::{ContextApi, ContextAttributesBuilder, Version};
use glutin::display::GetGlDisplay;
use glutin::prelude::*;
use glutin::surface::{SurfaceAttributesBuilder, WindowSurface};
use glutin_winit::DisplayBuilder;
use raw_window_handle::HasWindowHandle;
use std::num::NonZeroU32;
use std::sync::Arc;
use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::window::{Window, WindowId};

#[cfg(target_os = "linux")]
use winit::platform::x11::EventLoopBuilderExtX11;

struct SimpleApp {
    window: Option<Window>,
    gl_context: Option<glutin::context::PossiblyCurrentContext>,
    gl_surface: Option<glutin::surface::Surface<WindowSurface>>,
    gl: Option<Arc<Context>>,
    egui_ctx: Option<egui::Context>,
    egui_state: Option<egui_winit::State>,
    painter: Option<egui_glow::Painter>,

    // Simple animated color
    time: f32,

    // GL resources for a simple triangle
    program: Option<glow::Program>,
    vao: Option<glow::VertexArray>,
}

impl Default for SimpleApp {
    fn default() -> Self {
        Self {
            window: None,
            gl_context: None,
            gl_surface: None,
            gl: None,
            egui_ctx: None,
            egui_state: None,
            painter: None,
            time: 0.0,
            program: None,
            vao: None,
        }
    }
}

impl SimpleApp {
    unsafe fn create_gl_resources(gl: &Context) -> (glow::Program, glow::VertexArray) {
        // Simple vertex shader - draws a triangle using vertex ID
        let vertex_shader_src = r#"#version 300 es
        precision mediump float;

        void main() {
            // Create a triangle using gl_VertexID
            vec2 pos[3] = vec2[3](
                vec2(-0.6, -0.6),
                vec2(0.6, -0.6),
                vec2(0.0, 0.6)
            );
            gl_Position = vec4(pos[gl_VertexID], 0.0, 1.0);
        }
        "#;

        // Simple fragment shader - rainbow colors
        let fragment_shader_src = r#"#version 300 es
        precision mediump float;

        uniform float u_time;
        out vec4 fragColor;

        void main() {
            float r = sin(u_time) * 0.5 + 0.5;
            float g = sin(u_time * 0.7) * 0.5 + 0.5;
            float b = sin(u_time * 1.3) * 0.5 + 0.5;
            fragColor = vec4(r, g, b, 1.0);
        }
        "#;

        let program = gl.create_program().unwrap();

        let vertex_shader = gl.create_shader(VERTEX_SHADER).unwrap();
        gl.shader_source(vertex_shader, vertex_shader_src);
        gl.compile_shader(vertex_shader);
        if !gl.get_shader_compile_status(vertex_shader) {
            panic!("Vertex shader error: {}", gl.get_shader_info_log(vertex_shader));
        }

        let fragment_shader = gl.create_shader(FRAGMENT_SHADER).unwrap();
        gl.shader_source(fragment_shader, fragment_shader_src);
        gl.compile_shader(fragment_shader);
        if !gl.get_shader_compile_status(fragment_shader) {
            panic!("Fragment shader error: {}", gl.get_shader_info_log(fragment_shader));
        }

        gl.attach_shader(program, vertex_shader);
        gl.attach_shader(program, fragment_shader);
        gl.link_program(program);
        if !gl.get_program_link_status(program) {
            panic!("Program link error: {}", gl.get_program_info_log(program));
        }

        gl.delete_shader(vertex_shader);
        gl.delete_shader(fragment_shader);

        // Create VAO
        let vao = gl.create_vertex_array().unwrap();

        (program, vao)
    }
}

impl ApplicationHandler for SimpleApp {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }

        let window_attributes = Window::default_attributes()
            .with_title("Simple egui + OpenGL Test")
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

        let gl = Arc::new(unsafe {
            Context::from_loader_function_cstr(|s| gl_display.get_proc_address(s))
        });

        // Create GL resources for triangle rendering
        let (program, vao) = unsafe { Self::create_gl_resources(&gl) };

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

        println!("Simple egui + OpenGL app initialized!");

        self.window = Some(window);
        self.gl_context = Some(gl_context);
        self.gl_surface = Some(gl_surface);
        self.gl = Some(gl);
        self.egui_ctx = Some(egui_ctx);
        self.egui_state = Some(egui_state);
        self.painter = Some(painter);
        self.program = Some(program);
        self.vao = Some(vao);
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        event: WindowEvent,
    ) {
        if let Some(egui_state) = &mut self.egui_state {
            let _ = egui_state.on_window_event(&self.window.as_ref().unwrap(), &event);
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
                if let (Some(window), Some(gl), Some(egui_ctx), Some(egui_state), Some(painter), Some(gl_context), Some(gl_surface)) = (
                    self.window.as_ref(),
                    self.gl.as_ref(),
                    self.egui_ctx.as_ref(),
                    self.egui_state.as_mut(),
                    self.painter.as_mut(),
                    self.gl_context.as_ref(),
                    self.gl_surface.as_ref(),
                ) {
                    let size = window.inner_size();

                    // Animate time
                    self.time += 0.016;
                    let r = (self.time.sin() * 0.5 + 0.5) as f32;
                    let g = ((self.time * 0.7).sin() * 0.5 + 0.5) as f32;
                    let b = ((self.time * 1.3).sin() * 0.5 + 0.5) as f32;

                    unsafe {
                        gl.viewport(0, 0, size.width as i32, size.height as i32);
                        gl.clear_color(r * 0.2, g * 0.2, b * 0.2, 1.0);
                        gl.clear(COLOR_BUFFER_BIT);

                        // Draw a triangle with OpenGL
                        if let (Some(program), Some(vao)) = (self.program, self.vao) {
                            gl.use_program(Some(program));

                            // Set time uniform
                            if let Some(location) = gl.get_uniform_location(program, "u_time") {
                                gl.uniform_1_f32(Some(&location), self.time);
                            }

                            gl.bind_vertex_array(Some(vao));
                            gl.draw_arrays(TRIANGLES, 0, 3);
                            gl.bind_vertex_array(None);
                        }
                    }

                    // Run egui
                    let raw_input = egui_state.take_egui_input(window);
                    let full_output = egui_ctx.run(raw_input, |ctx| {
                        egui::CentralPanel::default().show(ctx, |ui| {
                            ui.heading("Simple egui + OpenGL Test");
                            ui.separator();
                            ui.label(format!("Time: {:.2}s", self.time));
                            ui.label(format!("Background RGB: ({:.2}, {:.2}, {:.2})", r, g, b));
                            ui.separator();
                            ui.label("✓ egui is working!");
                            ui.label("✓ OpenGL triangle should be visible behind this panel");

                            ui.add_space(20.0);

                            if ui.button("Click me!").clicked() {
                                println!("Button clicked at time {:.2}", self.time);
                            }

                            ui.add_space(20.0);
                            ui.label("The background color is animated.");
                            ui.label("The triangle color is also animated.");
                        });
                    });

                    egui_state.handle_platform_output(window, full_output.platform_output);

                    // Paint egui
                    let clipped_primitives = egui_ctx.tessellate(full_output.shapes, full_output.pixels_per_point);
                    let size_in_pixels = [size.width, size.height];
                    painter.paint_and_update_textures(size_in_pixels, full_output.pixels_per_point, &clipped_primitives, &full_output.textures_delta);

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

impl Drop for SimpleApp {
    fn drop(&mut self) {
        if let Some(mut painter) = self.painter.take() {
            painter.destroy();
        }
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Starting simple egui + OpenGL test...");

    #[cfg(target_os = "linux")]
    let event_loop = {
        let mut builder = EventLoop::builder();
        builder.with_x11();
        builder.build()?
    };

    #[cfg(not(target_os = "linux"))]
    let event_loop = EventLoop::new()?;

    event_loop.set_control_flow(ControlFlow::Poll);

    let mut app = SimpleApp::default();
    event_loop.run_app(&mut app)?;

    Ok(())
}
