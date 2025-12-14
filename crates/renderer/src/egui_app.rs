//! Egui application for comparing three raytracer implementations side-by-side

use egui::{ColorImage, TextureHandle, TextureOptions};
use glow::*;
use renderer::scenes::TestModel;
use renderer::{BcfCpuTracer, CameraConfig, CpuCubeTracer, GlCubeTracer, GpuTracer, MeshRenderer, Renderer};
use std::rc::Rc;
use std::sync::{Arc, Mutex};
use std::thread;

// Message to send to CPU renderer thread
struct RenderRequest {
    width: u32,
    height: u32,
    time: f32,
    camera: Option<CameraConfig>,
    disable_lighting: bool,
    model: TestModel,
}

// Response from CPU renderer thread
struct RenderResponse {
    image: image::ImageBuffer<image::Rgb<u8>, Vec<u8>>,
    render_time_ms: f32,
}

/// Which renderer to use for diff comparison
#[derive(Debug, Clone, Copy, PartialEq)]
enum DiffSource {
    Cpu,
    Gl,
    BcfCpu,
}

impl DiffSource {
    fn name(&self) -> &str {
        match self {
            DiffSource::Cpu => "CPU",
            DiffSource::Gl => "GL (WebGL 2.0)",
            DiffSource::BcfCpu => "BCF CPU",
        }
    }
}

pub struct DualRendererApp {
    // Five renderers
    cpu_sync_request: Arc<Mutex<Option<RenderRequest>>>,
    cpu_sync_response: Arc<Mutex<Option<RenderResponse>>>,
    gl_renderer: GlCubeTracer,
    bcf_cpu_renderer: BcfCpuTracer,
    gpu_renderer: GpuTracer,
    mesh_renderer: MeshRenderer,

    // GL framebuffers
    gl_framebuffer: Option<Framebuffer>,
    gl_texture: Option<Texture>,
    gpu_framebuffer: Option<Framebuffer>,
    gpu_texture: Option<Texture>,
    mesh_framebuffer: Option<Framebuffer>,
    mesh_texture: Option<Texture>,
    framebuffer_size: (i32, i32),

    // Mesh state
    current_cube: Rc<cube::Cube<u8>>,
    mesh_indices: Vec<usize>,

    // egui textures
    cpu_texture: Option<TextureHandle>,
    gl_egui_texture: Option<TextureHandle>,
    bcf_cpu_texture: Option<TextureHandle>,
    gpu_egui_texture: Option<TextureHandle>,
    mesh_egui_texture: Option<TextureHandle>,
    diff_texture: Option<TextureHandle>,

    // Timing
    start_time: std::time::Instant,
    frame_count: u64,
    last_fps_update: std::time::Instant,
    fps: f32,
    cpu_render_time_ms: f32,
    gl_render_time_ms: f32,
    bcf_cpu_render_time_ms: f32,
    gpu_render_time_ms: f32,
    mesh_render_time_ms: f32,

    // Settings
    render_size: (u32, u32),
    sync_mode: bool, // If true, wait for CPU renderer to complete each frame

    // Camera control
    camera: CameraConfig,
    camera_target: glam::Vec3,
    use_manual_camera: bool,
    mouse_sensitivity: f32,
    zoom_sensitivity: f32,

    // Rendering settings
    disable_lighting: bool,
    show_gl_errors: bool,
    current_model: TestModel,
    single_voxel_material: u8, // Material value for SingleRedVoxel model (0-255)

    // Current display frames
    cpu_latest_frame: Option<ColorImage>,
    gl_latest_frame: Option<ColorImage>,
    bcf_cpu_latest_frame: Option<ColorImage>,
    gpu_latest_frame: Option<ColorImage>,
    mesh_latest_frame: Option<ColorImage>,

    // Diff comparison settings
    diff_left: DiffSource,
    diff_right: DiffSource,
}

impl DualRendererApp {
    #[allow(dead_code)]
    pub unsafe fn new(gl: &Arc<Context>) -> Result<Self, String> {
        unsafe { Self::new_with_sync(gl, false) }
    }

