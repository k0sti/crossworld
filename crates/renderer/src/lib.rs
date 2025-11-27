//! Planet CW Renderer Library
//!
//! Provides multiple raytracer implementations for rendering octree-based voxel worlds:
//! - **cpu_tracer**: Pure Rust software raytracer using cube.raycast()
//! - **bcf_cpu_tracer**: CPU raytracer using BCF binary format (GPU-compatible)
//! - **gl_tracer**: WebGL 2.0 fragment shader raytracer with octree support
//! - **gpu_tracer**: Compute shader raytracer for high-performance rendering

pub mod bcf_cpu_tracer;
pub mod cpu_tracer;
pub mod gl_tracer;
pub mod gpu_tracer;

pub mod renderer;
pub mod scenes;
pub mod shader_utils;

// Re-export commonly used types
pub use bcf_cpu_tracer::BcfCpuTracer;
pub use cpu_tracer::CpuCubeTracer;
pub use gl_tracer::GlCubeTracer;
pub use gpu_tracer::GpuTracer;

pub use renderer::{
    AMBIENT, BACKGROUND_COLOR, CameraConfig, DIFFUSE_STRENGTH, LIGHT_DIR, Renderer,
};
pub use scenes::create_octa_cube;
