//! Planet CW Renderer Library
//!
//! Provides multiple raytracer implementations for rendering octree-based voxel worlds:
//! - **cpu_tracer**: Pure Rust software raytracer using cube.raycast()
//! - **bcf_cpu_tracer**: CPU raytracer using BCF binary format (GPU-compatible)
//! - **gl_tracer**: WebGL 2.0 fragment shader raytracer with octree support
//! - **gpu_tracer**: Compute shader raytracer for high-performance rendering

pub mod bcf_cpu_tracer;
pub mod bcf_raycast;
pub mod cpu_tracer;
pub mod gl_tracer;
pub mod gpu_tracer;
pub mod mesh_renderer;

pub mod renderer;
pub mod scenes;
pub mod shader_utils;

// Re-export commonly used types
pub use bcf_cpu_tracer::BcfTracer;
pub use cpu_tracer::CpuTracer;
pub use gl_tracer::GlTracer;
pub use gpu_tracer::ComputeTracer;
pub use mesh_renderer::MeshRenderer;

pub use renderer::{
    AMBIENT, BACKGROUND_COLOR, CameraConfig, CubeObject, DIFFUSE_STRENGTH, Entity, LIGHT_DIR,
    Renderer,
};
pub use scenes::create_octa_cube;