    pub unsafe fn new_with_sync(gl: &Arc<Context>, sync_mode: bool) -> Result<Self, String> {
        // Create default scene (Octa Cube - Depth 1 for debugging)
        let default_model = TestModel::OctaCube;
        let cube = default_model.create();

        // Initialize GL renderer (WebGL 2.0 fragment shader)
        let mut gl_renderer = GlCubeTracer::new(cube.clone());
        unsafe {
            gl_renderer.init_gl(gl)?;
        }

        // Initialize BCF CPU renderer
        let bcf_cpu_renderer = BcfCpuTracer::new_from_cube(cube.clone());

        // Initialize GPU renderer (compute shader)
        let mut gpu_renderer = GpuTracer::new(cube.clone());
        let gpu_available = unsafe {
            match gpu_renderer.init_gl(gl) {
                Ok(_) => true,
                Err(e) => {
                    eprintln!("Warning: GPU compute shader not available: {}", e);
                    false
                }
            }
        };

        // Initialize mesh renderer
        let mut mesh_renderer = MeshRenderer::new();
        unsafe {
            mesh_renderer.init_gl(gl)?;
        }

        let render_size = (400, 300);

        // Initialize camera
        let camera_target = glam::Vec3::ZERO;
        let camera =
            CameraConfig::look_at(glam::Vec3::new(3.0, 2.0, 3.0), camera_target, glam::Vec3::Y);

        // Create CPU renderer thread
        let cpu_sync_request = Arc::new(Mutex::new(None));
        let cpu_sync_response = Arc::new(Mutex::new(None));

        let request_clone = Arc::clone(&cpu_sync_request);
        let response_clone = Arc::clone(&cpu_sync_response);

        thread::spawn(move || {
            let initial_cube = default_model.create();
            let mut cpu_renderer = CpuCubeTracer::new_with_cube(initial_cube);
            let mut current_model = default_model;

            loop {
                let request: Option<RenderRequest> = {
                    let mut req_lock = request_clone.lock().unwrap();
                    req_lock.take()
                };

                if let Some(request) = request {
                    let start = std::time::Instant::now();

                    // Recreate renderer if model changed
                    if request.model != current_model {
                        current_model = request.model;
                        let new_cube = current_model.create();
                        cpu_renderer = CpuCubeTracer::new_with_cube(new_cube);
                    }

                    // Set lighting mode
                    cpu_renderer.set_disable_lighting(request.disable_lighting);

                    if let Some(camera) = request.camera {
                        cpu_renderer.render_with_camera(request.width, request.height, &camera);
                    } else {
                        cpu_renderer.render(request.width, request.height, request.time);
                    }

                    let render_time = start.elapsed();

                    if let Some(image) = cpu_renderer.image_buffer() {
                        let response = RenderResponse {
                            image: image.clone(),
                            render_time_ms: render_time.as_secs_f32() * 1000.0,
                        };
                        *response_clone.lock().unwrap() = Some(response);
                    }
                } else {
                    std::thread::sleep(std::time::Duration::from_millis(10));
                }
            }
        });

        Ok(Self {
            cpu_sync_request,
            cpu_sync_response,
            gl_renderer,
            bcf_cpu_renderer,
            gpu_renderer,
            mesh_renderer,
            gl_framebuffer: None,
            gl_texture: None,
            gpu_framebuffer: None,
            gpu_texture: None,
            mesh_framebuffer: None,
            mesh_texture: None,
            framebuffer_size: (0, 0),
            current_cube: cube.clone(),
            mesh_indices: Vec::new(),
            cpu_texture: None,
            gl_egui_texture: None,
            bcf_cpu_texture: None,
            gpu_egui_texture: None,
            mesh_egui_texture: None,
            diff_texture: None,
            start_time: std::time::Instant::now(),
            frame_count: 0,
            last_fps_update: std::time::Instant::now(),
            fps: 0.0,
            cpu_render_time_ms: 0.0,
            gl_render_time_ms: 0.0,
            bcf_cpu_render_time_ms: 0.0,
            gpu_render_time_ms: if gpu_available { 0.0 } else { -1.0 },
            mesh_render_time_ms: 0.0,
            render_size,
            sync_mode,
            camera,
            camera_target,
            use_manual_camera: false,
            mouse_sensitivity: 0.005,
            zoom_sensitivity: 0.5,
            disable_lighting: false,
            show_gl_errors: false,
            current_model: default_model,
            single_voxel_material: 224, // Default: red (R2G3B2 encoded)
            cpu_latest_frame: None,
            gl_latest_frame: None,
            bcf_cpu_latest_frame: None,
            gpu_latest_frame: None,
            mesh_latest_frame: None,
            diff_left: DiffSource::Cpu,
            diff_right: DiffSource::Gl,
        })
    }

