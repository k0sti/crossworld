// MagicaVoxel .vox file loading

pub mod loader;

// Re-export main functions
#[allow(deprecated)]
pub use loader::load_vox_to_cube;
pub use loader::load_vox_to_cubebox;
pub use loader::load_vox_to_cubebox_compact;
