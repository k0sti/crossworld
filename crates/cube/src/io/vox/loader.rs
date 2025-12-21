use crate::core::cube::Voxel;
use crate::core::{Cube, CubeBox};
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

/// Load a .vox file from bytes and convert to a CubeBox with bounds preserved.
///
/// The model is always positioned at origin (0,0,0) within the octree.
/// Use `CubeBox::place_in()` to position the model with desired alignment.
///
/// # Arguments
/// * `bytes` - VOX file bytes
///
/// # Returns
/// A CubeBox<u8> containing:
/// - The octree with voxel data
/// - Original model dimensions (size) from the .vox file header
/// - Octree depth
///
/// # Note
/// This uses the canvas size from the .vox file. If the model has empty space
/// around it, use `load_vox_to_cubebox_compact()` instead to compute tight bounds.
///
/// # Example
/// ```ignore
/// let cubebox = load_vox_to_cubebox(bytes)?;
/// println!("Model size: {:?}", cubebox.size);  // e.g., (16, 30, 12)
/// println!("Octree size: {}", cubebox.octree_size());  // e.g., 32
/// ```
pub fn load_vox_to_cubebox(bytes: &[u8]) -> Result<CubeBox<u8>, String> {
    let vox_data =
        dot_vox::load_bytes(bytes).map_err(|e| format!("Failed to parse .vox file: {}", e))?;

    convert_dotvox_to_cubebox(&vox_data, false)
}

/// Load a .vox file with compact bounds calculated from actual voxel positions.
///
/// Unlike `load_vox_to_cubebox()`, this computes the actual min/max bounds of
/// voxels in the model, eliminating any empty space around the model.
///
/// The voxels are shifted so the model starts at origin (0,0,0).
///
/// # Arguments
/// * `bytes` - VOX file bytes
///
/// # Returns
/// A CubeBox<u8> with tight bounds around the actual voxels
///
/// # Example
/// ```ignore
/// // A 32x32x32 canvas with a 10x20x5 model in the corner
/// let cubebox = load_vox_to_cubebox_compact(bytes)?;
/// println!("Compact size: {:?}", cubebox.size);  // (10, 20, 5) - not (32, 32, 32)
/// ```
pub fn load_vox_to_cubebox_compact(bytes: &[u8]) -> Result<CubeBox<u8>, String> {
    let vox_data =
        dot_vox::load_bytes(bytes).map_err(|e| format!("Failed to parse .vox file: {}", e))?;

    convert_dotvox_to_cubebox(&vox_data, true)
}

/// Load a .vox file from bytes and convert to a Cube with specified alignment
///
/// # Deprecated
/// Use `load_vox_to_cubebox()` instead, which preserves model bounds.
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
#[deprecated(
    since = "0.2.0",
    note = "Use load_vox_to_cubebox() instead, which preserves model bounds"
)]
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

    // Get model size (MagicaVoxel coordinates)
    let vox_size = dot_vox_model.size;
    let model_width = vox_size.x;
    let model_height = vox_size.z; // MagicaVoxel's Z becomes our Y (height)
    let model_depth = vox_size.y; // MagicaVoxel's Y becomes our Z (depth)

    // Calculate required depth from model dimensions
    let max_model_size = model_width.max(model_height).max(model_depth);
    let depth = if max_model_size == 0 {
        0
    } else {
        // Calculate log2 and round up to get minimum depth
        (max_model_size as f32).log2().ceil() as u8
    };
    let cube_size = 1 << depth;

    // Clamp alignment to valid range
    let align = align.clamp(Vec3::ZERO, Vec3::ONE);

    // Calculate offset based on alignment
    let offset_x = ((cube_size - model_width) as f32 * align.x) as i32;
    let offset_y = ((cube_size - model_height) as f32 * align.y) as i32;
    let offset_z = ((cube_size - model_depth) as f32 * align.z) as i32;

    // Build voxel list for batch construction
    let voxels: Vec<Voxel> = dot_vox_model
        .voxels
        .iter()
        .map(|voxel| {
            // MagicaVoxel uses 1-based indexing for colors, we use 0-based
            let palette_index = if voxel.i > 0 { voxel.i - 1 } else { 0 };

            // Get the RGB color from the vox palette
            let color = &vox_data.palette[palette_index as usize];

            // Map vox palette color to material index using bit operations
            let material = map_color_to_material(color.r, color.g, color.b);

            // Transform coordinates:
            // MagicaVoxel (x, y, z) -> Our system (x, z, y) with Y-up
            // Then apply alignment offset
            let x = voxel.x as i32 + offset_x;
            let y = voxel.z as i32 + offset_y; // MagicaVoxel's Z becomes our Y (up)
            let z = voxel.y as i32 + offset_z; // MagicaVoxel's Y becomes our Z (depth)

            Voxel {
                pos: IVec3::new(x, y, z),
                material,
            }
        })
        .collect();

    // Build octree using efficient batch constructor
    Ok(Cube::from_voxels(&voxels, depth as u32, 0))
}

