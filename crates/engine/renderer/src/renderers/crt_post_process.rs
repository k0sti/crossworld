//! CRT Post-Processing Effect
//!
//! Applies CRT monitor simulation effects to rendered scenes:
//! - Scanlines with configurable intensity and count
//! - Screen curvature distortion
//! - Vignette darkening at edges
//! - Bloom/glow effects
//! - RGB chromatic aberration
//! - Brightness, contrast, saturation adjustments
//! - CRT flicker animation
//!
//! # Usage
//!
//! ```ignore
//! let mut crt = CrtPostProcess::new();
//! crt.init_gl(gl)?;
//!
//! // In render loop:
//! crt.begin(gl, width, height);  // Redirects rendering to internal framebuffer
//! // ... render your scene here ...
//! crt.end(gl, width, height, time);  // Applies CRT effect and draws to screen
//! ```
//!
//! Based on https://github.com/gingerbeardman/webgl-crt-shader

use crate::shader_utils;
use glow::*;

// Shader sources
const VERTEX_SHADER_SOURCE: &str = include_str!("../shaders/crt.vert");
const FRAGMENT_SHADER_SOURCE: &str = include_str!("../shaders/crt.frag");

/// CRT effect configuration parameters
#[derive(Debug, Clone)]
pub struct CrtConfig {
    /// Intensity of scanline darkening (0.0 = off, 1.0 = max)
    pub scanline_intensity: f32,
    /// Number of scanlines across the screen height
    pub scanline_count: f32,
    /// Vertical offset for scanline animation
    pub y_offset: f32,
    /// Brightness multiplier (1.0 = normal)
    pub brightness: f32,
    /// Contrast adjustment (1.0 = normal)
    pub contrast: f32,
    /// Saturation adjustment (1.0 = normal)
    pub saturation: f32,
    /// Bloom glow intensity (0.0 = off)
    pub bloom_intensity: f32,
    /// Luminance threshold for bloom effect
    pub bloom_threshold: f32,
    /// RGB chromatic aberration shift amount
    pub rgb_shift: f32,
    /// Adaptive scanline intensity based on Y position
    pub adaptive_intensity: f32,
    /// Edge vignette darkening strength
    pub vignette_strength: f32,
    /// Screen curvature amount (0.0 = flat)
    pub curvature: f32,
    /// CRT flicker intensity
    pub flicker_strength: f32,
}

impl Default for CrtConfig {
    fn default() -> Self {
        Self {
            scanline_intensity: 0.15,
            scanline_count: 400.0,
            y_offset: 0.0,
            brightness: 1.1,
            contrast: 1.05,
            saturation: 1.1,
            bloom_intensity: 0.2,
            bloom_threshold: 0.5,
            rgb_shift: 0.0,
            adaptive_intensity: 0.5,
            vignette_strength: 0.3,
            curvature: 0.15,
            flicker_strength: 0.01,
        }
    }
}

impl CrtConfig {
    /// Create a subtle CRT effect configuration
    pub fn subtle() -> Self {
        Self {
            scanline_intensity: 0.08,
            scanline_count: 300.0,
            curvature: 0.05,
            vignette_strength: 0.15,
            bloom_intensity: 0.1,
            flicker_strength: 0.005,
            ..Default::default()
        }
    }

    /// Create an intense retro CRT effect configuration
    pub fn retro() -> Self {
        Self {
            scanline_intensity: 0.25,
            scanline_count: 240.0,
            curvature: 0.25,
            vignette_strength: 0.5,
            bloom_intensity: 0.3,
            rgb_shift: 0.5,
            flicker_strength: 0.02,
            ..Default::default()
        }
    }

    /// Create a configuration with no effects (passthrough)
    pub fn disabled() -> Self {
        Self {
            scanline_intensity: 0.0,
            bloom_intensity: 0.0,
            rgb_shift: 0.0,
            vignette_strength: 0.0,
            curvature: 0.0,
            flicker_strength: 0.0,
            brightness: 1.0,
            contrast: 1.0,
            saturation: 1.0,
            ..Default::default()
        }
    }
}

/// CRT post-processing effect renderer
///
/// Renders the scene to an internal framebuffer, then applies CRT effects
/// when drawing to the screen.
pub struct CrtPostProcess {
    /// Effect configuration
    pub config: CrtConfig,
    /// Whether the effect is enabled
    pub enabled: bool,

