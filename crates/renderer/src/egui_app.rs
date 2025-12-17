//! Egui application for comparing multiple raytracer implementations side-by-side
//!
//! This module provides `CubeRendererApp`, an egui application that displays
//! five different cube renderers (CPU, GL, BCF CPU, GPU Compute, and Mesh)
//! with diff comparison capabilities.

use cube::FabricConfig;
use egui::{ColorImage, TextureHandle, TextureOptions};
use glow::*;
use renderer::scenes::{create_cube_from_id, get_fabric_config, get_model, get_single_cube_config};
use renderer::{BcfTracer, Camera, ComputeTracer, CpuTracer, GlTracer, MeshRenderer, Renderer};
use std::rc::Rc;
use std::sync::{Arc, Mutex};
use std::thread;

// Message to send to CPU renderer thread
struct RenderRequest {
    width: u32,
    height: u32,
    time: f32,
    camera: Option<Camera>,
    disable_lighting: bool,
    model_id: String,
    // For fabric models, we need the config and max_depth
    fabric_config: Option<FabricConfig>,
    fabric_max_depth: Option<u32>,
    // For single voxel model, we need the material
    single_voxel_material: Option<u8>,
}

// Response from CPU renderer thread
struct RenderResponse {
    image: image::ImageBuffer<image::Rgb<u8>, Vec<u8>>,
    render_time_ms: f32,
}

/// Convert a fabric Cube<Quat> to Cube<u8> using surface detection (standalone function for thread use)
fn fabric_to_material_cube(fabric_cube: &cube::Cube<glam::Quat>, depth: u32) -> cube::Cube<u8> {
    use cube::fabric::quaternion_to_color;
    use cube::Cube;

    match fabric_cube {
        Cube::Solid(quat) => {
            let magnitude = quat.length();
            if magnitude < 1.0 {
                // Inside: solid voxel
                // Convert quaternion rotation to color
                let [r, g, b] = quaternion_to_color(*quat);
                // Encode as R2G3B2 for shader (values 128-255)
                // Shader expects: 128 + (r2 << 5) + (g3 << 2) + b2
                // where r2 is 2 bits (0-3), g3 is 3 bits (0-7), b2 is 2 bits (0-3)
                let r2 = (r >> 6) & 0x03;
                let g3 = (g >> 5) & 0x07;
                let b2 = (b >> 6) & 0x03;
                Cube::Solid(128 | (r2 << 5) | (g3 << 2) | b2)
            } else {
                // Outside: empty
                Cube::Solid(0)
            }
        }
        Cube::Cubes(children) if depth > 0 => {
            let new_children: [Rc<Cube<u8>>; 8] =
                std::array::from_fn(|i| Rc::new(fabric_to_material_cube(&children[i], depth - 1)));
            Cube::Cubes(Box::new(new_children))
        }
        Cube::Cubes(children) => {
            // At max depth, evaluate first child
            fabric_to_material_cube(&children[0], 0)
        }
        _ => Cube::Solid(0),
    }
}

/// Which renderer to use for diff comparison
#[derive(Debug, Clone, Copy, PartialEq)]
enum DiffSource {
    Cpu,
    Gl,
    BcfCpu,
    Gpu,
    Mesh,
}

impl DiffSource {
    fn name(&self) -> &str {
        match self {
            DiffSource::Cpu => "CPU",
            DiffSource::Gl => "GL (WebGL 2.0)",
            DiffSource::BcfCpu => "BCF CPU",
            DiffSource::Gpu => "GPU (Compute)",
            DiffSource::Mesh => "Mesh",
        }
    }

    fn from_str(s: &str) -> Option<Self> {
        match s {
            "cpu" => Some(DiffSource::Cpu),
            "gl" => Some(DiffSource::Gl),
            "bcf" => Some(DiffSource::BcfCpu),
            "compute" | "gpu" => Some(DiffSource::Gpu),
            "mesh" => Some(DiffSource::Mesh),
            _ => None,
        }
    }
}

/// State for collapsible model panel sections
#[derive(Debug, Clone)]
struct ModelPanelState {
    single_cube_expanded: bool,
    vox_expanded: bool,
    csm_expanded: bool,
    fabric_expanded: bool,
}

