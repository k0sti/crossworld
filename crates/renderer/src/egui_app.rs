use crate::cpu_tracer::CpuCubeTracer;
use crate::gpu_tracer::GpuTracer;
use crate::renderer::{CameraConfig, Renderer};
use crate::scenes::create_octa_cube;
use egui::{ColorImage, TextureHandle, TextureOptions};
use glow::*;
use std::sync::{Arc, Mutex};
use std::thread;

// Message to send to CPU renderer thread
struct RenderRequest {
    width: u32,
    height: u32,
    time: f32,
    camera: Option<CameraConfig>,
}

// Response from CPU renderer thread
struct RenderResponse {
    image: image::ImageBuffer<image::Rgb<u8>, Vec<u8>>,
    render_time_ms: f32,
}

pub struct DualRendererApp {
    gpu_renderer: GpuTracer,

    // CPU renderer runs in separate thread
    // Only stores the latest sync frame request, drops old ones
    cpu_sync_request: Arc<Mutex<Option<RenderRequest>>>,
    cpu_sync_response: Arc<Mutex<Option<RenderResponse>>>,

    // GL texture for rendering
    gl_framebuffer: Option<Framebuffer>,
    gl_texture: Option<Texture>,
    gl_texture_size: (i32, i32),

    // egui textures
    gl_egui_texture: Option<TextureHandle>,
    cpu_texture: Option<TextureHandle>,
    diff_texture: Option<TextureHandle>,

    // Timing
    start_time: std::time::Instant,
    frame_count: u64,
    last_fps_update: std::time::Instant,
    fps: f32,
    gl_render_time_ms: f32,
    cpu_render_time_ms: f32,

    // Settings
    render_size: (u32, u32),

    // Camera control
    camera: CameraConfig,
    camera_target: glam::Vec3, // Point camera orbits around
    use_manual_camera: bool,
    mouse_sensitivity: f32,
    zoom_sensitivity: f32,

    // Current display frames (updated every frame)
    gl_latest_frame: Option<ColorImage>,
    cpu_latest_frame: Option<ColorImage>,
}