    unsafe fn ensure_framebuffer(&mut self, gl: &Arc<Context>, width: i32, height: i32) {
        if self.framebuffer_size != (width, height) || self.gl_framebuffer.is_none() {
            unsafe {
                // Clean up old resources
                if let Some(fb) = self.gl_framebuffer {
                    gl.delete_framebuffer(fb);
                }
                if let Some(tex) = self.gl_texture {
                    gl.delete_texture(tex);
                }
                if let Some(fb) = self.gpu_framebuffer {
                    gl.delete_framebuffer(fb);
                }
                if let Some(tex) = self.gpu_texture {
                    gl.delete_texture(tex);
                }
                if let Some(fb) = self.mesh_framebuffer {
                    gl.delete_framebuffer(fb);
                }
                if let Some(tex) = self.mesh_texture {
                    gl.delete_texture(tex);
                }

                // Helper to create framebuffer + texture
                let create_fb_tex = |gl: &Context| -> (Framebuffer, Texture) {
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

                    let framebuffer = gl.create_framebuffer().unwrap();
                    gl.bind_framebuffer(FRAMEBUFFER, Some(framebuffer));
                    gl.framebuffer_texture_2d(
                        FRAMEBUFFER,
                        COLOR_ATTACHMENT0,
                        TEXTURE_2D,
                        Some(texture),
                        0,
                    );

                    (framebuffer, texture)
                };

                // Create all framebuffers
                let (gl_framebuffer, gl_texture) = create_fb_tex(gl);
                let (gpu_framebuffer, gpu_texture) = create_fb_tex(gl);
                let (mesh_framebuffer, mesh_texture) = create_fb_tex(gl);

                gl.bind_framebuffer(FRAMEBUFFER, None);

                self.gl_texture = Some(gl_texture);
                self.gl_framebuffer = Some(gl_framebuffer);
                self.gpu_texture = Some(gpu_texture);
                self.gpu_framebuffer = Some(gpu_framebuffer);
                self.mesh_texture = Some(mesh_texture);
                self.mesh_framebuffer = Some(mesh_framebuffer);
                self.framebuffer_size = (width, height);
            }
        }
    }

    pub unsafe fn render_gl_to_texture(&mut self, gl: &Arc<Context>, time: f32) {
        let (width, height) = self.render_size;

        unsafe {
            self.ensure_framebuffer(gl, width as i32, height as i32);

            // Render GL tracer
            gl.bind_framebuffer(FRAMEBUFFER, self.gl_framebuffer);
            gl.viewport(0, 0, width as i32, height as i32);

            let start = std::time::Instant::now();

            // Update rendering settings
            self.gl_renderer.set_disable_lighting(self.disable_lighting);
            self.gl_renderer.set_show_errors(self.show_gl_errors);

            if self.use_manual_camera {
                self.gl_renderer.render_to_gl_with_camera(
                    gl,
                    width as i32,
                    height as i32,
                    &self.camera,
                );
            } else {
                self.gl_renderer
                    .render_to_gl(gl, width as i32, height as i32, time);
            }

            gl.finish();
            self.gl_render_time_ms = start.elapsed().as_secs_f32() * 1000.0;

            gl.bind_framebuffer(FRAMEBUFFER, None);
        }
    }

