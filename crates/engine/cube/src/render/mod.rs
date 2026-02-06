// Orthographic rendering to 2D images

pub mod orthographic;

// Re-export main types and functions
pub use orthographic::{
    render_orthographic, render_orthographic_2d, render_orthographic_3d, RenderedImage,
    ViewDirection,
};
