//! Renderer implementations for voxel octrees
//!
//! This module contains all renderer implementations supporting different rendering backends:
//! - Software renderers (CPU-based, output to image buffer)
//! - GL-based renderers (GPU-accelerated, output to framebuffer)
//! - Post-processing effects (CRT simulation, etc.)
//!
//! All renderers implement the `Renderer` trait from the parent module, providing
//! a unified interface for initialization, rendering, and resource cleanup.
//!
//! # Software Renderers
//!
//! - [`CpuTracer`] - Pure Rust software raytracer using octree traversal
//! - [`BcfTracer`] - CPU raytracer using Binary Cube Format (BCF)
//!
//! Software renderers render to an internal image buffer accessible via `image_buffer()`.
//!
//! # GL-Based Renderers
//!
//! - [`GlTracer`] - Fragment shader raytracer with octree support (WebGL 2.0)
//! - [`ComputeTracer`] - Compute shader raytracer for high-performance rendering
//! - [`MeshRenderer`] - Triangle mesh renderer with standard OpenGL pipeline
//!
//! GL renderers require an OpenGL context and render to the bound framebuffer.
//! Initialize with `init_gl()`, render with `render_to_framebuffer()`, and cleanup with `destroy_gl()`.
//!
//! # Post-Processing Effects
//!
//! - [`CrtPostProcess`] - CRT monitor simulation (scanlines, curvature, bloom, etc.)

pub mod bcf_cpu_tracer;
pub mod cpu_tracer;
pub mod crt_post_process;
pub mod gl_tracer;
pub mod gpu_tracer;
pub mod mesh_renderer;
pub mod skybox_renderer;

pub use bcf_cpu_tracer::BcfTracer;
pub use cpu_tracer::CpuTracer;
pub use crt_post_process::{CrtConfig, CrtPostProcess};
pub use gl_tracer::GlTracer;
pub use gpu_tracer::ComputeTracer;
pub use mesh_renderer::{MeshRenderer, WireframeDepthMode};
pub use skybox_renderer::SkyboxRenderer;