    // GL resources (None until init_gl is called)
    gl_resources: Option<CrtGlResources>,

    // Current framebuffer dimensions
    fb_width: u32,
    fb_height: u32,
}

/// OpenGL resources for CRT post-processing
struct CrtGlResources {
    program: Program,
    vao: VertexArray,
    framebuffer: Framebuffer,
    color_texture: Texture,
    depth_renderbuffer: Renderbuffer,

    // Uniform locations
    t_diffuse_loc: Option<UniformLocation>,
    scanline_intensity_loc: Option<UniformLocation>,
    scanline_count_loc: Option<UniformLocation>,
    time_loc: Option<UniformLocation>,
    y_offset_loc: Option<UniformLocation>,
    brightness_loc: Option<UniformLocation>,
    contrast_loc: Option<UniformLocation>,
    saturation_loc: Option<UniformLocation>,
    bloom_intensity_loc: Option<UniformLocation>,
    bloom_threshold_loc: Option<UniformLocation>,
    rgb_shift_loc: Option<UniformLocation>,
    adaptive_intensity_loc: Option<UniformLocation>,
    vignette_strength_loc: Option<UniformLocation>,
    curvature_loc: Option<UniformLocation>,
    flicker_strength_loc: Option<UniformLocation>,
}

impl Default for CrtPostProcess {
    fn default() -> Self {
        Self::new()
    }
}

impl CrtPostProcess {
    /// Create a new CRT post-processor with default configuration
    pub fn new() -> Self {
        Self {
            config: CrtConfig::default(),
            enabled: true,
            gl_resources: None,
            fb_width: 0,
            fb_height: 0,
        }
    }

    /// Create with a specific configuration
    pub fn with_config(config: CrtConfig) -> Self {
        Self {
            config,
            enabled: true,
            gl_resources: None,
            fb_width: 0,
            fb_height: 0,
        }
    }

    /// Initialize OpenGL resources
    ///
    /// # Safety
    /// Must be called with an active GL context.
    pub unsafe fn init_gl(&mut self, gl: &Context) -> Result<(), String> {
        unsafe {
            println!("[CRT] Compiling CRT post-processing shaders...");
            let program =
                shader_utils::create_program(gl, VERTEX_SHADER_SOURCE, FRAGMENT_SHADER_SOURCE)?;
            println!("[CRT] ✓ Shaders compiled successfully!");

            // Create VAO for fullscreen triangle
            let vao = gl
                .create_vertex_array()
                .map_err(|e| format!("Failed to create VAO: {}", e))?;

            // Create framebuffer
            let framebuffer = gl
                .create_framebuffer()
                .map_err(|e| format!("Failed to create framebuffer: {}", e))?;

            // Create color texture (will be resized on first use)
            let color_texture = gl
                .create_texture()
                .map_err(|e| format!("Failed to create color texture: {}", e))?;

            // Create depth renderbuffer
            let depth_renderbuffer = gl
                .create_renderbuffer()
                .map_err(|e| format!("Failed to create depth renderbuffer: {}", e))?;

            // Get uniform locations
            let t_diffuse_loc = gl.get_uniform_location(program, "tDiffuse");
            let scanline_intensity_loc = gl.get_uniform_location(program, "scanlineIntensity");
            let scanline_count_loc = gl.get_uniform_location(program, "scanlineCount");
            let time_loc = gl.get_uniform_location(program, "time");
            let y_offset_loc = gl.get_uniform_location(program, "yOffset");
            let brightness_loc = gl.get_uniform_location(program, "brightness");
            let contrast_loc = gl.get_uniform_location(program, "contrast");
            let saturation_loc = gl.get_uniform_location(program, "saturation");
            let bloom_intensity_loc = gl.get_uniform_location(program, "bloomIntensity");
            let bloom_threshold_loc = gl.get_uniform_location(program, "bloomThreshold");
            let rgb_shift_loc = gl.get_uniform_location(program, "rgbShift");
            let adaptive_intensity_loc = gl.get_uniform_location(program, "adaptiveIntensity");
            let vignette_strength_loc = gl.get_uniform_location(program, "vignetteStrength");
            let curvature_loc = gl.get_uniform_location(program, "curvature");
            let flicker_strength_loc = gl.get_uniform_location(program, "flickerStrength");

            self.gl_resources = Some(CrtGlResources {
                program,
                vao,
                framebuffer,
                color_texture,
                depth_renderbuffer,
                t_diffuse_loc,
                scanline_intensity_loc,
                scanline_count_loc,
                time_loc,
                y_offset_loc,
                brightness_loc,
                contrast_loc,
                saturation_loc,
                bloom_intensity_loc,
                bloom_threshold_loc,
                rgb_shift_loc,
                adaptive_intensity_loc,
                vignette_strength_loc,
                curvature_loc,
                flicker_strength_loc,
            });

            println!("[CRT] Post-processing initialized");
            Ok(())
        }
    }