    pub unsafe fn render_bcf_cpu(&mut self, _gl: &Arc<Context>, time: f32) {
        let (width, height) = self.render_size;

        // Render BCF CPU tracer
        let start = std::time::Instant::now();

        self.bcf_cpu_renderer
            .set_disable_lighting(self.disable_lighting);

        if self.use_manual_camera {
            self.bcf_cpu_renderer
                .render_with_camera(width, height, &self.camera);
        } else {
            self.bcf_cpu_renderer.render(width, height, time);
        }

        self.bcf_cpu_render_time_ms = start.elapsed().as_secs_f32() * 1000.0;
    }

    pub unsafe fn render_gpu(&mut self, gl: &Arc<Context>, time: f32) {
        // Skip if GPU not available
        if self.gpu_render_time_ms < 0.0 {
            return;
        }

        let (width, height) = self.render_size;

        unsafe {
            self.ensure_framebuffer(gl, width as i32, height as i32);

            // Render GPU tracer
            gl.bind_framebuffer(FRAMEBUFFER, self.gpu_framebuffer);
            gl.viewport(0, 0, width as i32, height as i32);

            let start = std::time::Instant::now();

            // Note: GPU tracer doesn't have disable_lighting or show_errors settings

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

            gl.finish();
            self.gpu_render_time_ms = start.elapsed().as_secs_f32() * 1000.0;

            gl.bind_framebuffer(FRAMEBUFFER, None);
        }
    }

    pub unsafe fn render_mesh(&mut self, gl: &Arc<Context>, time: f32) {
        let (width, height) = self.render_size;

        unsafe {
            self.ensure_framebuffer(gl, width as i32, height as i32);

            // Render mesh
            gl.bind_framebuffer(FRAMEBUFFER, self.mesh_framebuffer);
            gl.viewport(0, 0, width as i32, height as i32);
            gl.clear_color(0.4, 0.5, 0.6, 1.0);
            gl.clear(COLOR_BUFFER_BIT | DEPTH_BUFFER_BIT);

            let start = std::time::Instant::now();

            // Upload mesh if not already done
            if self.mesh_indices.is_empty() {
                let depth = 1; // Match OctaCube depth
                match self.mesh_renderer.upload_mesh(gl, &self.current_cube, depth) {
                    Ok(idx) => {
                        self.mesh_indices.push(idx);
                    }
                    Err(e) => {
                        eprintln!("Failed to upload mesh: {}", e);
                        self.mesh_render_time_ms = -1.0;
                        gl.bind_framebuffer(FRAMEBUFFER, None);
                        return;
                    }
                }
            }

            // Render mesh at center
            if !self.mesh_indices.is_empty() {
                let position = glam::Vec3::ZERO;
                let rotation = if !self.use_manual_camera {
                    // Auto-rotate
                    glam::Quat::from_rotation_y(time * 0.5)
                } else {
                    glam::Quat::IDENTITY
                };

                self.mesh_renderer.render_mesh(
                    gl,
                    0,
                    position,
                    rotation,
                    &self.camera,
                    width as i32,
                    height as i32,
                );
            }

            gl.finish();
            self.mesh_render_time_ms = start.elapsed().as_secs_f32() * 1000.0;

            gl.bind_framebuffer(FRAMEBUFFER, None);
        }
    }

