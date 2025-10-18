use super::voxel_model::{Voxel, VoxelModel, VoxelPalette};
use dot_vox::DotVoxData;

/// Load a .vox file from bytes and convert to VoxelModel
pub fn load_vox_from_bytes(bytes: &[u8]) -> Result<VoxelModel, String> {
    let vox_data =
        dot_vox::load_bytes(bytes).map_err(|e| format!("Failed to parse .vox file: {}", e))?;

    convert_dotvox_to_model(&vox_data)
}

/// Convert DotVoxData to our VoxelModel format
fn convert_dotvox_to_model(vox_data: &DotVoxData) -> Result<VoxelModel, String> {
    if vox_data.models.is_empty() {
        return Err("No models found in .vox file".to_string());
    }

    // Use the first model
    let dot_vox_model = &vox_data.models[0];

    // Get model size
    // MagicaVoxel uses Z-up, we use Y-up, so swap dimensions
    let size = dot_vox_model.size;
    let size_x = size.x as u8;
    let size_y = size.z as u8; // MagicaVoxel's Z becomes our Y (height)
    let size_z = size.y as u8; // MagicaVoxel's Y becomes our Z (depth)

    // Create our model
    let mut model = VoxelModel::new(size_x, size_y, size_z);

    // Convert palette
    model.palette = convert_palette(&vox_data.palette);

    // Convert voxels
    // MagicaVoxel uses Z-up coordinate system, we use Y-up
    // So we swap Y and Z: MagicaVoxel (x, y, z) -> Our system (x, z, y)
    for voxel in &dot_vox_model.voxels {
        // MagicaVoxel uses 1-based indexing for colors, we use 0-based
        let color_index = if voxel.i > 0 { voxel.i - 1 } else { 0 };

        model.add_voxel(Voxel {
            x: voxel.x,
            y: voxel.z, // MagicaVoxel's Z becomes our Y (up)
            z: voxel.y, // MagicaVoxel's Y becomes our Z (depth)
            color_index,
        });
    }

    Ok(model)
}

/// Convert DotVox palette to our VoxelPalette
fn convert_palette(palette: &[dot_vox::Color]) -> VoxelPalette {
    let mut voxel_palette = VoxelPalette::new();

    for color in palette.iter() {
        // dot_vox Color has r, g, b, a fields (u8)
        let r = color.r as f32 / 255.0;
        let g = color.g as f32 / 255.0;
        let b = color.b as f32 / 255.0;
        // We ignore alpha for now

        voxel_palette.add_color(r, g, b);
    }

    voxel_palette
}