/// Convert DotVoxData to a CubeBox with bounds preserved
///
/// # Arguments
/// * `vox_data` - Parsed VOX data
/// * `compact` - If true, compute tight bounds from actual voxels; if false, use canvas size
fn convert_dotvox_to_cubebox(vox_data: &DotVoxData, compact: bool) -> Result<CubeBox<u8>, String> {
    if vox_data.models.is_empty() {
        return Err("No models found in .vox file".to_string());
    }

    // Use the first model
    let dot_vox_model = &vox_data.models[0];

    // Handle empty voxel case early
    if dot_vox_model.voxels.is_empty() {
        return Ok(CubeBox::new(Cube::Solid(0), IVec3::ZERO, 0));
    }

    // Calculate bounds and offset based on compact mode
    let (size, offset) = if compact {
        // Compute actual min/max bounds from voxels
        let mut min_x = i32::MAX;
        let mut min_y = i32::MAX;
        let mut min_z = i32::MAX;
        let mut max_x = i32::MIN;
        let mut max_y = i32::MIN;
        let mut max_z = i32::MIN;

        for voxel in &dot_vox_model.voxels {
            // Transform coordinates: MagicaVoxel (x, y, z) -> Our system (x, z, y)
            let x = voxel.x as i32;
            let y = voxel.z as i32; // MagicaVoxel's Z becomes our Y (up)
            let z = voxel.y as i32; // MagicaVoxel's Y becomes our Z (depth)

            min_x = min_x.min(x);
            min_y = min_y.min(y);
            min_z = min_z.min(z);
            max_x = max_x.max(x);
            max_y = max_y.max(y);
            max_z = max_z.max(z);
        }

        // Size is max - min + 1 (inclusive bounds)
        let size = IVec3::new(max_x - min_x + 1, max_y - min_y + 1, max_z - min_z + 1);
        // Offset to shift voxels to origin
        let offset = IVec3::new(min_x, min_y, min_z);
        (size, offset)
    } else {
        // Use canvas size from .vox file header
        let vox_size = dot_vox_model.size;
        let model_width = vox_size.x as i32;
        let model_height = vox_size.z as i32; // MagicaVoxel's Z becomes our Y (height)
        let model_depth_z = vox_size.y as i32; // MagicaVoxel's Y becomes our Z (depth)

        let size = IVec3::new(model_width, model_height, model_depth_z);
        let offset = IVec3::ZERO; // No offset needed
        (size, offset)
    };

    // Calculate required octree depth from model dimensions
    let depth = CubeBox::<u8>::min_depth_for_size(size);

    // Build voxel list for batch construction
    // Model is positioned at origin (0,0,0) - apply offset to shift voxels
    let voxels: Vec<Voxel> = dot_vox_model
        .voxels
        .iter()
        .map(|voxel| {
            // MagicaVoxel uses 1-based indexing for colors, we use 0-based
            let palette_index = if voxel.i > 0 { voxel.i - 1 } else { 0 };

            // Get the RGB color from the vox palette
            let color = &vox_data.palette[palette_index as usize];

            // Map vox palette color to material index using bit operations
            let material = map_color_to_material(color.r, color.g, color.b);

            // Transform coordinates:
            // MagicaVoxel (x, y, z) -> Our system (x, z, y) with Y-up
            // Apply offset to shift model to origin (for compact mode)
            let x = voxel.x as i32 - offset.x;
            let y = voxel.z as i32 - offset.y; // MagicaVoxel's Z becomes our Y (up)
            let z = voxel.y as i32 - offset.z; // MagicaVoxel's Y becomes our Z (depth)

            Voxel {
                pos: IVec3::new(x, y, z),
                material,
            }
        })
        .collect();

    // Build octree using efficient batch constructor
    let cube = Cube::from_voxels(&voxels, depth, 0);

    Ok(CubeBox::new(cube, size, depth))
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