    /// Resize the internal framebuffer if needed
    ///
    /// # Safety
    /// Must be called with an active GL context.
    unsafe fn ensure_framebuffer_size(&mut self, gl: &Context, width: u32, height: u32) {
        if self.fb_width == width && self.fb_height == height {
            return;
        }

        let Some(res) = &self.gl_resources else {
            return;
        };

        unsafe {
            // Resize color texture
            gl.bind_texture(TEXTURE_2D, Some(res.color_texture));
            gl.tex_image_2d(
                TEXTURE_2D,
                0,
                RGBA8 as i32,
                width as i32,
                height as i32,
                0,
                RGBA,
                UNSIGNED_BYTE,
                PixelUnpackData::Slice(None::<&[u8]>),
            );
            gl.tex_parameter_i32(TEXTURE_2D, TEXTURE_MIN_FILTER, LINEAR as i32);
            gl.tex_parameter_i32(TEXTURE_2D, TEXTURE_MAG_FILTER, LINEAR as i32);
            gl.tex_parameter_i32(TEXTURE_2D, TEXTURE_WRAP_S, CLAMP_TO_EDGE as i32);
            gl.tex_parameter_i32(TEXTURE_2D, TEXTURE_WRAP_T, CLAMP_TO_EDGE as i32);

            // Resize depth renderbuffer
            gl.bind_renderbuffer(RENDERBUFFER, Some(res.depth_renderbuffer));
            gl.renderbuffer_storage(RENDERBUFFER, DEPTH_COMPONENT24, width as i32, height as i32);

            // Attach to framebuffer
            gl.bind_framebuffer(FRAMEBUFFER, Some(res.framebuffer));
            gl.framebuffer_texture_2d(
                FRAMEBUFFER,
                COLOR_ATTACHMENT0,
                TEXTURE_2D,
                Some(res.color_texture),
                0,
            );
            gl.framebuffer_renderbuffer(
                FRAMEBUFFER,
                DEPTH_ATTACHMENT,
                RENDERBUFFER,
                Some(res.depth_renderbuffer),
            );

            // Check framebuffer status
            let status = gl.check_framebuffer_status(FRAMEBUFFER);
            if status != FRAMEBUFFER_COMPLETE {
                eprintln!("[CRT] Warning: Framebuffer incomplete, status: {}", status);
            }

            gl.bind_framebuffer(FRAMEBUFFER, None);
            gl.bind_texture(TEXTURE_2D, None);
            gl.bind_renderbuffer(RENDERBUFFER, None);
        }

        self.fb_width = width;
        self.fb_height = height;
    }

    /// Begin rendering to the CRT framebuffer
    ///
    /// Call this before rendering your scene. All subsequent draw calls
    /// will be captured to the internal framebuffer until `end()` is called.
    ///
    /// When CRT is disabled, this ensures rendering goes to the default framebuffer.
    ///
    /// # Safety
    /// Must be called with an active GL context.
    pub unsafe fn begin(&mut self, gl: &Context, width: u32, height: u32) {
        unsafe {
            if !self.enabled || self.gl_resources.is_none() {
                // When disabled, ensure we're rendering to the default framebuffer
                gl.bind_framebuffer(FRAMEBUFFER, None);
                gl.viewport(0, 0, width as i32, height as i32);
                return;
            }

            self.ensure_framebuffer_size(gl, width, height);

            if let Some(res) = &self.gl_resources {
                gl.bind_framebuffer(FRAMEBUFFER, Some(res.framebuffer));
                gl.viewport(0, 0, width as i32, height as i32);
            }
        }
    }