impl DualRendererApp {
    pub unsafe fn new(gl: &Arc<Context>) -> Result<Self, String> {
        // Create octa cube scene
        let cube = create_octa_cube();
        let mut gpu_renderer = GpuTracer::new(cube);

        // Initialize GL resources for GPU renderer
        unsafe {
            gpu_renderer.init_gl(gl)?;
        }

        let render_size = (400, 300);

        // Initialize camera looking at the cube (origin)
        let camera_target = glam::Vec3::ZERO;
        let camera =
            CameraConfig::look_at(glam::Vec3::new(3.0, 2.0, 3.0), camera_target, glam::Vec3::Y);

        // Create shared Mutex slots for CPU renderer thread communication
        let cpu_sync_request = Arc::new(Mutex::new(None));
        let cpu_sync_response = Arc::new(Mutex::new(None));

        let request_clone = Arc::clone(&cpu_sync_request);
        let response_clone = Arc::clone(&cpu_sync_response);

        thread::spawn(move || {
            let mut cpu_renderer = CpuCubeTracer::new();

            loop {
                // Check for new render request (non-blocking)
                let request: Option<RenderRequest> = {
                    let mut req_lock = request_clone.lock().unwrap();
                    req_lock.take() // Take the request, leaving None
                };

                if let Some(request) = request {
                    let start = std::time::Instant::now();

                    // Render based on whether we have a manual camera or not
                    if let Some(camera) = request.camera {
                        cpu_renderer.render_with_camera(request.width, request.height, &camera);
                    } else {
                        cpu_renderer.render(request.width, request.height, request.time);
                    }

                    let render_time = start.elapsed();

                    // Store result in response slot
                    if let Some(image) = cpu_renderer.image_buffer() {
                        let response = RenderResponse {
                            image: image.clone(),
                            render_time_ms: render_time.as_secs_f32() * 1000.0,
                        };
                        *response_clone.lock().unwrap() = Some(response);
                    }
                } else {
                    // No request available, sleep briefly to avoid busy-waiting
                    std::thread::sleep(std::time::Duration::from_millis(10));
                }
            }
        });

        Ok(Self {
            gpu_renderer,
            cpu_sync_request,
            cpu_sync_response,
            gl_framebuffer: None,
            gl_texture: None,
            gl_texture_size: (0, 0),
            gl_egui_texture: None,
            cpu_texture: None,
            diff_texture: None,
            start_time: std::time::Instant::now(),
            frame_count: 0,
            last_fps_update: std::time::Instant::now(),
            fps: 0.0,
            gl_render_time_ms: 0.0,
            cpu_render_time_ms: 0.0,
            render_size,
            camera,
            camera_target,
            use_manual_camera: false,
            mouse_sensitivity: 0.005,
            zoom_sensitivity: 0.5,
            gl_latest_frame: None,
            cpu_latest_frame: None,
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

            let start = std::time::Instant::now();

            // Use current camera state
            if self.use_manual_camera {
                self.gpu_renderer.render_to_gl_with_camera(
                    gl,
                    width as i32,
                    height as i32,
                    &self.camera,
                );
            } else {
                self.gpu_renderer
                    .render_to_gl(gl, width as i32, height as i32, time);
            }

            // Make sure rendering is complete before measuring time
            gl.finish();
            self.gl_render_time_ms = start.elapsed().as_secs_f32() * 1000.0;

            gl.bind_framebuffer(FRAMEBUFFER, None);
        }
    }

    pub unsafe fn read_gl_texture_to_image(&self, gl: &Arc<Context>) -> Option<ColorImage> {
        self.gl_framebuffer?;

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

        // Use current camera state
        let camera = if self.use_manual_camera {
            Some(self.camera)
        } else {
            None
        };

        let request = RenderRequest {
            width,
            height,
            time,
            camera,
        };

        // Write request to Mutex slot (overwrites any pending request)
        // This drops old requests if CPU hasn't processed them yet
        *self.cpu_sync_request.lock().unwrap() = Some(request);

        // Try to read completed render (non-blocking)
        let response = {
            let mut resp_lock = self.cpu_sync_response.lock().unwrap();
            resp_lock.take() // Take the response, leaving None
        };

        if let Some(response) = response {
            self.cpu_render_time_ms = response.render_time_ms;

            // Convert to ColorImage and store
            let image_buffer = response.image;
            let (width, height) = (
                image_buffer.width() as usize,
                image_buffer.height() as usize,
            );
            let pixels: Vec<egui::Color32> = image_buffer
                .pixels()
                .map(|p| egui::Color32::from_rgb(p[0], p[1], p[2]))
                .collect();

            let color_image = ColorImage {
                size: [width, height],
                pixels,
            };

            self.cpu_latest_frame = Some(color_image);
        }
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

    fn compute_difference_image(
        &self,
        gl_image: &ColorImage,
        cpu_image: &ColorImage,
    ) -> ColorImage {
        assert_eq!(gl_image.size, cpu_image.size);
        let size = gl_image.size;

        let diff_pixels: Vec<egui::Color32> = gl_image
            .pixels
            .iter()
            .zip(cpu_image.pixels.iter())
            .map(|(gl_pixel, cpu_pixel)| {
                // Compute absolute difference per channel
                let r_diff = (gl_pixel.r() as i16 - cpu_pixel.r() as i16).unsigned_abs();
                let g_diff = (gl_pixel.g() as i16 - cpu_pixel.g() as i16).unsigned_abs();
                let b_diff = (gl_pixel.b() as i16 - cpu_pixel.b() as i16).unsigned_abs();

                // Amplify differences for visibility (multiply by 10, capped at 255)
                let r_amp = (r_diff * 10).min(255) as u8;
                let g_amp = (g_diff * 10).min(255) as u8;
                let b_amp = (b_diff * 10).min(255) as u8;

                egui::Color32::from_rgb(r_amp, g_amp, b_amp)
            })
            .collect();

        ColorImage {
            size,
            pixels: diff_pixels,
        }
    }

    pub fn show_ui(&mut self, ctx: &egui::Context, gl: &Arc<Context>) {
        let time = self.start_time.elapsed().as_secs_f32();

        // Render both GL and CPU
        unsafe { self.render_gl_to_texture(gl, time) };
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
                ui.label(format!(
                    "Resolution: {}x{}",
                    self.render_size.0, self.render_size.1
                ));
                ui.separator();
                ui.checkbox(&mut self.use_manual_camera, "Manual Camera");
                if self.use_manual_camera {
                    ui.label(
                        "(Drag left/right: rotate Y-axis, up/down: camera angle, scroll: zoom)",
                    );
                }
            });
        });

        // Main panel with three columns
        egui::CentralPanel::default().show(ctx, |ui| {
            // Read and update GL frame
            if let Some(gl_img) = unsafe { self.read_gl_texture_to_image(gl) } {
                self.gl_latest_frame = Some(gl_img);
            }

            // Calculate diff when BOTH renderers have latest frames available
            if let (Some(gl_img), Some(cpu_img)) = (&self.gl_latest_frame, &self.cpu_latest_frame) {
                let diff_image = self.compute_difference_image(gl_img, cpu_img);
                self.diff_texture =
                    Some(ctx.load_texture("diff_render", diff_image, TextureOptions::LINEAR));
            }

            // Create/update display textures from latest frames
            if let Some(ref gl_img) = self.gl_latest_frame {
                self.gl_egui_texture =
                    Some(ctx.load_texture("gl_render", gl_img.clone(), TextureOptions::LINEAR));
            }
            if let Some(ref cpu_img) = self.cpu_latest_frame {
                self.cpu_texture =
                    Some(ctx.load_texture("cpu_render", cpu_img.clone(), TextureOptions::LINEAR));
            }

            ui.columns(3, |columns| {
                // Left: GL Renderer
                columns[0].vertical_centered(|ui| {
                    ui.heading("GPU");
                    ui.label("OpenGL shader");
                    ui.label(format!("Render: {:.2}ms", self.gl_render_time_ms));

                    if let Some(texture) = &self.gl_egui_texture {
                        let response = ui.add(
                            egui::Image::from_texture(egui::load::SizedTexture {
                                id: texture.id(),
                                size: [self.render_size.0 as f32, self.render_size.1 as f32].into(),
                            })
                            .sense(egui::Sense::click_and_drag()),
                        );

                        // Handle mouse interaction for camera control
                        if self.use_manual_camera {
                            self.handle_camera_input(&response);
                        }
                    } else {
                        ui.label("N/A");
                    }
                });

                // Center: Difference
                columns[1].vertical_centered(|ui| {
                    ui.heading("Difference");
                    ui.label("Amplified 10x");

                    if let Some(texture) = &self.diff_texture {
                        ui.image(egui::ImageSource::Texture(egui::load::SizedTexture {
                            id: texture.id(),
                            size: [self.render_size.0 as f32, self.render_size.1 as f32].into(),
                        }));
                    } else {
                        ui.label("N/A");
                    }
                });

                // Right: CPU Renderer
                columns[2].vertical_centered(|ui| {
                    ui.heading("CPU");
                    ui.label("Pure Rust (async)");
                    ui.label(format!("Render: {:.2}ms", self.cpu_render_time_ms));

                    if let Some(texture) = &self.cpu_texture {
                        let response = ui.add(
                            egui::Image::from_texture(egui::load::SizedTexture {
                                id: texture.id(),
                                size: [self.render_size.0 as f32, self.render_size.1 as f32].into(),
                            })
                            .sense(egui::Sense::click_and_drag()),
                        );

                        // Handle mouse interaction for camera control
                        if self.use_manual_camera {
                            self.handle_camera_input(&response);
                        }
                    } else {
                        ui.label("N/A");
                    }
                });
            });
        });
    }

    fn handle_camera_input(&mut self, response: &egui::Response) {
        // Handle mouse drag for orbit rotation
        if response.dragged() {
            let delta = response.drag_delta();

            // Calculate yaw (horizontal) and pitch (vertical) changes
            let yaw_delta = -delta.x * self.mouse_sensitivity;
            let pitch_delta = -delta.y * self.mouse_sensitivity;

            // Orbit around the target (cube center)
            self.camera
                .orbit(self.camera_target, yaw_delta, pitch_delta);
        }

        // Handle scroll for zoom (move closer/farther from target)
        if response.hovered() {
            let scroll_delta = response.ctx.input(|i| i.smooth_scroll_delta.y);
            if scroll_delta.abs() > 0.01 {
                // Calculate zoom by moving along the camera-to-target direction
                let to_target = self.camera_target - self.camera.position;
                let distance = to_target.length();
                let zoom_amount = scroll_delta * self.zoom_sensitivity * 0.01;

                // Don't zoom too close (min distance of 0.5) or too far (max distance of 20)
                let new_distance = (distance - zoom_amount).clamp(0.5, 20.0);
                let zoom_factor = new_distance / distance;

                self.camera.position = self.camera_target - to_target * zoom_factor;
            }
        }
    }

    pub unsafe fn destroy(&mut self, gl: &Arc<Context>) {
        unsafe {
            if let Some(fb) = self.gl_framebuffer {
                gl.delete_framebuffer(fb);
            }
            if let Some(tex) = self.gl_texture {
                gl.delete_texture(tex);
            }
            self.gpu_renderer.destroy_gl(gl);
        }
    }
}