impl Default for ModelPanelState {
    fn default() -> Self {
        Self {
            single_cube_expanded: true,
            vox_expanded: false,
            csm_expanded: true,
            fabric_expanded: false,
        }
    }
}

/// Multi-renderer comparison application
///
/// Displays five different cube renderers side-by-side:
/// - CPU: Pure Rust raytracer
/// - GL: WebGL 2.0 fragment shader raytracer
/// - BCF CPU: Binary Cube Format traversal raytracer
/// - GPU: Compute shader raytracer
/// - Mesh: Triangle mesh rasterizer
pub struct CubeRendererApp {
    // Five renderers
    cpu_sync_request: Arc<Mutex<Option<RenderRequest>>>,
    cpu_sync_response: Arc<Mutex<Option<RenderResponse>>>,
    gl_renderer: GlTracer,
    bcf_cpu_renderer: BcfTracer,
    gpu_renderer: ComputeTracer,
    mesh_renderer: MeshRenderer,

    // GL framebuffers
    gl_framebuffer: Option<Framebuffer>,
    gl_texture: Option<Texture>,
    gpu_framebuffer: Option<Framebuffer>,
    gpu_texture: Option<Texture>,
    mesh_framebuffer: Option<Framebuffer>,
    mesh_texture: Option<Texture>,
    mesh_depth_rb: Option<Renderbuffer>,
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
    camera: Camera,
    camera_target: glam::Vec3,
    use_manual_camera: bool,
    mouse_sensitivity: f32,
    zoom_sensitivity: f32,

    // Rendering settings
    disable_lighting: bool,
    show_gl_errors: bool,
    current_model_id: String,
    single_voxel_material: u8, // Material value for SingleRedVoxel model (0-255)

    // Model selector panel state
    model_panel_expanded: ModelPanelState,

    // Fabric parameters (editable copy)
    fabric_config: FabricConfig,
    fabric_max_depth: u32,

    // Mesh caching settings
    mesh_cache_enabled: bool,
    mesh_needs_regeneration: bool,
    mesh_upload_time_ms: f32,
    mesh_vertex_count: usize,
    mesh_face_count: usize,

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

impl CubeRendererApp {
    #[allow(dead_code)]
    pub unsafe fn new(gl: &Arc<Context>) -> Result<Self, String> {
        unsafe { Self::new_with_sync(gl, false, None) }
    }

