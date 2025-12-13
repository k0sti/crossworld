use crate::core::Cube;
use crate::CubeCoord;
use dot_vox::DotVoxData;
use glam::{IVec3, Vec3};

/// Map RGB color (0-255) to material index using R2G3B2 encoding
/// Materials 128-255 use 7-bit color encoding: r:2, g:3, b:2
#[inline]
fn map_color_to_material(r: u8, g: u8, b: u8) -> u8 {
    // Extract top bits: red (2 bits), green (3 bits), blue (2 bits)
    // R2G3B2: rrrgggbb
    let r_bits = (r >> 6) & 0b11; // Top 2 bits of red
    let g_bits = (g >> 5) & 0b111; // Top 3 bits of green
    let b_bits = (b >> 6) & 0b11; // Top 2 bits of blue

    // Combine into 7-bit index and add 128 offset
    128 + ((r_bits << 5) | (g_bits << 2) | b_bits)
}

/// Load a .vox file from bytes and convert to a Cube with specified alignment
///
/// # Arguments
/// * `bytes` - VOX file bytes
/// * `align` - Alignment factor for positioning (0.0 to 1.0 per axis)
///   - 0.0: Model starts at coordinate 0
///   - 0.5: Model is centered
///   - 1.0: Model ends at maximum coordinate
///
/// # Returns
/// A Cube<u8> where material IDs (128-255) are stored as u8 values
/// The cube depth is automatically calculated to fit the model dimensions
pub fn load_vox_to_cube(bytes: &[u8], align: Vec3) -> Result<Cube<u8>, String> {
    let vox_data =
        dot_vox::load_bytes(bytes).map_err(|e| format!("Failed to parse .vox file: {}", e))?;

    convert_dotvox_to_cube(&vox_data, align)
}

/// Convert DotVoxData to a Cube structure with alignment
fn convert_dotvox_to_cube(vox_data: &DotVoxData, align: Vec3) -> Result<Cube<u8>, String> {
    if vox_data.models.is_empty() {
        return Err("No models found in .vox file".to_string());
    }

    // Use the first model
    let dot_vox_model = &vox_data.models[0];

    eprintln!("[VOX Loading] Number of voxels in file: {}", dot_vox_model.voxels.len());

    // Get model size (MagicaVoxel coordinates)
    let vox_size = dot_vox_model.size;
    let model_width = vox_size.x;
    let model_height = vox_size.z; // MagicaVoxel's Z becomes our Y (height)
    let model_depth = vox_size.y; // MagicaVoxel's Y becomes our Z (depth)

    eprintln!("[VOX Loading] Model dimensions: {}x{}x{} (width x height x depth)", model_width, model_height, model_depth);

    // Calculate required depth from model dimensions
    let max_model_size = model_width.max(model_height).max(model_depth);
    let depth = if max_model_size == 0 {
        0
    } else {
        // Calculate log2 and round up to get minimum depth
        (max_model_size as f32).log2().ceil() as u8
    };
    let cube_size = 1 << depth;

    eprintln!("[VOX Loading] Calculated depth: {}, cube size: {}^3", depth, cube_size);

    // Clamp alignment to valid range
    let align = align.clamp(Vec3::ZERO, Vec3::ONE);

    // Calculate offset based on alignment
    let offset_x = ((cube_size - model_width) as f32 * align.x) as i32;
    let offset_y = ((cube_size - model_height) as f32 * align.y) as i32;
    let offset_z = ((cube_size - model_depth) as f32 * align.z) as i32;

    eprintln!("[VOX Loading] Offsets: x={}, y={}, z={}", offset_x, offset_y, offset_z);

    // Create cube with empty material (0)
    let mut cube = Cube::solid(0u8);

    // Track materials for debugging
    let mut materials_used = std::collections::HashSet::new();
    let mut sample_voxels = Vec::new();

    // Convert voxels with coordinate transformation and alignment
    for (idx, voxel) in dot_vox_model.voxels.iter().enumerate() {
        // MagicaVoxel uses 1-based indexing for colors, we use 0-based
        let palette_index = if voxel.i > 0 { voxel.i - 1 } else { 0 };

        // Get the RGB color from the vox palette
        let color = &vox_data.palette[palette_index as usize];

        // Map vox palette color to material index using bit operations
        let material = map_color_to_material(color.r, color.g, color.b);
        materials_used.insert(material);

        // Transform coordinates:
        // MagicaVoxel (x, y, z) -> Our system (x, z, y) with Y-up
        // Then apply alignment offset
        let x = voxel.x as i32 + offset_x;
        let y = voxel.z as i32 + offset_y; // MagicaVoxel's Z becomes our Y (up)
        let z = voxel.y as i32 + offset_z; // MagicaVoxel's Y becomes our Z (depth)

        // Collect first few samples for debugging
        if sample_voxels.len() < 5 {
            sample_voxels.push((idx, x, y, z, material, color.r, color.g, color.b));
        }

        // Set voxel in cube using immutable set_voxel
        let depth = depth as u32;
        cube = cube.update(
            CubeCoord {
                pos: IVec3 { x, y, z },
                depth,
            },
            Cube::solid(material),
        );
    }

    eprintln!("[VOX Loading] Materials used during loading: {} unique", materials_used.len());
    eprintln!("[VOX Loading] Sample voxels:");
    for (idx, x, y, z, mat, r, g, b) in sample_voxels {
        eprintln!("  Voxel {}: pos=({},{},{}), material={}, RGB=({},{},{})", idx, x, y, z, mat, r, g, b);
    }

    Ok(cube)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_color_mapping_black() {
        // Black (0,0,0): r=00, g=000, b=00 -> 00000000 = 0 + 128 = 128
        assert_eq!(map_color_to_material(0, 0, 0), 128);
    }

    #[test]
    fn test_color_mapping_white() {
        // White (255,255,255): r=11, g=111, b=11 -> 11111111 = 127 + 128 = 255
        assert_eq!(map_color_to_material(255, 255, 255), 255);
    }

    #[test]
    fn test_color_mapping_red() {
        // Red (255,0,0): r=11, g=000, b=00 -> 11000000 = 96 + 128 = 224
        assert_eq!(map_color_to_material(255, 0, 0), 224);
    }

    #[test]
    fn test_color_mapping_green() {
        // Green (0,255,0): r=00, g=111, b=00 -> 00111100 = 28 + 128 = 156
        assert_eq!(map_color_to_material(0, 255, 0), 156);
    }

    #[test]
    fn test_color_mapping_blue() {
        // Blue (0,0,255): r=00, g=000, b=11 -> 00000011 = 3 + 128 = 131
        assert_eq!(map_color_to_material(0, 0, 255), 131);
    }

    #[test]
    fn test_color_mapping_bit_extraction() {
        // Test that bit extraction works correctly
        // RGB(192, 224, 192) = (11000000, 11100000, 11000000)
        // r_bits = 11 (top 2 bits of 192)
        // g_bits = 111 (top 3 bits of 224)
        // b_bits = 11 (top 2 bits of 192)
        // Result: 11111111 = 127 + 128 = 255
        assert_eq!(map_color_to_material(192, 224, 192), 255);
    }

    #[test]
    fn test_color_mapping_mid_values() {
        // RGB(128, 128, 128) = (10000000, 10000000, 10000000)
        // r_bits = 10 (2)
        // g_bits = 100 (4)
        // b_bits = 10 (2)
        // Result: 10100010 = 82 + 128 = 210
        assert_eq!(map_color_to_material(128, 128, 128), 210);
    }
}