    /// End CRT rendering and apply effects to screen
    ///
    /// Call this after rendering your scene. This will apply the CRT
    /// post-processing effects and draw the result to the default framebuffer.
    ///
    /// # Safety
    /// Must be called with an active GL context.
    pub unsafe fn end(&mut self, gl: &Context, width: u32, height: u32, time: f32) {
        if !self.enabled {
            return;
        }

        let Some(res) = &self.gl_resources else {
            return;
        };

        unsafe {
            // Bind default framebuffer
            gl.bind_framebuffer(FRAMEBUFFER, None);
            gl.viewport(0, 0, width as i32, height as i32);

            // Clear (optional, since we're drawing fullscreen)
            gl.clear_color(0.0, 0.0, 0.0, 1.0);
            gl.clear(COLOR_BUFFER_BIT);

            // Disable depth testing for fullscreen quad
            gl.disable(DEPTH_TEST);

            // Use CRT shader
            gl.use_program(Some(res.program));
            gl.bind_vertex_array(Some(res.vao));

            // Bind scene texture
            gl.active_texture(TEXTURE0);
            gl.bind_texture(TEXTURE_2D, Some(res.color_texture));
            if let Some(loc) = &res.t_diffuse_loc {
                gl.uniform_1_i32(Some(loc), 0);
            }

            // Set uniforms
            if let Some(loc) = &res.scanline_intensity_loc {
                gl.uniform_1_f32(Some(loc), self.config.scanline_intensity);
            }
            if let Some(loc) = &res.scanline_count_loc {
                gl.uniform_1_f32(Some(loc), self.config.scanline_count);
            }
            if let Some(loc) = &res.time_loc {
                gl.uniform_1_f32(Some(loc), time);
            }
            if let Some(loc) = &res.y_offset_loc {
                gl.uniform_1_f32(Some(loc), self.config.y_offset);
            }
            if let Some(loc) = &res.brightness_loc {
                gl.uniform_1_f32(Some(loc), self.config.brightness);
            }
            if let Some(loc) = &res.contrast_loc {
                gl.uniform_1_f32(Some(loc), self.config.contrast);
            }
            if let Some(loc) = &res.saturation_loc {
                gl.uniform_1_f32(Some(loc), self.config.saturation);
            }
            if let Some(loc) = &res.bloom_intensity_loc {
                gl.uniform_1_f32(Some(loc), self.config.bloom_intensity);
            }
            if let Some(loc) = &res.bloom_threshold_loc {
                gl.uniform_1_f32(Some(loc), self.config.bloom_threshold);
            }
            if let Some(loc) = &res.rgb_shift_loc {
                gl.uniform_1_f32(Some(loc), self.config.rgb_shift);
            }
            if let Some(loc) = &res.adaptive_intensity_loc {
                gl.uniform_1_f32(Some(loc), self.config.adaptive_intensity);
            }
            if let Some(loc) = &res.vignette_strength_loc {
                gl.uniform_1_f32(Some(loc), self.config.vignette_strength);
            }
            if let Some(loc) = &res.curvature_loc {
                gl.uniform_1_f32(Some(loc), self.config.curvature);
            }
            if let Some(loc) = &res.flicker_strength_loc {
                gl.uniform_1_f32(Some(loc), self.config.flicker_strength);
            }

            // Draw fullscreen triangle
            gl.draw_arrays(TRIANGLES, 0, 3);

            // Re-enable depth testing
            gl.enable(DEPTH_TEST);

            // Cleanup
            gl.bind_texture(TEXTURE_2D, None);
        }
    }

    /// Clean up OpenGL resources
    ///
    /// # Safety
    /// Must be called with an active GL context.
    pub unsafe fn destroy_gl(&mut self, gl: &Context) {
        if let Some(res) = self.gl_resources.take() {
            unsafe {
                gl.delete_program(res.program);
                gl.delete_vertex_array(res.vao);
                gl.delete_framebuffer(res.framebuffer);
                gl.delete_texture(res.color_texture);
                gl.delete_renderbuffer(res.depth_renderbuffer);
            }
        }
        self.fb_width = 0;
        self.fb_height = 0;
    }

    /// Check if GL resources are initialized
    pub fn is_initialized(&self) -> bool {
        self.gl_resources.is_some()
    }
}