    pub unsafe fn new_with_sync(gl: &Arc<Context>, sync_mode: bool, model_name: Option<&str>) -> Result<Self, String> {
        // Create default scene (Octa Cube - Depth 1 for debugging, or user-specified model)
        let default_model_id = model_name.unwrap_or("octa");
        let fabric_config = get_fabric_config().clone();
        let fabric_depth = get_fabric_config().max_depth;
        let single_material = get_single_cube_config().default_material;

        let cube = if default_model_id == "fabric" {
            // Generate fabric model
            use cube::FabricGenerator;
            let generator = FabricGenerator::new(fabric_config.clone());
            let fabric_cube = generator.generate_cube(fabric_depth);
            Rc::new(fabric_to_material_cube(&fabric_cube, fabric_depth))
        } else if default_model_id == "single" {
            // Single cube with configured material
            use cube::Cube;
            Rc::new(Cube::Solid(single_material))
        } else {
            create_cube_from_id(default_model_id).or_else(|e| {
                eprintln!("Warning: {}, falling back to 'octa'", e);
                create_cube_from_id("octa")
            })?
        };

        // Initialize GL renderer (WebGL 2.0 fragment shader)
        let mut gl_renderer = GlTracer::new(cube.clone());
        unsafe {
            gl_renderer.init_gl(gl)?;
        }

        // Initialize BCF CPU renderer
        let bcf_cpu_renderer = BcfTracer::new_from_cube(cube.clone());

        // Initialize GPU renderer (compute shader)
        let mut gpu_renderer = ComputeTracer::new(cube.clone());
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
            Camera::look_at(glam::Vec3::new(3.0, 2.0, 3.0), camera_target, glam::Vec3::Y);

        // Create CPU renderer thread
        let cpu_sync_request = Arc::new(Mutex::new(None));
        let cpu_sync_response = Arc::new(Mutex::new(None));

        let request_clone = Arc::clone(&cpu_sync_request);
        let response_clone = Arc::clone(&cpu_sync_response);

        let thread_model_id = default_model_id.to_string();
        let thread_fabric_config = get_fabric_config().clone();
        let thread_fabric_depth = get_fabric_config().max_depth;
        let thread_single_material = get_single_cube_config().default_material;
        thread::spawn(move || {
            // Create initial cube - handle fabric and single specially
            let initial_cube = if thread_model_id == "fabric" {
                use cube::FabricGenerator;
                let generator = FabricGenerator::new(thread_fabric_config.clone());
                let fabric_cube = generator.generate_cube(thread_fabric_depth);
                Rc::new(fabric_to_material_cube(&fabric_cube, thread_fabric_depth))
            } else if thread_model_id == "single" {
                use cube::Cube;
                Rc::new(Cube::Solid(thread_single_material))
            } else {
                create_cube_from_id(&thread_model_id).unwrap_or_else(|e| {
                    eprintln!("Failed to load model '{}': {}, falling back to octa", thread_model_id, e);
                    create_cube_from_id("octa").unwrap()
                })
            };
            let mut cpu_renderer = CpuTracer::new_with_cube(initial_cube);
            let mut current_model_id = thread_model_id;
            let mut current_fabric_config: Option<FabricConfig> = Some(thread_fabric_config);
            let mut current_fabric_depth: Option<u32> = Some(thread_fabric_depth);
            let mut current_single_material: Option<u8> = Some(thread_single_material);

            loop {
                let request: Option<RenderRequest> = {
                    let mut req_lock = request_clone.lock().unwrap();
                    req_lock.take()
                };

                if let Some(request) = request {
                    let start = std::time::Instant::now();

                    // Determine if we need to recreate the renderer
                    let model_changed = request.model_id != current_model_id;
                    let fabric_changed = request.fabric_config != current_fabric_config
                        || request.fabric_max_depth != current_fabric_depth;
                    let single_changed = request.single_voxel_material != current_single_material;

                    if model_changed || (request.model_id == "fabric" && fabric_changed)
                        || (request.model_id == "single" && single_changed)
                    {
                        current_model_id = request.model_id.clone();
                        current_fabric_config = request.fabric_config.clone();
                        current_fabric_depth = request.fabric_max_depth;
                        current_single_material = request.single_voxel_material;

                        let new_cube = if request.model_id == "fabric" {
                            // Generate fabric model
                            if let (Some(config), Some(depth)) =
                                (&request.fabric_config, request.fabric_max_depth)
                            {
                                use cube::FabricGenerator;
                                let generator = FabricGenerator::new(config.clone());
                                let fabric_cube = generator.generate_cube(depth);
                                Rc::new(fabric_to_material_cube(&fabric_cube, depth))
                            } else {
                                // Fallback to octa if fabric config missing
                                create_cube_from_id("octa").unwrap()
                            }
                        } else if request.model_id == "single" {
                            // Single cube with current material
                            use cube::Cube;
                            let material = request.single_voxel_material.unwrap_or(224);
                            Rc::new(Cube::Solid(material))
                        } else {
                            create_cube_from_id(&request.model_id).unwrap_or_else(|e| {
                                eprintln!("Failed to load model '{}': {}, falling back to octa", request.model_id, e);
                                create_cube_from_id("octa").unwrap()
                            })
                        };
                        cpu_renderer = CpuTracer::new_with_cube(new_cube);
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
            mesh_depth_rb: None,
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
            current_model_id: default_model_id.to_string(),
            single_voxel_material: get_single_cube_config().default_material,
            model_panel_expanded: ModelPanelState::default(),
            fabric_config: get_fabric_config().clone(),
            fabric_max_depth: get_fabric_config().max_depth,
            mesh_cache_enabled: true,
            mesh_needs_regeneration: true, // Start with regeneration needed
            mesh_upload_time_ms: 0.0,
            mesh_vertex_count: 0,
            mesh_face_count: 0,
            cpu_latest_frame: None,
            gl_latest_frame: None,
            bcf_cpu_latest_frame: None,
            gpu_latest_frame: None,
            mesh_latest_frame: None,
            diff_left: DiffSource::Mesh,
            diff_right: DiffSource::Cpu,
        })
    }

    /// Set the diff sources from CLI argument strings
    pub fn set_diff_sources(&mut self, left: &str, right: &str) {
        if let Some(l) = DiffSource::from_str(left) {
            self.diff_left = l;
        }
        if let Some(r) = DiffSource::from_str(right) {
            self.diff_right = r;
        }
    }

    /// Get the diff source names for output file naming
    pub fn get_diff_source_names(&self) -> (&'static str, &'static str) {
        let left = match self.diff_left {
            DiffSource::Cpu => "cpu",
            DiffSource::Gl => "gl",
            DiffSource::BcfCpu => "bcf",
            DiffSource::Gpu => "gpu",
            DiffSource::Mesh => "mesh",
        };
        let right = match self.diff_right {
            DiffSource::Cpu => "cpu",
            DiffSource::Gl => "gl",
            DiffSource::BcfCpu => "bcf",
            DiffSource::Gpu => "gpu",
            DiffSource::Mesh => "mesh",
        };
        (left, right)
    }

    /// Save all individual renderer frames to files for debugging.
    pub fn save_all_frames(&self, output_dir: &str) -> Result<(), String> {
        std::fs::create_dir_all(output_dir)
            .map_err(|e| format!("Failed to create output directory: {}", e))?;

        let frames = [
            ("cpu", &self.cpu_latest_frame),
            ("gl", &self.gl_latest_frame),
            ("bcf_cpu", &self.bcf_cpu_latest_frame),
            ("gpu", &self.gpu_latest_frame),
            ("mesh", &self.mesh_latest_frame),
        ];

        for (name, frame_opt) in frames {
            if let Some(frame) = frame_opt {
                let filename = format!("{}/frame_{}.png", output_dir, name);
                let width = frame.size[0] as u32;
                let height = frame.size[1] as u32;
                let mut rgba_data = Vec::with_capacity((width * height * 4) as usize);
                for pixel in &frame.pixels {
                    rgba_data.push(pixel.r());
                    rgba_data.push(pixel.g());
                    rgba_data.push(pixel.b());
                    rgba_data.push(pixel.a());
                }

                let img: image::ImageBuffer<image::Rgba<u8>, Vec<u8>> =
                    image::ImageBuffer::from_raw(width, height, rgba_data)
                        .ok_or_else(|| format!("Failed to create image buffer for {}", name))?;

                img.save(&filename)
                    .map_err(|e| format!("Failed to save {}: {}", name, e))?;
                println!("Saved: {}", filename);
            } else {
                println!("Frame {} not available", name);
            }
        }

        Ok(())
    }

    /// Save the diff image to a file. Returns the path if successful.
    pub fn save_diff_image(&self, output_dir: &str) -> Result<String, String> {
        // Get the frames for diffing
        let left_frame = self.get_frame(self.diff_left);
        let right_frame = self.get_frame(self.diff_right);

        match (left_frame, right_frame) {
            (Some(left_img), Some(right_img)) => {
                let diff_image = self.compute_difference_image(left_img, right_img);
                let (left_name, right_name) = self.get_diff_source_names();
                let filename = format!("{}/diff_{}-{}.png", output_dir, left_name, right_name);

                // Convert ColorImage to image crate format and save
                let width = diff_image.size[0] as u32;
                let height = diff_image.size[1] as u32;
                let mut rgba_data = Vec::with_capacity((width * height * 4) as usize);
                for pixel in &diff_image.pixels {
                    rgba_data.push(pixel.r());
                    rgba_data.push(pixel.g());
                    rgba_data.push(pixel.b());
                    rgba_data.push(pixel.a());
                }

                let img: image::ImageBuffer<image::Rgba<u8>, Vec<u8>> =
                    image::ImageBuffer::from_raw(width, height, rgba_data)
                        .ok_or_else(|| "Failed to create image buffer".to_string())?;

                img.save(&filename)
                    .map_err(|e| format!("Failed to save image: {}", e))?;

                Ok(filename)
            }
            (None, _) => Err(format!(
                "Left frame ({:?}) not available",
                self.diff_left
            )),
            (_, None) => Err(format!(
                "Right frame ({:?}) not available",
                self.diff_right
            )),
        }
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
                if let Some(rb) = self.mesh_depth_rb {
                    gl.delete_renderbuffer(rb);
                }

                // Helper to create framebuffer + texture (color only)
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

                // Helper to create framebuffer + texture + depth buffer (for mesh rendering)
                let create_fb_tex_depth =
                    |gl: &Context| -> (Framebuffer, Texture, Renderbuffer) {
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

                        // Create depth renderbuffer
                        let depth_rb = gl.create_renderbuffer().unwrap();
                        gl.bind_renderbuffer(RENDERBUFFER, Some(depth_rb));
                        gl.renderbuffer_storage(RENDERBUFFER, DEPTH_COMPONENT16, width, height);

                        let framebuffer = gl.create_framebuffer().unwrap();
                        gl.bind_framebuffer(FRAMEBUFFER, Some(framebuffer));
                        gl.framebuffer_texture_2d(
                            FRAMEBUFFER,
                            COLOR_ATTACHMENT0,
                            TEXTURE_2D,
                            Some(texture),
                            0,
                        );
                        gl.framebuffer_renderbuffer(
                            FRAMEBUFFER,
                            DEPTH_ATTACHMENT,
                            RENDERBUFFER,
                            Some(depth_rb),
                        );

                        (framebuffer, texture, depth_rb)
                    };

                // Create all framebuffers
                let (gl_framebuffer, gl_texture) = create_fb_tex(gl);
                let (gpu_framebuffer, gpu_texture) = create_fb_tex(gl);
                let (mesh_framebuffer, mesh_texture, mesh_depth_rb) = create_fb_tex_depth(gl);

                gl.bind_framebuffer(FRAMEBUFFER, None);

                self.gl_texture = Some(gl_texture);
                self.gl_framebuffer = Some(gl_framebuffer);
                self.gpu_texture = Some(gpu_texture);
                self.gpu_framebuffer = Some(gpu_framebuffer);
                self.mesh_texture = Some(mesh_texture);
                self.mesh_depth_rb = Some(mesh_depth_rb);
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
            // Background color with gamma correction to match CPU tracer
            // BACKGROUND_COLOR is (0.4, 0.5, 0.6), gamma corrected: pow(x, 1/2.2)
            let bg_r = 0.4_f32.powf(1.0 / 2.2);
            let bg_g = 0.5_f32.powf(1.0 / 2.2);
            let bg_b = 0.6_f32.powf(1.0 / 2.2);
            gl.clear_color(bg_r, bg_g, bg_b, 1.0);
            gl.clear(COLOR_BUFFER_BIT | DEPTH_BUFFER_BIT);

            let render_start = std::time::Instant::now();

            // Determine if we need to upload the mesh
            let needs_upload = if !self.mesh_cache_enabled {
                // Caching disabled: always regenerate (clear old mesh first)
                if !self.mesh_indices.is_empty() {
                    self.mesh_renderer.clear_meshes(gl);
                    self.mesh_indices.clear();
                }
                true
            } else if self.mesh_needs_regeneration || self.mesh_indices.is_empty() {
                // Caching enabled but mesh needs regeneration
                if !self.mesh_indices.is_empty() {
                    self.mesh_renderer.clear_meshes(gl);
                    self.mesh_indices.clear();
                }
                true
            } else {
                false
            };

            // Upload mesh if needed
            if needs_upload {
                let upload_start = std::time::Instant::now();
                let depth = self.current_cube.max_depth() as u32;
                match self
                    .mesh_renderer
                    .upload_mesh(gl, &self.current_cube, depth)
                {
                    Ok(idx) => {
                        self.mesh_indices.push(idx);
                        self.mesh_upload_time_ms = upload_start.elapsed().as_secs_f32() * 1000.0;
                        self.mesh_needs_regeneration = false;

                        // Update mesh statistics (estimate from cube depth)
                        // For now, use a rough estimate - actual counts would require mesh builder access
                        let grid_size = 1 << depth;
                        self.mesh_vertex_count = grid_size * grid_size * grid_size * 8; // Max vertices
                        self.mesh_face_count = grid_size * grid_size * grid_size * 6; // Max faces
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
                let rotation = glam::Quat::IDENTITY; // Mesh is never rotated

                // Use time-based camera when not in manual camera mode
                // This matches the raytracer's camera orbit behavior
                let camera = if self.use_manual_camera {
                    self.camera
                } else {
                    // Match CPU tracer camera orbit: position orbits at time * 0.3
                    let camera_pos = glam::Vec3::new(
                        3.0 * (time * 0.3).cos(),
                        2.0,
                        3.0 * (time * 0.3).sin(),
                    );
                    let target = glam::Vec3::ZERO;
                    Camera::look_at(camera_pos, target, glam::Vec3::Y)
                };

                // Render at unit scale (mesh is [0,1] space)
                self.mesh_renderer.render_mesh_with_scale(
                    gl,
                    0,
                    position,
                    rotation,
                    1.0,
                    &camera,
                    width as i32,
                    height as i32,
                );
            }

            gl.finish();
            self.mesh_render_time_ms = render_start.elapsed().as_secs_f32() * 1000.0;

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
            model_id: self.current_model_id.clone(),
            fabric_config: if self.current_model_id == "fabric" {
                Some(self.fabric_config.clone())
            } else {
                None
            },
            fabric_max_depth: if self.current_model_id == "fabric" {
                Some(self.fabric_max_depth)
            } else {
                None
            },
            single_voxel_material: if self.current_model_id == "single" {
                Some(self.single_voxel_material)
            } else {
                None
            },
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
            DiffSource::Gpu => self.gpu_latest_frame.as_ref(),
            DiffSource::Mesh => self.mesh_latest_frame.as_ref(),
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

        // Track if model needs reloading
        let mut model_changed = false;
        let mut material_changed = false;

        // Top panel - rendering controls only
        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.heading("Cube Renderer");
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
                let current_model_name = get_model(&self.current_model_id)
                    .map(|m| m.name.as_str())
                    .unwrap_or("Unknown");
                ui.label(format!("Model: {}", current_model_name));
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
                ui.checkbox(&mut self.mesh_cache_enabled, "Cache Mesh");
                if ui.button("Regen Mesh").clicked() {
                    self.mesh_needs_regeneration = true;
                }
            });
        });

        // Left side panel - model selector
        egui::SidePanel::left("model_panel")
            .default_width(200.0)
            .resizable(true)
            .show(ctx, |ui| {
                ui.heading("Models");
                ui.separator();

                egui::ScrollArea::vertical().show(ui, |ui| {
                    // Single Cube category
                    let single_header = egui::CollapsingHeader::new("Single Cube")
                        .default_open(self.model_panel_expanded.single_cube_expanded)
                        .show(ui, |ui| {
                            if ui
                                .selectable_label(self.current_model_id == "single", "Single Voxel")
                                .clicked()
                            {
                                self.current_model_id = "single".to_string();
                                model_changed = true;
                            }

                            // Material selector (only when single is selected)
                            if self.current_model_id == "single" {
                                ui.add_space(4.0);
                                ui.label("Material:");

                                // Preset dropdown
                                egui::ComboBox::from_id_salt("single_material_preset")
                                    .selected_text(format!("{}", self.single_voxel_material))
                                    .show_ui(ui, |ui| {
                                        let presets = [
                                            (0, "Empty"),
                                            (1, "Error: Hot Pink"),
                                            (224, "R2G3B2: Red"),
                                            (252, "R2G3B2: Yellow"),
                                            (156, "R2G3B2: Green"),
                                            (131, "R2G3B2: Blue"),
                                        ];
                                        for (value, name) in presets {
                                            if ui
                                                .selectable_value(
                                                    &mut self.single_voxel_material,
                                                    value,
                                                    name,
                                                )
                                                .clicked()
                                            {
                                                material_changed = true;
                                            }
                                        }
                                    });

                                // Slider for fine control
                                let mut mat_val = self.single_voxel_material as i32;
                                if ui
                                    .add(egui::Slider::new(&mut mat_val, 0..=255).text(""))
                                    .changed()
                                {
                                    self.single_voxel_material = mat_val as u8;
                                    material_changed = true;
                                }
                            }
                        });
                    self.model_panel_expanded.single_cube_expanded = single_header.fully_open();

                    // CSM Models category
                    let csm_header = egui::CollapsingHeader::new("CSM Models")
                        .default_open(self.model_panel_expanded.csm_expanded)
                        .show(ui, |ui| {
                            let csm_models = ["octa", "extended", "depth3", "quad", "layer", "sdf", "generated", "test_expansion"];
                            for model_id in csm_models {
                                if let Some(model) = get_model(model_id)
                                    && ui
                                        .selectable_label(self.current_model_id == model_id, &model.name)
                                        .clicked()
                                {
                                    self.current_model_id = model_id.to_string();
                                    model_changed = true;
                                }
                            }
                        });
                    self.model_panel_expanded.csm_expanded = csm_header.fully_open();

                    // VOX Models category
                    let vox_header = egui::CollapsingHeader::new("VOX Models")
                        .default_open(self.model_panel_expanded.vox_expanded)
                        .show(ui, |ui| {
                            let vox_models = ["vox_alien_bot", "vox_army", "vox_naked"];
                            for model_id in vox_models {
                                if let Some(model) = get_model(model_id)
                                    && ui
                                        .selectable_label(self.current_model_id == model_id, &model.name)
                                        .clicked()
                                {
                                    self.current_model_id = model_id.to_string();
                                    model_changed = true;
                                }
                            }
                        });
                    self.model_panel_expanded.vox_expanded = vox_header.fully_open();

                    // Fabric Models category
                    let fabric_header = egui::CollapsingHeader::new("Fabric Models")
                        .default_open(self.model_panel_expanded.fabric_expanded)
                        .show(ui, |ui| {
                            if ui
                                .selectable_label(self.current_model_id == "fabric", "Procedural Sphere")
                                .clicked()
                            {
                                self.current_model_id = "fabric".to_string();
                                model_changed = true;
                            }

                            // Fabric parameters (only when fabric is selected)
                            if self.current_model_id == "fabric" {
                                ui.add_space(4.0);
                                ui.label("Parameters:");

                                // Max depth slider
                                let mut depth = self.fabric_max_depth as i32;
                                if ui
                                    .add(egui::Slider::new(&mut depth, 1..=7).text("Max Depth"))
                                    .changed()
                                {
                                    self.fabric_max_depth = depth as u32;
                                    model_changed = true;
                                }

                                // Root magnitude
                                if ui
                                    .add(
                                        egui::Slider::new(&mut self.fabric_config.root_magnitude, 0.1..=0.9)
                                            .text("Root Mag"),
                                    )
                                    .changed()
                                {
                                    model_changed = true;
                                }

                                // Boundary magnitude
                                if ui
                                    .add(
                                        egui::Slider::new(&mut self.fabric_config.boundary_magnitude, 1.1..=5.0)
                                            .text("Boundary"),
                                    )
                                    .changed()
                                {
                                    model_changed = true;
                                }

                                // Surface radius
                                if ui
                                    .add(
                                        egui::Slider::new(&mut self.fabric_config.surface_radius, 0.3..=1.5)
                                            .text("Surface R"),
                                    )
                                    .changed()
                                {
                                    model_changed = true;
                                }
                            }
                        });
                    self.model_panel_expanded.fabric_expanded = fabric_header.fully_open();
                });
            });

        // Handle model/material changes
        if model_changed {
            self.reload_model(gl);
        } else if material_changed && self.current_model_id == "single" {
            // Material change for single cube
            use cube::Cube;
            let new_cube = Rc::new(Cube::Solid(self.single_voxel_material));
            self.current_cube = new_cube.clone();
            self.mesh_needs_regeneration = true;

            self.gl_renderer = GlTracer::new(new_cube.clone());
            unsafe {
                if let Err(e) = self.gl_renderer.init_gl(gl) {
                    eprintln!("Failed to reinitialize GL renderer: {}", e);
                }
            }

            self.bcf_cpu_renderer = BcfTracer::new_from_cube(new_cube.clone());

            self.gpu_renderer = ComputeTracer::new(new_cube.clone());
            unsafe {
                if let Err(e) = self.gpu_renderer.init_gl(gl) {
                    eprintln!("Failed to reinitialize GPU renderer: {}", e);
                    self.gpu_render_time_ms = -1.0;
                }
            }
        }

        // Main panel with 3x2 grid
        egui::CentralPanel::default().show(ctx, |ui| {
            // Read frames from framebuffers
            if let Some(gl_img) = unsafe { self.read_framebuffer_to_image(gl, self.gl_framebuffer) }
            {
                self.gl_latest_frame = Some(gl_img);
            }
            if let Some(gpu_img) =
                unsafe { self.read_framebuffer_to_image(gl, self.gpu_framebuffer) }
            {
                self.gpu_latest_frame = Some(gpu_img);
            }
            if let Some(mesh_img) =
                unsafe { self.read_framebuffer_to_image(gl, self.mesh_framebuffer) }
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
                self.gpu_egui_texture =
                    Some(ctx.load_texture("gpu_render", gpu_img.clone(), TextureOptions::LINEAR));
            }
            if let Some(ref mesh_img) = self.mesh_latest_frame {
                self.mesh_egui_texture =
                    Some(ctx.load_texture("mesh_render", mesh_img.clone(), TextureOptions::LINEAR));
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

                    let mesh_response = self.render_mesh_view(ui);
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

    fn render_mesh_view(&self, ui: &mut egui::Ui) -> Option<egui::Response> {
        let mut image_response = None;
        ui.vertical(|ui| {
            ui.heading("Mesh");
            ui.label("Triangle Renderer");
            if self.mesh_render_time_ms >= 0.0 {
                ui.label(format!("Render: {:.2}ms", self.mesh_render_time_ms));
            } else {
                ui.label("Not Available");
            }

            // Cache status
            let cache_status = if !self.mesh_cache_enabled {
                "Cache: Disabled"
            } else if self.mesh_needs_regeneration {
                "Cache: Pending"
            } else {
                "Cache: Active"
            };
            ui.label(cache_status);

            // Upload time (only show if meaningful)
            if self.mesh_upload_time_ms > 0.0 {
                ui.label(format!("Upload: {:.2}ms", self.mesh_upload_time_ms));
            }

            if let Some(tex) = &self.mesh_egui_texture {
                let response = ui.add(
                    egui::Image::from_texture(egui::load::SizedTexture {
                        id: tex.id(),
                        size: [self.render_size.0 as f32, self.render_size.1 as f32].into(),
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
                        ui.selectable_value(
                            &mut self.diff_left,
                            DiffSource::Gpu,
                            DiffSource::Gpu.name(),
                        );
                        ui.selectable_value(
                            &mut self.diff_left,
                            DiffSource::Mesh,
                            DiffSource::Mesh.name(),
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
                        ui.selectable_value(
                            &mut self.diff_right,
                            DiffSource::Gpu,
                            DiffSource::Gpu.name(),
                        );
                        ui.selectable_value(
                            &mut self.diff_right,
                            DiffSource::Mesh,
                            DiffSource::Mesh.name(),
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

    /// Reload the current model into all renderers
    fn reload_model(&mut self, gl: &Arc<Context>) {
        let new_cube = if self.current_model_id == "fabric" {
            // Generate fabric model
            use cube::FabricGenerator;
            let generator = FabricGenerator::new(self.fabric_config.clone());
            let fabric_cube = generator.generate_cube(self.fabric_max_depth);
            // Convert Cube<Quat> to Cube<u8> using surface detection
            Rc::new(fabric_to_material_cube(&fabric_cube, self.fabric_max_depth))
        } else if self.current_model_id == "single" {
            // Single cube with current material
            use cube::Cube;
            Rc::new(Cube::Solid(self.single_voxel_material))
        } else {
            // Load from config
            match create_cube_from_id(&self.current_model_id) {
                Ok(cube) => cube,
                Err(e) => {
                    eprintln!("Failed to load model '{}': {}", self.current_model_id, e);
                    return;
                }
            }
        };

        self.current_cube = new_cube.clone();
        self.mesh_needs_regeneration = true;

        self.gl_renderer = GlTracer::new(new_cube.clone());
        unsafe {
            if let Err(e) = self.gl_renderer.init_gl(gl) {
                eprintln!("Failed to reinitialize GL renderer: {}", e);
            }
        }

        self.bcf_cpu_renderer = BcfTracer::new_from_cube(new_cube.clone());

        self.gpu_renderer = ComputeTracer::new(new_cube.clone());
        unsafe {
            if let Err(e) = self.gpu_renderer.init_gl(gl) {
                eprintln!("Failed to reinitialize GPU renderer: {}", e);
                self.gpu_render_time_ms = -1.0;
            }
        }
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
            if let Some(rb) = self.mesh_depth_rb {
                gl.delete_renderbuffer(rb);
            }
            self.gl_renderer.destroy_gl(gl);
            self.gpu_renderer.destroy_gl(gl);
            self.mesh_renderer.destroy_gl(gl);
            // BCF CPU renderer has no GL resources to destroy
        }
    }
}
