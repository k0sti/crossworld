use crate::cpu_tracer::CpuCubeTracer;
use crate::gl_tracer::GlCubeTracer;
use crate::renderer::Renderer;
use egui::{ColorImage, TextureHandle, TextureOptions};
use glow::*;
use std::sync::Arc;

pub struct DualRendererApp {
    gl_renderer: GlCubeTracer,
    cpu_renderer: CpuCubeTracer,

    // GL texture for rendering
    gl_framebuffer: Option<Framebuffer>,
    gl_texture: Option<Texture>,
    gl_texture_size: (i32, i32),

    // egui textures
    gl_egui_texture: Option<TextureHandle>,
    cpu_texture: Option<TextureHandle>,

    // Timing
    start_time: std::time::Instant,
    frame_count: u64,
    last_fps_update: std::time::Instant,
    fps: f32,

    // Settings
    render_size: (u32, u32),
}

impl DualRendererApp {
    pub unsafe fn new(gl: &Arc<Context>) -> Result<Self, String> {
        let gl_renderer = unsafe { GlCubeTracer::new(gl)? };
        let cpu_renderer = CpuCubeTracer::new();

        let render_size = (400, 300);

        Ok(Self {
            gl_renderer,
            cpu_renderer,
            gl_framebuffer: None,
            gl_texture: None,
            gl_texture_size: (0, 0),
            gl_egui_texture: None,
            cpu_texture: None,
            start_time: std::time::Instant::now(),
            frame_count: 0,
            last_fps_update: std::time::Instant::now(),
            fps: 0.0,
            render_size,
        })
    }

    unsafe fn ensure_gl_framebuffer(&mut self, gl: &Arc<Context>, width: i32, height: i32) {
        if self.gl_texture_size != (width, height) || self.gl_framebuffer.is_none() {
            unsafe {
                // Clean up old resources
                if let Some(fb) = self.gl_framebuffer {
                    gl.delete_framebuffer(fb);
            }
            if let Some(tex) = self.gl_texture {
                gl.delete_texture(tex);
            }

            // Create texture (use RGBA for better compatibility)
            let texture = gl.create_texture().unwrap();
            gl.bind_texture(TEXTURE_2D, Some(texture));
            gl.tex_image_2d(
                TEXTURE_2D,
                0,
                RGBA as i32,
                width,
                height,
                0,
                RGBA,
                UNSIGNED_BYTE,
                None,
            );
            gl.tex_parameter_i32(TEXTURE_2D, TEXTURE_MIN_FILTER, LINEAR as i32);
            gl.tex_parameter_i32(TEXTURE_2D, TEXTURE_MAG_FILTER, LINEAR as i32);

            // Create framebuffer
            let framebuffer = gl.create_framebuffer().unwrap();
            gl.bind_framebuffer(FRAMEBUFFER, Some(framebuffer));
            gl.framebuffer_texture_2d(
                FRAMEBUFFER,
                COLOR_ATTACHMENT0,
                TEXTURE_2D,
                Some(texture),
                0,
            );

            // Set draw buffer
            gl.draw_buffers(&[COLOR_ATTACHMENT0]);

            // Check framebuffer status
            let status = gl.check_framebuffer_status(FRAMEBUFFER);
            if status != FRAMEBUFFER_COMPLETE {
                panic!("Framebuffer not complete: {:?}", status);
            }

            gl.bind_framebuffer(FRAMEBUFFER, None);

                self.gl_texture = Some(texture);
                self.gl_framebuffer = Some(framebuffer);
                self.gl_texture_size = (width, height);
            }
        }
    }

    pub unsafe fn render_gl_to_texture(&mut self, gl: &Arc<Context>, time: f32) {
        let (width, height) = self.render_size;
        unsafe {
            self.ensure_gl_framebuffer(gl, width as i32, height as i32);

            // Render to framebuffer
            gl.bind_framebuffer(FRAMEBUFFER, self.gl_framebuffer);
            gl.viewport(0, 0, width as i32, height as i32);
            self.gl_renderer.render_to_gl(gl, width as i32, height as i32, time);
            gl.bind_framebuffer(FRAMEBUFFER, None);
        }
    }

