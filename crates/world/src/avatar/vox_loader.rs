use super::voxel_model::{Voxel, VoxelModel, VoxelPalette};
use dot_vox::DotVoxData;

/// Load a .vox file from bytes and convert to VoxelModel
pub fn load_vox_from_bytes(bytes: &[u8]) -> Result<VoxelModel, String> {
    let vox_data =
        dot_vox::load_bytes(bytes).map_err(|e| format!("Failed to parse .vox file: {}", e))?;

    convert_dotvox_to_model(&vox_data)
}

/// Map RGB color (0-255) to closest material index using R2G3B2 encoding
/// Materials 128-255 use 7-bit color encoding: r:2, g:3, b:2
fn map_color_to_material(r: u8, g: u8, b: u8) -> u8 {
    // Quantize RGB to R2G3B2 format
    // Red: 2 bits (0x00, 0x49, 0x92, 0xDB)
    let r_bits = if r < 0x24 {
        0
    } else if r < 0x6D {
        1
    } else if r < 0xB6 {
        2
    } else {
        3
    };

    // Green: 3 bits (0x00, 0x24, 0x49, 0x6D, 0x92, 0xB6, 0xDB, 0xFF)
    let g_bits = if g < 0x12 {
        0
    } else if g < 0x36 {
        1
    } else if g < 0x5B {
        2
    } else if g < 0x7F {
        3
    } else if g < 0xA4 {
        4
    } else if g < 0xC8 {
        5
    } else if g < 0xED {
        6
    } else {
        7
    };

    // Blue: 2 bits (0x00, 0x49, 0x92, 0xDB)
    let b_bits = if b < 0x24 {
        0
    } else if b < 0x6D {
        1
    } else if b < 0xB6 {
        2
    } else {
        3
    };

    // Combine into 7-bit index: rrrgggbb
    let index = (r_bits << 5) | (g_bits << 2) | b_bits;
    128 + index
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
        let palette_index = if voxel.i > 0 { voxel.i - 1 } else { 0 };

        // Get the RGB color from the vox palette
        let color = &vox_data.palette[palette_index as usize];

        // Map vox palette color to closest material color in 128-255 range
        // Uses R2G3B2 encoding to find the nearest matching material
        let color_index = map_color_to_material(color.r, color.g, color.b);

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_color_mapping_black() {
        // Black (0, 0, 0) should map to index 128 (binary: 10000000)
        assert_eq!(map_color_to_material(0, 0, 0), 128);
    }

    #[test]
    fn test_color_mapping_white() {
        // White (255, 255, 255) should map to index 255 (binary: 11111111)
        // r=3 (11), g=7 (111), b=3 (11) -> 11111111 = 127 + 128
        assert_eq!(map_color_to_material(255, 255, 255), 255);
    }

    #[test]
    fn test_color_mapping_red() {
        // Pure red (0xDB, 0, 0) should have r_bits=3, g_bits=0, b_bits=0
        // Result: 11100000 = 224
        assert_eq!(map_color_to_material(0xDB, 0, 0), 224);
    }

    #[test]
    fn test_color_mapping_green() {
        // Pure green (0, 0xFF, 0) should have r_bits=0, g_bits=7, b_bits=0
        // Result: 00011100 = 28 + 128 = 156
        assert_eq!(map_color_to_material(0, 0xFF, 0), 156);
    }

    #[test]
    fn test_color_mapping_blue() {
        // Pure blue (0, 0, 0xDB) should have r_bits=0, g_bits=0, b_bits=3
        // Result: 00000011 = 3 + 128 = 131
        assert_eq!(map_color_to_material(0, 0, 0xDB), 131);
    }

    #[test]
    fn test_color_mapping_threshold_boundaries() {
        // Test boundary conditions for quantization
        // Red bit 0->1 boundary at 0x24
        assert_eq!(map_color_to_material(0x23, 0, 0), 128); // r_bits=0
        assert_eq!(map_color_to_material(0x24, 0, 0), 160); // r_bits=1

        // Green bit 0->1 boundary at 0x12
        assert_eq!(map_color_to_material(0, 0x11, 0), 128); // g_bits=0
        assert_eq!(map_color_to_material(0, 0x12, 0), 132); // g_bits=1

        // Blue bit 0->1 boundary at 0x24
        assert_eq!(map_color_to_material(0, 0, 0x23), 128); // b_bits=0
        assert_eq!(map_color_to_material(0, 0, 0x24), 129); // b_bits=1
    }

    #[test]
    fn test_color_mapping_mid_values() {
        // Test a mid-range color (127, 127, 127)
        // r: 127 (0x7F) -> between 0x6D and 0xB6, so r_bits=2
        // g: 127 (0x7F) -> between 0x7F and 0xA4, so g_bits=4
        // b: 127 (0x7F) -> between 0x6D and 0xB6, so b_bits=2
        // Result: 01010010 = 82 + 128 = 210
        assert_eq!(map_color_to_material(127, 127, 127), 210);
    }
}