    unsafe fn read_framebuffer_to_image(
        &self,
        gl: &Arc<Context>,
        framebuffer: Option<Framebuffer>,
    ) -> Option<ColorImage> {
        framebuffer?;

        let (width, height) = self.render_size;
        let mut pixels = vec![0u8; (width * height * 4) as usize];

        unsafe {
            gl.bind_framebuffer(FRAMEBUFFER, framebuffer);
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
            disable_lighting: self.disable_lighting,
            model: self.current_model,
        };

        *self.cpu_sync_request.lock().unwrap() = Some(request);

        // In sync mode, wait for the CPU renderer to complete
        if self.sync_mode {
            // Poll until response is ready (blocking)
            loop {
                let response = {
                    let mut resp_lock = self.cpu_sync_response.lock().unwrap();
                    resp_lock.take()
                };

                if let Some(response) = response {
                    self.cpu_render_time_ms = response.render_time_ms;

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
                    break;
                }

                // Small sleep to avoid busy waiting
                std::thread::sleep(std::time::Duration::from_micros(100));
            }
        } else {
            // Async mode: just check if there's a response ready
            let response = {
                let mut resp_lock = self.cpu_sync_response.lock().unwrap();
                resp_lock.take()
            };

            if let Some(response) = response {
                self.cpu_render_time_ms = response.render_time_ms;

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
        left_image: &ColorImage,
        right_image: &ColorImage,
    ) -> ColorImage {
        assert_eq!(left_image.size, right_image.size);
        let size = left_image.size;

        let diff_pixels: Vec<egui::Color32> = left_image
            .pixels
            .iter()
            .zip(right_image.pixels.iter())
            .map(|(left_pixel, right_pixel)| {
                let r_diff = (left_pixel.r() as i16 - right_pixel.r() as i16).unsigned_abs();
                let g_diff = (left_pixel.g() as i16 - right_pixel.g() as i16).unsigned_abs();
                let b_diff = (left_pixel.b() as i16 - right_pixel.b() as i16).unsigned_abs();

                // Amplify differences for visibility
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

    fn get_frame(&self, source: DiffSource) -> Option<&ColorImage> {
        match source {
            DiffSource::Cpu => self.cpu_latest_frame.as_ref(),
            DiffSource::Gl => self.gl_latest_frame.as_ref(),
            DiffSource::BcfCpu => self.bcf_cpu_latest_frame.as_ref(),
        }
    }

    pub fn show_ui(&mut self, ctx: &egui::Context, gl: &Arc<Context>) {
        let time = self.start_time.elapsed().as_secs_f32();

        // Render all tracers
        unsafe {
            self.render_gl_to_texture(gl, time);
            self.render_bcf_cpu(gl, time);
            self.render_gpu(gl, time);
            self.render_mesh(gl, time);
        }
        self.render_cpu(time);

        self.update_fps();

        // Top panel
        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.heading("Triple Cube Raytracer");
                ui.separator();
                ui.label(format!("FPS: {:.1}", self.fps));
                ui.separator();
                ui.label(format!("Time: {:.2}s", time));
                ui.separator();
                ui.label(format!(
                    "Resolution: {}x{}",
                    self.render_size.0, self.render_size.1
                ));
            });

            ui.horizontal_wrapped(|ui| {
                ui.checkbox(&mut self.use_manual_camera, "Manual Camera");
                if self.use_manual_camera {
                    ui.label("(Drag: orbit, Scroll: zoom)");
                }
                ui.separator();
                ui.checkbox(&mut self.disable_lighting, "Disable Lighting");
                ui.separator();
                ui.checkbox(&mut self.show_gl_errors, "Show GL Errors");
                ui.separator();

                // Model selector
                ui.label("Test Model:");
                let mut model_changed = false;
                egui::ComboBox::from_label("")
                    .selected_text(self.current_model.name())
                    .show_ui(ui, |ui| {
                        for model in TestModel::all() {
                            if ui
                                .selectable_value(&mut self.current_model, *model, model.name())
                                .clicked()
                            {
                                model_changed = true;
                            }
                        }
                    });

                // Reload scene if model changed
                if model_changed {
                    let new_cube = self.current_model.create();
                    self.current_cube = new_cube.clone();
                    self.mesh_indices.clear(); // Force re-upload of mesh

                    self.gl_renderer = GlCubeTracer::new(new_cube.clone());
                    unsafe {
                        if let Err(e) = self.gl_renderer.init_gl(gl) {
                            eprintln!("Failed to reinitialize GL renderer: {}", e);
                        }
                    }

                    self.bcf_cpu_renderer = BcfCpuTracer::new_from_cube(new_cube.clone());

                    self.gpu_renderer = GpuTracer::new(new_cube.clone());
                    unsafe {
                        if let Err(e) = self.gpu_renderer.init_gl(gl) {
                            eprintln!("Failed to reinitialize GPU renderer: {}", e);
                            self.gpu_render_time_ms = -1.0;
                        }
                    }

                    // CPU renderer will be updated on next frame via sync thread
                }

                // Material selector for Single Red Voxel (depth 0) model
                if self.current_model == TestModel::SingleRedVoxel {
                    ui.separator();
                    ui.label("Material:");
                    let mut material_changed = false;

                    // Preset material buttons
                    egui::ComboBox::from_label("Preset")
                        .selected_text(format!("{}", self.single_voxel_material))
                        .show_ui(ui, |ui| {
                            let presets = [
                                (0, "Empty"),
                                (1, "Error: Hot Pink"),
                                (2, "Error: Red-Orange"),
                                (3, "Error: Orange"),
                                (4, "Error: Sky Blue"),
                                (5, "Error: Purple"),
                                (6, "Error: Spring Green"),
                                (7, "Error: Yellow"),
                                (10, "Palette: Index 10"),
                                (50, "Palette: Index 50"),
                                (100, "Palette: Index 100"),
                                (224, "R2G3B2: Red"),
                                (252, "R2G3B2: Yellow"),
                            ];

                            for (value, name) in presets {
                                if ui
                                    .selectable_value(&mut self.single_voxel_material, value, name)
                                    .clicked()
                                {
                                    material_changed = true;
                                }
                            }
                        });

                    // Numeric slider for fine control
                    let mut mat_val = self.single_voxel_material as i32;
                    if ui
                        .add(egui::Slider::new(&mut mat_val, 0..=255).text("Value"))
                        .changed()
                    {
                        self.single_voxel_material = mat_val as u8;
                        material_changed = true;
                    }

                    // Reload if material changed
                    if material_changed {
                        use cube::Cube;
                        let new_cube = Rc::new(Cube::Solid(self.single_voxel_material));
                        self.current_cube = new_cube.clone();
                        self.mesh_indices.clear(); // Force re-upload

                        self.gl_renderer = GlCubeTracer::new(new_cube.clone());
                        unsafe {
                            if let Err(e) = self.gl_renderer.init_gl(gl) {
                                eprintln!("Failed to reinitialize GL renderer: {}", e);
                            }
                        }

                        self.bcf_cpu_renderer = BcfCpuTracer::new_from_cube(new_cube.clone());

                        self.gpu_renderer = GpuTracer::new(new_cube.clone());
                        unsafe {
                            if let Err(e) = self.gpu_renderer.init_gl(gl) {
                                eprintln!("Failed to reinitialize GPU renderer: {}", e);
                                self.gpu_render_time_ms = -1.0;
                            }
                        }

                        // CPU renderer will be updated on next frame via sync thread
                    }
                }
            });
        });

        // Main panel with 3x2 grid
        egui::CentralPanel::default().show(ctx, |ui| {
            // Read frames from framebuffers
            if let Some(gl_img) = unsafe { self.read_framebuffer_to_image(gl, self.gl_framebuffer) }
            {
                self.gl_latest_frame = Some(gl_img);
            }
            if let Some(gpu_img) = unsafe { self.read_framebuffer_to_image(gl, self.gpu_framebuffer) }
            {
                self.gpu_latest_frame = Some(gpu_img);
            }
            if let Some(mesh_img) = unsafe { self.read_framebuffer_to_image(gl, self.mesh_framebuffer) }
            {
                self.mesh_latest_frame = Some(mesh_img);
            }

            // Get BCF CPU tracer image
            if let Some(bcf_image) = self.bcf_cpu_renderer.image_buffer() {
                let (width, height) = (bcf_image.width() as usize, bcf_image.height() as usize);
                let mut pixels = vec![0u8; width * height * 4];
                for (i, pixel) in bcf_image.pixels().enumerate() {
                    pixels[i * 4] = pixel[0];
                    pixels[i * 4 + 1] = pixel[1];
                    pixels[i * 4 + 2] = pixel[2];
                    pixels[i * 4 + 3] = 255;
                }
                self.bcf_cpu_latest_frame =
                    Some(ColorImage::from_rgba_unmultiplied([width, height], &pixels));
            }

            // Calculate diff
            if let (Some(left_img), Some(right_img)) = (
                self.get_frame(self.diff_left),
                self.get_frame(self.diff_right),
            ) {
                let diff_image = self.compute_difference_image(left_img, right_img);
                self.diff_texture =
                    Some(ctx.load_texture("diff_render", diff_image, TextureOptions::LINEAR));
            }

            // Create textures
            if let Some(ref cpu_img) = self.cpu_latest_frame {
                self.cpu_texture =
                    Some(ctx.load_texture("cpu_render", cpu_img.clone(), TextureOptions::LINEAR));
            }
            if let Some(ref gl_img) = self.gl_latest_frame {
                self.gl_egui_texture =
                    Some(ctx.load_texture("gl_render", gl_img.clone(), TextureOptions::LINEAR));
            }
            if let Some(ref bcf_img) = self.bcf_cpu_latest_frame {
                self.bcf_cpu_texture = Some(ctx.load_texture(
                    "bcf_cpu_render",
                    bcf_img.clone(),
                    TextureOptions::LINEAR,
                ));
            }
            if let Some(ref gpu_img) = self.gpu_latest_frame {
                self.gpu_egui_texture = Some(ctx.load_texture(
                    "gpu_render",
                    gpu_img.clone(),
                    TextureOptions::LINEAR,
                ));
            }
            if let Some(ref mesh_img) = self.mesh_latest_frame {
                self.mesh_egui_texture = Some(ctx.load_texture(
                    "mesh_render",
                    mesh_img.clone(),
                    TextureOptions::LINEAR,
                ));
            }

            // 3x2 Grid layout
            let mut responses = Vec::new();
            egui::Grid::new("tracer_grid")
                .spacing([10.0, 10.0])
                .show(ui, |ui| {
                    // Row 1: CPU | GL | GPU
                    let cpu_response = Self::render_view_static(
                        ui,
                        "CPU",
                        "Pure Rust",
                        self.cpu_render_time_ms,
                        &self.cpu_texture,
                        self.render_size,
                    );
                    if let Some(r) = cpu_response {
                        responses.push(r);
                    }

                    let gl_response = Self::render_view_static(
                        ui,
                        "GL",
                        "WebGL 2.0 Fragment",
                        self.gl_render_time_ms,
                        &self.gl_egui_texture,
                        self.render_size,
                    );
                    if let Some(r) = gl_response {
                        responses.push(r);
                    }

                    let gpu_response = Self::render_view_static(
                        ui,
                        "GPU",
                        "Compute Shader",
                        self.gpu_render_time_ms,
                        &self.gpu_egui_texture,
                        self.render_size,
                    );
                    if let Some(r) = gpu_response {
                        responses.push(r);
                    }
                    ui.end_row();

                    // Row 2: BCF CPU | Mesh | Diff
                    let bcf_response = Self::render_view_static(
                        ui,
                        "BCF CPU",
                        "BCF Traversal",
                        self.bcf_cpu_render_time_ms,
                        &self.bcf_cpu_texture,
                        self.render_size,
                    );
                    if let Some(r) = bcf_response {
                        responses.push(r);
                    }

                    let mesh_response = Self::render_view_static(
                        ui,
                        "Mesh",
                        "Triangle Renderer",
                        self.mesh_render_time_ms,
                        &self.mesh_egui_texture,
                        self.render_size,
                    );
                    if let Some(r) = mesh_response {
                        responses.push(r);
                    }

                    self.render_diff_view(ui);
                    ui.end_row();
                });

            // Handle camera input after all views are rendered
            if self.use_manual_camera {
                for response in responses {
                    self.handle_camera_input(&response);
                }
            }
        });
    }

    fn render_view_static(
        ui: &mut egui::Ui,
        title: &str,
        subtitle: &str,
        render_time: f32,
        texture: &Option<TextureHandle>,
        render_size: (u32, u32),
    ) -> Option<egui::Response> {
        let mut image_response = None;
        ui.vertical(|ui| {
            ui.heading(title);
            ui.label(subtitle);
            if render_time >= 0.0 {
                ui.label(format!("Render: {:.2}ms", render_time));
            } else {
                ui.label("Not Available");
            }

            if let Some(tex) = texture {
                let response = ui.add(
                    egui::Image::from_texture(egui::load::SizedTexture {
                        id: tex.id(),
                        size: [render_size.0 as f32, render_size.1 as f32].into(),
                    })
                    .sense(egui::Sense::click_and_drag()),
                );
                image_response = Some(response);
            } else {
                ui.label("N/A");
            }
        });
        image_response
    }

    fn render_diff_view(&mut self, ui: &mut egui::Ui) {
        ui.vertical(|ui| {
            ui.heading("Difference");

            // Dropdown selectors for diff comparison
            ui.horizontal(|ui| {
                egui::ComboBox::from_label("Left")
                    .selected_text(self.diff_left.name())
                    .show_ui(ui, |ui| {
                        ui.selectable_value(
                            &mut self.diff_left,
                            DiffSource::Cpu,
                            DiffSource::Cpu.name(),
                        );
                        ui.selectable_value(
                            &mut self.diff_left,
                            DiffSource::Gl,
                            DiffSource::Gl.name(),
                        );
                        ui.selectable_value(
                            &mut self.diff_left,
                            DiffSource::BcfCpu,
                            DiffSource::BcfCpu.name(),
                        );
                    });

                ui.label("vs");

                egui::ComboBox::from_label("Right")
                    .selected_text(self.diff_right.name())
                    .show_ui(ui, |ui| {
                        ui.selectable_value(
                            &mut self.diff_right,
                            DiffSource::Cpu,
                            DiffSource::Cpu.name(),
                        );
                        ui.selectable_value(
                            &mut self.diff_right,
                            DiffSource::Gl,
                            DiffSource::Gl.name(),
                        );
                        ui.selectable_value(
                            &mut self.diff_right,
                            DiffSource::BcfCpu,
                            DiffSource::BcfCpu.name(),
                        );
                    });
            });

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
    }

    fn handle_camera_input(&mut self, response: &egui::Response) {
        if response.dragged() {
            let delta = response.drag_delta();
            let yaw_delta = -delta.x * self.mouse_sensitivity;
            let pitch_delta = -delta.y * self.mouse_sensitivity;
            self.camera
                .orbit(self.camera_target, yaw_delta, pitch_delta);
        }

        if response.hovered() {
            let scroll_delta = response.ctx.input(|i| i.smooth_scroll_delta.y);
            if scroll_delta.abs() > 0.01 {
                let to_target = self.camera_target - self.camera.position;
                let distance = to_target.length();
                let zoom_amount = scroll_delta * self.zoom_sensitivity * 0.01;
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
            if let Some(fb) = self.gpu_framebuffer {
                gl.delete_framebuffer(fb);
            }
            if let Some(tex) = self.gpu_texture {
                gl.delete_texture(tex);
            }
            if let Some(fb) = self.mesh_framebuffer {
                gl.delete_framebuffer(fb);
            }
            if let Some(tex) = self.mesh_texture {
                gl.delete_texture(tex);
            }
            self.gl_renderer.destroy_gl(gl);
            self.gpu_renderer.destroy_gl(gl);
            self.mesh_renderer.destroy_gl(gl);
            // BCF CPU renderer has no GL resources to destroy
        }
    }
}