    pub unsafe fn read_gl_texture_to_image(&self, gl: &Arc<Context>) -> Option<ColorImage> {
        if self.gl_framebuffer.is_none() {
            return None;
        }

        let (width, height) = self.render_size;
        let mut pixels = vec![0u8; (width * height * 4) as usize];

        unsafe {
            // Bind framebuffer and read pixels
            gl.bind_framebuffer(FRAMEBUFFER, self.gl_framebuffer);
            gl.read_pixels(
                0,
                0,
                width as i32,
                height as i32,
                RGBA,
                UNSIGNED_BYTE,
                glow::PixelPackData::Slice(&mut pixels),
            );
            gl.bind_framebuffer(FRAMEBUFFER, None);
        }

        // Convert to egui ColorImage (flip Y because OpenGL is bottom-up)
        let mut egui_pixels = Vec::with_capacity((width * height) as usize);
        for y in (0..height).rev() {
            for x in 0..width {
                let idx = ((y * width + x) * 4) as usize;
                egui_pixels.push(egui::Color32::from_rgba_premultiplied(
                    pixels[idx],
                    pixels[idx + 1],
                    pixels[idx + 2],
                    pixels[idx + 3],
                ));
            }
        }

        Some(ColorImage {
            size: [width as usize, height as usize],
            pixels: egui_pixels,
        })
    }

    pub fn render_cpu(&mut self, time: f32) {
        let (width, height) = self.render_size;
        self.cpu_renderer.render(width, height, time);
    }

    pub fn update_fps(&mut self) {
        self.frame_count += 1;
        let now = std::time::Instant::now();
        let elapsed = now.duration_since(self.last_fps_update).as_secs_f32();

        if elapsed >= 0.5 {
            self.fps = self.frame_count as f32 / elapsed;
            self.frame_count = 0;
            self.last_fps_update = now;
        }
    }

    pub fn show_ui(&mut self, ctx: &egui::Context, gl: &Arc<Context>) {
        let time = self.start_time.elapsed().as_secs_f32();

        // Render both
        unsafe {
            self.render_gl_to_texture(gl, time);
        }
        self.render_cpu(time);
        self.update_fps();

        // Top panel with info
        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.heading("Dual Cube Raytracer");
                ui.separator();
                ui.label(format!("FPS: {:.1}", self.fps));
                ui.separator();
                ui.label(format!("Time: {:.2}s", time));
                ui.separator();
                ui.label(format!("Resolution: {}x{}", self.render_size.0, self.render_size.1));
            });
        });

        // Main panel with side-by-side views
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.columns(2, |columns| {
                // Left: GL Renderer
                columns[0].vertical_centered(|ui| {
                    ui.heading("GPU Raytracer (OpenGL)");
                    ui.label("Fragment shader, real-time");
                    ui.add_space(10.0);

                    // Read GL framebuffer and convert to egui texture
                    unsafe {
                        if let Some(color_image) = self.read_gl_texture_to_image(gl) {
                            // Recreate texture each frame
                            self.gl_egui_texture = Some(ctx.load_texture(
                                "gl_render",
                                color_image,
                                TextureOptions::LINEAR,
                            ));

                            if let Some(texture) = &self.gl_egui_texture {
                                ui.image(egui::ImageSource::Texture(egui::load::SizedTexture {
                                    id: texture.id(),
                                    size: [self.render_size.0 as f32, self.render_size.1 as f32].into(),
                                }));
                            }
                        } else {
                            ui.label("GL texture not available");
                        }
                    }
                });

                // Right: CPU Renderer
                columns[1].vertical_centered(|ui| {
                    ui.heading("CPU Raytracer (Pure Rust)");
                    ui.label("Software rendering, single-threaded");
                    ui.add_space(10.0);

                    // Convert CPU image to egui texture
                    if let Some(image_buffer) = self.cpu_renderer.image_buffer() {
                        let (width, height) = (image_buffer.width() as usize, image_buffer.height() as usize);

                        // Convert to egui ColorImage
                        let pixels: Vec<egui::Color32> = image_buffer
                            .pixels()
                            .map(|p| egui::Color32::from_rgb(p[0], p[1], p[2]))
                            .collect();

                        let color_image = ColorImage {
                            size: [width, height],
                            pixels,
                        };

                        // Recreate texture each frame
                        self.cpu_texture = Some(ctx.load_texture(
                            "cpu_render",
                            color_image,
                            TextureOptions::LINEAR,
                        ));

                        if let Some(texture) = &self.cpu_texture {
                            ui.image(egui::ImageSource::Texture(egui::load::SizedTexture {
                                id: texture.id(),
                                size: [self.render_size.0 as f32, self.render_size.1 as f32].into(),
                            }));
                        }
                    }
                });
            });
        });
    }

    pub unsafe fn destroy(&self, gl: &Arc<Context>) {
        unsafe {
            if let Some(fb) = self.gl_framebuffer {
                gl.delete_framebuffer(fb);
            }
            if let Some(tex) = self.gl_texture {
                gl.delete_texture(tex);
            }
            self.gl_renderer.destroy(gl);
        }
    }
}
