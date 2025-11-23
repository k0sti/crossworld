//! Planet CW Renderer Library
//!
//! Provides multiple raytracer implementations for rendering octree-based voxel worlds:
//! - **cpu_tracer**: Pure Rust software raytracer using cube.raycast()
//! - **gl_tracer**: WebGL 2.0 fragment shader raytracer with octree support
//! - **gpu_tracer**: Compute shader raytracer for high-performance rendering

pub mod cpu_tracer;
pub mod gl_tracer;
pub mod gpu_tracer;
pub mod materials;
pub mod renderer;
pub mod scenes;
pub mod shader_utils;

// Re-export commonly used types
pub use cpu_tracer::CpuCubeTracer;
pub use gl_tracer::GlCubeTracer;
pub use gpu_tracer::GpuTracer;
pub use materials::{get_material_color, MATERIAL_PALETTE};
pub use renderer::{CameraConfig, Renderer, AMBIENT, BACKGROUND_COLOR, DIFFUSE_STRENGTH, LIGHT_DIR};
pub use scenes::create_octa_cube;
