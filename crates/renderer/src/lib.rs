//! Planet CW Renderer Library
//!
//! Provides multiple raytracer implementations for rendering octree-based voxel worlds:
//! - **cpu_tracer**: Pure Rust software raytracer using cube.raycast()
//! - **bcf_cpu_tracer**: CPU raytracer using BCF binary format (GPU-compatible)
//! - **gl_tracer**: WebGL 2.0 fragment shader raytracer with octree support
//! - **gpu_tracer**: Compute shader raytracer for high-performance rendering
//! - **mesh_renderer**: Triangle mesh renderer using OpenGL
//!
//! # Architecture
//!
//! - **camera**: Camera configuration and controls
//! - **lighting**: Lighting constants for consistent rendering
//! - **renderer**: Unified Renderer trait for all implementations
//! - **renderers**: Renderer implementations (CpuTracer, GlTracer, etc.)

// Core modules
pub mod camera;
pub mod lighting;
pub mod renderer;

// Renderer implementations
pub mod renderers;

// Utilities and helpers
pub mod bcf_raycast;
pub mod scenes;
pub mod shader_utils;

// Backward-compatible re-exports (old flat structure)
pub mod bcf_cpu_tracer {
    pub use crate::renderers::bcf_cpu_tracer::*;
}
pub mod cpu_tracer {
    pub use crate::renderers::cpu_tracer::*;
}
pub mod gl_tracer {
    pub use crate::renderers::gl_tracer::*;
}
pub mod gpu_tracer {
    pub use crate::renderers::gpu_tracer::*;
}
pub mod mesh_renderer {
    pub use crate::renderers::mesh_renderer::*;
}

// Re-export commonly used types at crate root
pub use camera::{Camera, DEFAULT_VFOV};
pub use lighting::{AMBIENT, BACKGROUND_COLOR, DIFFUSE_STRENGTH, LIGHT_DIR};
pub use renderer::{Object, Renderer};
pub use renderers::{BcfTracer, ComputeTracer, CpuTracer, GlTracer, MeshRenderer};
pub use scenes::create_octa_cube;

// Backward compatibility alias
pub use camera::Camera as CameraConfig;
