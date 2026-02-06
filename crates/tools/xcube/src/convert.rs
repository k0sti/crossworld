//! Conversion utilities for XCube point clouds to Crossworld voxel formats
//!
//! This module provides functions to convert XCube inference results
//! (point clouds with normals) into Crossworld's voxel formats like CSM.

use crate::types::{XCubeError, XCubeResult};
use cube::{Cube, Voxel};
use glam::IVec3;
use std::collections::HashSet;

/// Color mapping mode for converting normals to material indices
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ColorMode {
    /// 6-color scheme based on dominant axis direction
    /// ±X = red/cyan, ±Y = green/magenta, ±Z = blue/yellow
    SixColor,
    /// Continuous RGB mapping where normal.xyz → rgb, shifted to [0,1]
    ContinuousRGB,
}

/// Configuration for point cloud voxelization
#[derive(Debug, Clone)]
pub struct VoxelizeConfig {
    /// Octree depth (determines grid size: 2^depth)
    pub depth: u32,
    /// Origin offset for coordinate mapping (default: [0.0, 0.0, 0.0])
    pub origin: [f32; 3],
    /// Scale factor for coordinate mapping (default: 1.0)
    pub scale: f32,
}

impl Default for VoxelizeConfig {
    fn default() -> Self {
        Self {
            depth: 5,
            origin: [0.0, 0.0, 0.0],
            scale: 1.0,
        }
    }
}

impl VoxelizeConfig {
    /// Create a new voxelization configuration with specified depth
    pub fn new(depth: u32) -> Self {
        Self {
            depth,
            ..Default::default()
        }
    }

    /// Set the origin offset
    pub fn with_origin(mut self, origin: [f32; 3]) -> Self {
        self.origin = origin;
        self
    }

    /// Set the scale factor
    pub fn with_scale(mut self, scale: f32) -> Self {
        self.scale = scale;
        self
    }
}

/// Convert XCube point cloud to discrete voxel grid coordinates
///
/// Maps XCube normalized coordinates (typically in range [-1, 1]) to discrete
/// voxel grid coordinates [0, 2^depth). Automatically handles duplicate points
/// that map to the same voxel cell.
///
/// # Arguments
///
/// * `points` - Slice of point coordinates as [x, y, z] arrays
/// * `config` - Voxelization configuration (depth, origin offset, scale factor)
///
/// # Returns
///
/// Vector of unique voxel coordinates as IVec3, with duplicates removed
///
/// # Coordinate Mapping
///
/// The mapping formula is:
/// ```text
/// voxel_coord = floor((point + origin) * scale * (2^depth / 2)) + 2^(depth-1)
/// ```
///
/// For default config (origin=[0,0,0], scale=1.0):
/// - Input [-1, -1, -1] maps to voxel [0, 0, 0]
/// - Input [0, 0, 0] maps to voxel [2^(depth-1), 2^(depth-1), 2^(depth-1)]
/// - Input [1, 1, 1] maps to voxel [2^depth - 1, 2^depth - 1, 2^depth - 1]
///
/// # Example
///
/// ```
/// use xcube::convert::{voxelize, VoxelizeConfig};
///
/// let points = vec![
///     [0.0, 0.0, 0.0],    // Center of grid
///     [-1.0, -1.0, -1.0], // Min corner
///     [1.0, 1.0, 1.0],    // Max corner
///     [0.0, 0.0, 0.0],    // Duplicate point
/// ];
///
/// let config = VoxelizeConfig::new(5); // 32x32x32 grid
/// let voxels = voxelize(&points, &config);
///
/// assert_eq!(voxels.len(), 3); // Duplicates removed
/// ```
pub fn voxelize(points: &[[f32; 3]], config: &VoxelizeConfig) -> Vec<IVec3> {
    if points.is_empty() {
        return Vec::new();
    }

    let grid_size = 1 << config.depth; // 2^depth
    let half_size = grid_size as f32 / 2.0;
    let max_coord = grid_size - 1;

    // Use HashSet to automatically handle duplicates
    let mut voxel_set = HashSet::new();

    for [x, y, z] in points {
        // Apply origin offset and scale
        let scaled_x = (x + config.origin[0]) * config.scale;
        let scaled_y = (y + config.origin[1]) * config.scale;
        let scaled_z = (z + config.origin[2]) * config.scale;

        // Map from normalized space [-1, 1] to voxel grid [0, 2^depth)
        let vx = ((scaled_x * half_size) + half_size).floor() as i32;
        let vy = ((scaled_y * half_size) + half_size).floor() as i32;
        let vz = ((scaled_z * half_size) + half_size).floor() as i32;

        // Clamp to valid grid bounds [0, 2^depth)
        let vx = vx.clamp(0, max_coord);
        let vy = vy.clamp(0, max_coord);
        let vz = vz.clamp(0, max_coord);

        voxel_set.insert(IVec3::new(vx, vy, vz));
    }

    // Convert HashSet to Vec for return
    voxel_set.into_iter().collect()
}

/// Convert surface normals to material indices using R2G3B2 color encoding
///
/// Maps XCube surface normals (unit vectors in range [-1, 1]) to material indices
/// in the range [128, 255], using Crossworld's R2G3B2 color encoding system.
///
/// # Arguments
///
/// * `normals` - Slice of normal vectors as [x, y, z] arrays (should be unit vectors)
/// * `mode` - Color mapping mode (SixColor or ContinuousRGB)
///
/// # Returns
///
/// Vector of material indices in range [128, 255], one per input normal
///
/// # Color Modes
///
/// ## SixColor
/// Maps normals to 6 distinct colors based on the dominant axis:
/// - +X (right) → Red (255, 0, 0)
/// - -X (left) → Cyan (0, 255, 255)
/// - +Y (up) → Green (0, 255, 0)
/// - -Y (down) → Magenta (255, 0, 255)
/// - +Z (forward) → Blue (0, 0, 255)
/// - -Z (back) → Yellow (255, 255, 0)
///
/// ## ContinuousRGB
/// Maps normal components directly to RGB:
/// - normal.x → red channel
/// - normal.y → green channel
/// - normal.z → blue channel
///
/// Values are shifted from [-1, 1] to [0, 1] before encoding
///
/// # Edge Cases
///
/// - Zero normals (0, 0, 0) map to material 128 (black)
/// - Unnormalized normals are normalized before processing
/// - NaN/Infinity values are treated as zero
///
/// # R2G3B2 Encoding
///
/// The encoding uses 7 bits (2 bits red, 3 bits green, 2 bits blue):
/// ```text
/// material_index = 128 + ((r_bits << 5) | (g_bits << 2) | b_bits)
/// ```
///
/// # Example
///
/// ```
/// use xcube::convert::{normals_to_materials, ColorMode};
///
/// let normals = vec![
///     [1.0, 0.0, 0.0],   // +X normal
///     [0.0, 1.0, 0.0],   // +Y normal
///     [0.0, 0.0, 1.0],   // +Z normal
/// ];
///
/// let materials = normals_to_materials(&normals, ColorMode::SixColor);
/// // Returns material indices corresponding to red, green, blue
/// ```
pub fn normals_to_materials(normals: &[[f32; 3]], mode: ColorMode) -> Vec<u8> {
    normals
        .iter()
        .map(|&[nx, ny, nz]| {
            // Handle NaN/Infinity by treating as zero
            let nx = if nx.is_finite() { nx } else { 0.0 };
            let ny = if ny.is_finite() { ny } else { 0.0 };
            let nz = if nz.is_finite() { nz } else { 0.0 };

            // Handle zero normals
            let length_sq = nx * nx + ny * ny + nz * nz;
            if length_sq < 1e-6 {
                return 128; // Black color for zero normals
            }

            // Normalize the normal vector
            let length = length_sq.sqrt();
            let nx = nx / length;
            let ny = ny / length;
            let nz = nz / length;

            // Convert to RGB based on mode
            let (r, g, b) = match mode {
                ColorMode::SixColor => normal_to_six_color(nx, ny, nz),
                ColorMode::ContinuousRGB => normal_to_continuous_rgb(nx, ny, nz),
            };

            // Encode RGB to R2G3B2 material index
            encode_r2g3b2(r, g, b)
        })
        .collect()
}

/// Map normal to one of 6 colors based on dominant axis
#[inline]
fn normal_to_six_color(nx: f32, ny: f32, nz: f32) -> (u8, u8, u8) {
    // Find the axis with maximum absolute value
    let abs_x = nx.abs();
    let abs_y = ny.abs();
    let abs_z = nz.abs();

    if abs_x >= abs_y && abs_x >= abs_z {
        // X axis dominant
        if nx > 0.0 {
            (255, 0, 0) // +X → Red
        } else {
            (0, 255, 255) // -X → Cyan
        }
    } else if abs_y >= abs_z {
        // Y axis dominant
        if ny > 0.0 {
            (0, 255, 0) // +Y → Green
        } else {
            (255, 0, 255) // -Y → Magenta
        }
    } else {
        // Z axis dominant
        if nz > 0.0 {
            (0, 0, 255) // +Z → Blue
        } else {
            (255, 255, 0) // -Z → Yellow
        }
    }
}

/// Map normal continuously to RGB by shifting from [-1,1] to [0,1]
#[inline]
fn normal_to_continuous_rgb(nx: f32, ny: f32, nz: f32) -> (u8, u8, u8) {
    // Shift from [-1, 1] to [0, 1] and convert to [0, 255]
    let r = ((nx + 1.0) * 0.5 * 255.0).clamp(0.0, 255.0) as u8;
    let g = ((ny + 1.0) * 0.5 * 255.0).clamp(0.0, 255.0) as u8;
    let b = ((nz + 1.0) * 0.5 * 255.0).clamp(0.0, 255.0) as u8;
    (r, g, b)
}

/// Encode RGB (0-255) to R2G3B2 material index (128-255)
#[inline]
fn encode_r2g3b2(r: u8, g: u8, b: u8) -> u8 {
    // Extract top bits: red (2 bits), green (3 bits), blue (2 bits)
    let r_bits = (r >> 6) & 0b11;
    let g_bits = (g >> 5) & 0b111;
    let b_bits = (b >> 6) & 0b11;

    // Combine into 7-bit index and add 128 offset
    128 + ((r_bits << 5) | (g_bits << 2) | b_bits)
}

/// Convert XCube point cloud to Crossworld Cube octree
///
/// Combines voxelization and color mapping into a complete octree structure.
/// This is the primary function for converting XCube AI-generated point clouds
/// into Crossworld's native voxel format.
///
/// # Arguments
///
/// * `result` - The XCube inference result containing point cloud and normals
/// * `depth` - Octree depth (determines grid size: 2^depth per axis)
/// * `color_mode` - How to map surface normals to material indices
///
/// # Returns
///
/// A `Cube<u8>` octree structure ready for rendering or serialization
///
/// # Resolution Selection
///
/// The function automatically selects between fine and coarse resolution:
/// - If `result.has_fine()` is true, uses fine-resolution data
/// - Otherwise, falls back to coarse-resolution data
///
/// # Coordinate Mapping
///
/// XCube outputs normalized coordinates in range [-1, 1].
/// These are mapped to voxel grid [0, 2^depth) using the voxelization config.
///
/// # Example
///
/// ```no_run
/// # use xcube::{XCubeClient, GenerationRequest};
/// # use xcube::convert::{xcube_to_cube, ColorMode};
/// # #[tokio::main]
/// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let client = XCubeClient::new("http://localhost:8000");
/// let request = GenerationRequest::new("a wooden chair");
/// let result = client.generate(&request).await?;
///
/// // Convert to octree with depth 5 (32x32x32 grid)
/// let cube = xcube_to_cube(&result, 5, ColorMode::SixColor)?;
/// # Ok(())
/// # }
/// ```
pub fn xcube_to_cube(
    result: &XCubeResult,
    depth: u32,
    color_mode: ColorMode,
) -> Result<Cube<u8>, XCubeError> {
    if result.coarse_xyz.is_empty() {
        return Err(XCubeError::ParseError("Result has no points".to_string()));
    }

    // Select resolution: prefer fine if available, otherwise use coarse
    let points = result.fine_xyz.as_ref().unwrap_or(&result.coarse_xyz);
    let normals = result.fine_normal.as_ref().unwrap_or(&result.coarse_normal);

    // Validate matching lengths
    if points.len() != normals.len() {
        return Err(XCubeError::ParseError(format!(
            "Point count ({}) does not match normal count ({})",
            points.len(),
            normals.len()
        )));
    }

    // Step 1: Voxelize point cloud to discrete grid coordinates
    let config = VoxelizeConfig::new(depth);

    // Step 2: Convert normals to material indices
    let materials = normals_to_materials(normals, color_mode);

    // Step 3: Build Voxel structs by combining positions and materials
    // Note: We need to match voxelized positions with their original normals
    // Since voxelize removes duplicates, we need to maintain a mapping
    let mut voxel_map = std::collections::HashMap::new();

    for (idx, [x, y, z]) in points.iter().enumerate() {
        // Apply same voxelization transform
        let grid_size = 1 << config.depth;
        let half_size = grid_size as f32 / 2.0;
        let max_coord = grid_size - 1;

        let scaled_x = (x + config.origin[0]) * config.scale;
        let scaled_y = (y + config.origin[1]) * config.scale;
        let scaled_z = (z + config.origin[2]) * config.scale;

        let vx = ((scaled_x * half_size) + half_size).floor() as i32;
        let vy = ((scaled_y * half_size) + half_size).floor() as i32;
        let vz = ((scaled_z * half_size) + half_size).floor() as i32;

        let vx = vx.clamp(0, max_coord);
        let vy = vy.clamp(0, max_coord);
        let vz = vz.clamp(0, max_coord);

        let pos = IVec3::new(vx, vy, vz);

        // Use the material from this point (if multiple points map to same voxel, last one wins)
        voxel_map.insert(pos, materials[idx]);
    }

    // Convert HashMap to Vec<Voxel>
    let voxels: Vec<Voxel> = voxel_map
        .into_iter()
        .map(|(pos, material)| Voxel { pos, material })
        .collect();

    // Step 4: Build octree from voxels using Cube::from_voxels
    // Use 0 (empty) as default for non-surface voxels
    let cube = Cube::from_voxels(&voxels, depth, 0);

    Ok(cube)
}

/// Convert XCube point cloud result to CSM (CubeScript Model) format
///
/// This function converts an XCube inference result (point cloud with normals)
/// into Crossworld's CSM text format, which can be parsed and rendered by the
/// cube crate.
///
/// # Arguments
///
/// * `result` - The XCube inference result containing point cloud data
///
/// # Returns
///
/// A CSM-formatted string representation of the model
///
/// # Example
///
/// ```no_run
/// # use xcube::{XCubeClient, GenerationRequest};
/// # #[tokio::main]
/// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
/// use xcube::convert::xcube_to_csm;
///
/// let client = XCubeClient::new("http://localhost:8000");
/// let request = GenerationRequest::new("a wooden chair");
/// let result = client.generate(&request).await?;
///
/// let csm_string = xcube_to_csm(&result)?;
/// # Ok(())
/// # }
/// ```
pub fn xcube_to_csm(result: &XCubeResult) -> Result<String, XCubeError> {
    if result.coarse_xyz.is_empty() {
        return Err(XCubeError::ParseError("Result has no points".to_string()));
    }

    // Convert to Cube octree with depth 6 (64x64x64 grid) and six-color mode
    let cube = xcube_to_cube(result, 6, ColorMode::SixColor)?;

    // Serialize to CSM format using cube's serializer
    let csm_string = cube::serialize_csm(&cube);

    Ok(csm_string)
}

/// Convert XCube point cloud to a discretized voxel grid
///
/// This creates a 3D array representation by discretizing the point cloud
/// positions into a voxel grid. The grid resolution is determined by the
/// point cloud density.
///
/// # Arguments
///
/// * `result` - The XCube inference result
/// * `resolution` - Grid resolution (voxels per unit)
/// * `color_mode` - Optional color mode for normal-to-material mapping
///   (if None, uses default material index 1)
///
/// # Returns
///
/// A 3D grid where `Some(material_index)` indicates a voxel is present with that material
pub fn xcube_to_grid(
    result: &XCubeResult,
    resolution: f32,
    color_mode: Option<ColorMode>,
) -> Result<Vec<Vec<Vec<Option<u8>>>>, XCubeError> {
    if result.coarse_xyz.is_empty() {
        return Err(XCubeError::ParseError("Result has no points".to_string()));
    }

    // Use fine points if available, otherwise use coarse
    let points = result.fine_xyz.as_ref().unwrap_or(&result.coarse_xyz);
    let normals = result.fine_normal.as_ref().unwrap_or(&result.coarse_normal);

    // Validate that points and normals have matching lengths
    if points.len() != normals.len() {
        return Err(XCubeError::ParseError(format!(
            "Point count ({}) does not match normal count ({})",
            points.len(),
            normals.len()
        )));
    }

    // Find bounding box
    let mut min = [f32::MAX, f32::MAX, f32::MAX];
    let mut max = [f32::MIN, f32::MIN, f32::MIN];

    for [x, y, z] in points {
        min[0] = min[0].min(*x);
        min[1] = min[1].min(*y);
        min[2] = min[2].min(*z);
        max[0] = max[0].max(*x);
        max[1] = max[1].max(*y);
        max[2] = max[2].max(*z);
    }

    // Calculate grid dimensions
    let width = ((max[0] - min[0]) * resolution).ceil() as usize + 1;
    let height = ((max[1] - min[1]) * resolution).ceil() as usize + 1;
    let depth = ((max[2] - min[2]) * resolution).ceil() as usize + 1;

    // Initialize empty grid
    let mut grid = vec![vec![vec![None; depth]; height]; width];

    // Convert normals to materials if color mode is specified
    let materials = if let Some(mode) = color_mode {
        normals_to_materials(normals, mode)
    } else {
        vec![1; normals.len()] // Default material index
    };

    // Fill grid with discretized points
    for (idx, [x, y, z]) in points.iter().enumerate() {
        let ix = ((x - min[0]) * resolution) as usize;
        let iy = ((y - min[1]) * resolution) as usize;
        let iz = ((z - min[2]) * resolution) as usize;

        if ix < width && iy < height && iz < depth {
            grid[ix][iy][iz] = Some(materials[idx]);
        }
    }

    Ok(grid)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_result() -> XCubeResult {
        XCubeResult {
            coarse_xyz: vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]],
            coarse_normal: vec![[0.0, 0.0, 1.0], [0.0, 0.0, 1.0], [0.0, 0.0, 1.0]],
            fine_xyz: None,
            fine_normal: None,
        }
    }

    #[test]
    fn test_xcube_to_grid() {
        let result = create_test_result();
        let grid = xcube_to_grid(&result, 1.0, None).unwrap();

        assert!(!grid.is_empty());
        assert!(!grid[0].is_empty());
        assert!(!grid[0][0].is_empty());

        // Check that at least some voxels are present
        let mut has_voxels = false;
        for x in &grid {
            for y in x {
                for z in y {
                    if z.is_some() {
                        has_voxels = true;
                    }
                }
            }
        }
        assert!(has_voxels);
    }

    #[test]
    fn test_xcube_to_csm_placeholder() {
        let result = create_test_result();
        let csm = xcube_to_csm(&result).unwrap();

        // CSM/BCF format should be non-empty and start with valid character
        // '>' for BCF format, 's' or digit for CSM format
        assert!(!csm.is_empty());
        let first_char = csm.chars().next().unwrap();
        assert!(
            first_char == '>' || first_char == 's' || first_char.is_ascii_digit(),
            "CSM should start with '>', 's', or digit, got: '{}'",
            first_char
        );
    }

    #[test]
    fn test_empty_result_error() {
        let empty_result = XCubeResult {
            coarse_xyz: vec![],
            coarse_normal: vec![],
            fine_xyz: None,
            fine_normal: None,
        };

        assert!(xcube_to_csm(&empty_result).is_err());
        assert!(xcube_to_grid(&empty_result, 1.0, None).is_err());
    }

    // Voxelization tests
    use super::{voxelize, VoxelizeConfig};

    #[test]
    fn test_voxelize_basic() {
        let points = vec![
            [0.0, 0.0, 0.0],    // Center
            [-1.0, -1.0, -1.0], // Min corner
            [1.0, 1.0, 1.0],    // Max corner
        ];

        let config = VoxelizeConfig::new(5); // 32x32x32 grid
        let voxels = voxelize(&points, &config);

        assert_eq!(voxels.len(), 3);

        // Center point should map to middle of grid
        assert!(voxels.iter().any(|v| v.x == 16 && v.y == 16 && v.z == 16));

        // Min corner should map to (0, 0, 0)
        assert!(voxels.iter().any(|v| v.x == 0 && v.y == 0 && v.z == 0));

        // Max corner should map to (31, 31, 31)
        assert!(voxels.iter().any(|v| v.x == 31 && v.y == 31 && v.z == 31));
    }

    #[test]
    fn test_voxelize_duplicates() {
        // Multiple points that map to the same voxel cell
        let points = vec![
            [0.0, 0.0, 0.0],
            [0.01, 0.01, 0.01], // Very close to origin, should map to same voxel
            [0.0, 0.0, 0.0],    // Exact duplicate
        ];

        let config = VoxelizeConfig::new(5);
        let voxels = voxelize(&points, &config);

        // All three points should map to the same voxel
        assert_eq!(voxels.len(), 1);
    }

    #[test]
    fn test_voxelize_empty_points() {
        let points: Vec<[f32; 3]> = vec![];
        let config = VoxelizeConfig::new(5);
        let voxels = voxelize(&points, &config);

        assert_eq!(voxels.len(), 0);
    }

    #[test]
    fn test_voxelize_with_origin_offset() {
        let points = vec![[0.0, 0.0, 0.0]];

        // With origin offset of [1, 1, 1], point at [0, 0, 0] becomes [1, 1, 1]
        let config = VoxelizeConfig::new(5).with_origin([1.0, 1.0, 1.0]);
        let voxels = voxelize(&points, &config);

        assert_eq!(voxels.len(), 1);
        // [1, 1, 1] in normalized space is at max corner
        assert!(voxels[0].x > 16); // Should be in upper half
        assert!(voxels[0].y > 16);
        assert!(voxels[0].z > 16);
    }

    #[test]
    fn test_voxelize_with_scale() {
        let points = vec![[0.5, 0.5, 0.5]];

        // With scale of 2.0, point [0.5, 0.5, 0.5] becomes [1.0, 1.0, 1.0]
        let config = VoxelizeConfig::new(5).with_scale(2.0);
        let voxels = voxelize(&points, &config);

        assert_eq!(voxels.len(), 1);
        // Should map to max corner (31, 31, 31)
        assert_eq!(voxels[0].x, 31);
        assert_eq!(voxels[0].y, 31);
        assert_eq!(voxels[0].z, 31);
    }

    #[test]
    fn test_voxelize_different_depths() {
        let points = vec![[0.0, 0.0, 0.0]];

        // Test with depth 3 (8x8x8 grid)
        let config3 = VoxelizeConfig::new(3);
        let voxels3 = voxelize(&points, &config3);
        assert_eq!(voxels3[0].x, 4); // Middle of 8x8x8

        // Test with depth 6 (64x64x64 grid)
        let config6 = VoxelizeConfig::new(6);
        let voxels6 = voxelize(&points, &config6);
        assert_eq!(voxels6[0].x, 32); // Middle of 64x64x64

        // Test with depth 7 (128x128x128 grid)
        let config7 = VoxelizeConfig::new(7);
        let voxels7 = voxelize(&points, &config7);
        assert_eq!(voxels7[0].x, 64); // Middle of 128x128x128
    }

    #[test]
    fn test_voxelize_bounds_clamping() {
        // Points outside [-1, 1] range should be clamped
        let points = vec![
            [-2.0, -2.0, -2.0], // Way outside min
            [2.0, 2.0, 2.0],    // Way outside max
        ];

        let config = VoxelizeConfig::new(5);
        let voxels = voxelize(&points, &config);

        assert_eq!(voxels.len(), 2);

        // Both should be clamped to valid grid bounds
        for voxel in &voxels {
            assert!(voxel.x >= 0 && voxel.x < 32);
            assert!(voxel.y >= 0 && voxel.y < 32);
            assert!(voxel.z >= 0 && voxel.z < 32);
        }
    }

    #[test]
    fn test_voxelize_synthetic_sphere() {
        // Generate points in a synthetic sphere pattern
        let mut points = Vec::new();
        for i in 0..10 {
            let angle = (i as f32) * std::f32::consts::PI * 2.0 / 10.0;
            let x = angle.cos() * 0.5;
            let y = angle.sin() * 0.5;
            let z = 0.0;
            points.push([x, y, z]);
        }

        let config = VoxelizeConfig::new(5);
        let voxels = voxelize(&points, &config);

        // Should have multiple unique voxels (some may overlap due to discretization)
        assert!(voxels.len() > 0);
        assert!(voxels.len() <= 10);

        // All voxels should be within valid bounds
        for voxel in &voxels {
            assert!(voxel.x >= 0 && voxel.x < 32);
            assert!(voxel.y >= 0 && voxel.y < 32);
            assert!(voxel.z >= 0 && voxel.z < 32);
        }
    }

    #[test]
    fn test_voxelize_line_pattern() {
        // Generate points along a line from [-1, 0, 0] to [1, 0, 0]
        let mut points = Vec::new();
        for i in 0..=10 {
            let x = -1.0 + (i as f32) * 0.2;
            points.push([x, 0.0, 0.0]);
        }

        let config = VoxelizeConfig::new(5);
        let voxels = voxelize(&points, &config);

        // Should have multiple voxels along x-axis
        assert!(voxels.len() > 0);

        // All should have y and z around center (16)
        for voxel in &voxels {
            assert_eq!(voxel.y, 16);
            assert_eq!(voxel.z, 16);
        }
    }

    #[test]
    fn test_voxelize_config_builder() {
        let config = VoxelizeConfig::new(6)
            .with_origin([0.5, 0.5, 0.5])
            .with_scale(1.5);

        assert_eq!(config.depth, 6);
        assert_eq!(config.origin, [0.5, 0.5, 0.5]);
        assert_eq!(config.scale, 1.5);
    }

    // Normal-to-material conversion tests
    use super::{normals_to_materials, ColorMode};

    #[test]
    fn test_normals_to_materials_six_color_mode() {
        let normals = vec![
            [1.0, 0.0, 0.0],  // +X → Red
            [-1.0, 0.0, 0.0], // -X → Cyan
            [0.0, 1.0, 0.0],  // +Y → Green
            [0.0, -1.0, 0.0], // -Y → Magenta
            [0.0, 0.0, 1.0],  // +Z → Blue
            [0.0, 0.0, -1.0], // -Z → Yellow
        ];

        let materials = normals_to_materials(&normals, ColorMode::SixColor);

        assert_eq!(materials.len(), 6);

        // All materials should be in valid range [128, 255]
        for &mat in &materials {
            assert!(mat >= 128);
        }

        // All 6 colors should be distinct
        let unique_count = materials
            .iter()
            .collect::<std::collections::HashSet<_>>()
            .len();
        assert_eq!(unique_count, 6);
    }

    #[test]
    fn test_normals_to_materials_continuous_rgb() {
        let normals = vec![
            [1.0, 0.0, 0.0],  // Max X → High red
            [0.0, 1.0, 0.0],  // Max Y → High green
            [0.0, 0.0, 1.0],  // Max Z → High blue
            [-1.0, 0.0, 0.0], // Min X → Low red
            [0.0, -1.0, 0.0], // Min Y → Low green
            [0.0, 0.0, -1.0], // Min Z → Low blue
        ];

        let materials = normals_to_materials(&normals, ColorMode::ContinuousRGB);

        assert_eq!(materials.len(), 6);

        // All materials should be in valid range [128, 255]
        for &mat in &materials {
            assert!(mat >= 128);
        }

        // Opposite normals should produce different materials
        assert_ne!(materials[0], materials[3]); // +X vs -X
        assert_ne!(materials[1], materials[4]); // +Y vs -Y
        assert_ne!(materials[2], materials[5]); // +Z vs -Z
    }

    #[test]
    fn test_normals_to_materials_zero_normal() {
        let normals = vec![[0.0, 0.0, 0.0]];
        let materials = normals_to_materials(&normals, ColorMode::SixColor);

        assert_eq!(materials.len(), 1);
        assert_eq!(materials[0], 128); // Zero normal → black (material 128)
    }

    #[test]
    fn test_normals_to_materials_unnormalized() {
        // Unnormalized normals should be normalized before processing
        let normals = vec![
            [2.0, 0.0, 0.0], // Should normalize to [1, 0, 0]
            [0.0, 5.0, 0.0], // Should normalize to [0, 1, 0]
            [0.0, 0.0, 0.5], // Should normalize to [0, 0, 1]
        ];

        let materials = normals_to_materials(&normals, ColorMode::SixColor);

        // Should produce the same results as normalized versions
        let normalized_normals = vec![[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]];
        let normalized_materials = normals_to_materials(&normalized_normals, ColorMode::SixColor);

        assert_eq!(materials, normalized_materials);
    }

    #[test]
    fn test_normals_to_materials_nan_infinity() {
        let normals = vec![
            [f32::NAN, 0.0, 0.0],
            [f32::INFINITY, 0.0, 0.0],
            [0.0, f32::NEG_INFINITY, 0.0],
            [f32::NAN, f32::NAN, f32::NAN],
        ];

        let materials = normals_to_materials(&normals, ColorMode::SixColor);

        assert_eq!(materials.len(), 4);

        // All NaN/Infinity values should be treated as zero and map to material 128
        for &mat in &materials {
            assert_eq!(mat, 128);
        }
    }

    #[test]
    fn test_normals_to_materials_diagonal_normals() {
        // Test normals that don't align with axes
        let normals = vec![
            [0.707, 0.707, 0.0],   // 45° between X and Y
            [0.577, 0.577, 0.577], // Equal components
            [-0.5, 0.5, -0.707],   // Mixed signs
        ];

        let materials = normals_to_materials(&normals, ColorMode::SixColor);

        assert_eq!(materials.len(), 3);

        // All should produce valid material indices
        for &mat in &materials {
            assert!(mat >= 128);
        }
    }

    #[test]
    fn test_normals_to_materials_continuous_rgb_range() {
        // Test that continuous RGB mode maps the full [-1, 1] range
        let normals = vec![
            [1.0, 1.0, 1.0],    // Max → should have high RGB
            [-1.0, -1.0, -1.0], // Min → should have low RGB
            [0.0, 0.0, 0.0],    // Zero → should map to middle (but treated as zero normal)
        ];

        let materials = normals_to_materials(&normals, ColorMode::ContinuousRGB);

        assert_eq!(materials.len(), 3);
        assert!(materials[0] >= 128);
        assert!(materials[1] >= 128);
        assert_eq!(materials[2], 128); // Zero normal
    }

    #[test]
    fn test_normals_to_materials_empty() {
        let normals: Vec<[f32; 3]> = vec![];
        let materials = normals_to_materials(&normals, ColorMode::SixColor);

        assert_eq!(materials.len(), 0);
    }

    #[test]
    fn test_normals_to_materials_large_batch() {
        // Test with many normals to ensure consistent behavior
        let mut normals = Vec::new();
        for i in 0..1000 {
            let angle = (i as f32) * std::f32::consts::PI * 2.0 / 1000.0;
            normals.push([angle.cos(), angle.sin(), 0.0]);
        }

        let materials = normals_to_materials(&normals, ColorMode::ContinuousRGB);

        assert_eq!(materials.len(), 1000);

        // All should be valid material indices
        for &mat in &materials {
            assert!(mat >= 128);
        }
    }

    #[test]
    fn test_xcube_to_grid_with_normals() {
        let result = create_test_result();

        // Test with SixColor mode
        let grid_six = xcube_to_grid(&result, 1.0, Some(ColorMode::SixColor)).unwrap();
        assert!(!grid_six.is_empty());

        // Test with ContinuousRGB mode
        let grid_cont = xcube_to_grid(&result, 1.0, Some(ColorMode::ContinuousRGB)).unwrap();
        assert!(!grid_cont.is_empty());

        // Test with no color mode (should use default material 1)
        let grid_default = xcube_to_grid(&result, 1.0, None).unwrap();
        assert!(!grid_default.is_empty());

        // Find a voxel in the default grid and verify it has material 1
        let mut found = false;
        'outer: for x in &grid_default {
            for y in x {
                for z in y {
                    if let Some(mat) = z {
                        assert_eq!(*mat, 1);
                        found = true;
                        break 'outer;
                    }
                }
            }
        }
        assert!(found);
    }

    #[test]
    fn test_xcube_to_grid_mismatched_lengths() {
        let bad_result = XCubeResult {
            coarse_xyz: vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0]],
            coarse_normal: vec![[0.0, 0.0, 1.0]], // Mismatched length
            fine_xyz: None,
            fine_normal: None,
        };

        let result = xcube_to_grid(&bad_result, 1.0, Some(ColorMode::SixColor));
        assert!(result.is_err());

        if let Err(e) = result {
            assert!(e.to_string().contains("does not match"));
        }
    }

    #[test]
    fn test_encode_r2g3b2_consistency() {
        use super::encode_r2g3b2;

        // Test known color encodings
        assert_eq!(encode_r2g3b2(255, 0, 0), 128 + (0b11 << 5)); // Red
        assert_eq!(encode_r2g3b2(0, 255, 0), 128 + (0b111 << 2)); // Green
        assert_eq!(encode_r2g3b2(0, 0, 255), 128 + 0b11); // Blue
        assert_eq!(encode_r2g3b2(0, 0, 0), 128); // Black
        assert_eq!(encode_r2g3b2(255, 255, 255), 255); // White

        // Test that all encoded values are in [128, 255]
        for r in [0, 64, 128, 192, 255] {
            for g in [0, 36, 73, 109, 146, 182, 219, 255] {
                for b in [0, 85, 170, 255] {
                    let mat = encode_r2g3b2(r, g, b);
                    assert!(mat >= 128);
                }
            }
        }
    }

    // xcube_to_cube integration tests
    use super::xcube_to_cube;

    fn create_test_cube_result() -> XCubeResult {
        // Create a simple cube-like point cloud
        let mut points = Vec::new();
        let mut normals = Vec::new();

        // Create points on the surface of a small cube
        // Front face (z = 0.5)
        for x in [-0.5, -0.25, 0.0, 0.25, 0.5] {
            for y in [-0.5, -0.25, 0.0, 0.25, 0.5] {
                points.push([x, y, 0.5]);
                normals.push([0.0, 0.0, 1.0]); // +Z normal
            }
        }

        // Back face (z = -0.5)
        for x in [-0.5, -0.25, 0.0, 0.25, 0.5] {
            for y in [-0.5, -0.25, 0.0, 0.25, 0.5] {
                points.push([x, y, -0.5]);
                normals.push([0.0, 0.0, -1.0]); // -Z normal
            }
        }

        // Top face (y = 0.5)
        for x in [-0.5, -0.25, 0.0, 0.25, 0.5] {
            for z in [-0.5, -0.25, 0.0, 0.25, 0.5] {
                points.push([x, 0.5, z]);
                normals.push([0.0, 1.0, 0.0]); // +Y normal
            }
        }

        // Bottom face (y = -0.5)
        for x in [-0.5, -0.25, 0.0, 0.25, 0.5] {
            for z in [-0.5, -0.25, 0.0, 0.25, 0.5] {
                points.push([x, -0.5, z]);
                normals.push([0.0, -1.0, 0.0]); // -Y normal
            }
        }

        XCubeResult {
            coarse_xyz: points,
            coarse_normal: normals,
            fine_xyz: None,
            fine_normal: None,
        }
    }

    #[test]
    fn test_xcube_to_cube_basic() {
        let result = create_test_cube_result();
        let cube = xcube_to_cube(&result, 5, ColorMode::SixColor).unwrap();

        // Should create a valid octree structure
        // The cube should not be a simple Solid (should have some structure)
        match cube {
            cube::Cube::Solid(_) => {
                // If it's solid, it should be empty (0)
                if let cube::Cube::Solid(val) = cube {
                    // For a sparse surface, we might get solid 0
                    assert_eq!(val, 0);
                }
            }
            cube::Cube::Cubes(_) => {
                // This is expected - an octree structure was created
            }
            _ => {
                // Other subdivision types are also valid
            }
        }
    }

    #[test]
    fn test_xcube_to_cube_different_depths() {
        let result = create_test_cube_result();

        // Test different depth values
        for depth in [3, 4, 5, 6, 7] {
            let cube = xcube_to_cube(&result, depth, ColorMode::SixColor);
            assert!(cube.is_ok(), "Failed to convert at depth {}", depth);
        }
    }

    #[test]
    fn test_xcube_to_cube_color_modes() {
        let result = create_test_cube_result();

        // Test both color modes
        let cube_six = xcube_to_cube(&result, 5, ColorMode::SixColor).unwrap();
        let cube_rgb = xcube_to_cube(&result, 5, ColorMode::ContinuousRGB).unwrap();

        // Both should produce valid cubes
        // They may differ in internal structure due to different materials
        assert!(matches!(
            cube_six,
            cube::Cube::Solid(_) | cube::Cube::Cubes(_)
        ));
        assert!(matches!(
            cube_rgb,
            cube::Cube::Solid(_) | cube::Cube::Cubes(_)
        ));
    }

    #[test]
    fn test_xcube_to_cube_empty_result() {
        let empty_result = XCubeResult {
            coarse_xyz: vec![],
            coarse_normal: vec![],
            fine_xyz: None,
            fine_normal: None,
        };

        let result = xcube_to_cube(&empty_result, 5, ColorMode::SixColor);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Result has no points"));
    }

    #[test]
    fn test_xcube_to_cube_mismatched_lengths() {
        let bad_result = XCubeResult {
            coarse_xyz: vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0]],
            coarse_normal: vec![[0.0, 0.0, 1.0]], // Mismatched length
            fine_xyz: None,
            fine_normal: None,
        };

        let result = xcube_to_cube(&bad_result, 5, ColorMode::SixColor);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("does not match normal count"));
    }

    #[test]
    fn test_xcube_to_cube_fine_resolution() {
        // Create a result with both coarse and fine data
        let coarse_points = vec![[0.0, 0.0, 0.0]];
        let coarse_normals = vec![[0.0, 0.0, 1.0]];

        let fine_points = vec![
            [0.0, 0.0, 0.0],
            [0.1, 0.0, 0.0],
            [0.0, 0.1, 0.0],
            [0.0, 0.0, 0.1],
        ];
        let fine_normals = vec![
            [0.0, 0.0, 1.0],
            [1.0, 0.0, 0.0],
            [0.0, 1.0, 0.0],
            [0.0, 0.0, 1.0],
        ];

        let result_with_fine = XCubeResult {
            coarse_xyz: coarse_points,
            coarse_normal: coarse_normals,
            fine_xyz: Some(fine_points),
            fine_normal: Some(fine_normals),
        };

        let cube = xcube_to_cube(&result_with_fine, 5, ColorMode::SixColor).unwrap();

        // Should use fine resolution (4 points) instead of coarse (1 point)
        assert!(matches!(cube, cube::Cube::Solid(_) | cube::Cube::Cubes(_)));
    }

    #[test]
    fn test_xcube_to_cube_single_point() {
        let result = XCubeResult {
            coarse_xyz: vec![[0.0, 0.0, 0.0]],
            coarse_normal: vec![[0.0, 0.0, 1.0]],
            fine_xyz: None,
            fine_normal: None,
        };

        let cube = xcube_to_cube(&result, 5, ColorMode::SixColor).unwrap();

        // Single point should create a simple structure
        assert!(matches!(cube, cube::Cube::Solid(_) | cube::Cube::Cubes(_)));
    }

    #[test]
    fn test_xcube_to_cube_duplicate_points() {
        // Multiple points that map to the same voxel
        let result = XCubeResult {
            coarse_xyz: vec![
                [0.0, 0.0, 0.0],
                [0.01, 0.01, 0.01], // Very close, should map to same voxel
                [0.0, 0.0, 0.0],    // Exact duplicate
            ],
            coarse_normal: vec![[0.0, 0.0, 1.0], [0.0, 0.0, 1.0], [0.0, 0.0, 1.0]],
            fine_xyz: None,
            fine_normal: None,
        };

        let cube = xcube_to_cube(&result, 5, ColorMode::SixColor).unwrap();

        // Should handle duplicates gracefully
        assert!(matches!(cube, cube::Cube::Solid(_) | cube::Cube::Cubes(_)));
    }

    #[test]
    fn test_xcube_to_cube_sparse_surface() {
        // Create a very sparse point cloud (few points scattered)
        let result = XCubeResult {
            coarse_xyz: vec![
                [-0.8, -0.8, -0.8],
                [0.8, 0.8, 0.8],
                [-0.8, 0.8, -0.8],
                [0.8, -0.8, 0.8],
            ],
            coarse_normal: vec![
                [-1.0, -1.0, -1.0],
                [1.0, 1.0, 1.0],
                [-1.0, 1.0, -1.0],
                [1.0, -1.0, 1.0],
            ],
            fine_xyz: None,
            fine_normal: None,
        };

        let cube = xcube_to_cube(&result, 5, ColorMode::SixColor).unwrap();

        // Sparse surface should still produce a valid octree
        assert!(matches!(cube, cube::Cube::Solid(_) | cube::Cube::Cubes(_)));
    }

    #[test]
    fn test_xcube_to_csm_integration() {
        let result = create_test_cube_result();

        // Test that xcube_to_csm works (it internally calls xcube_to_cube)
        let csm = super::xcube_to_csm(&result).unwrap();

        // Should produce a non-empty CSM/BCF string
        assert!(!csm.is_empty());

        // CSM/BCF format should start with valid character
        // '>' for BCF format, 's' or digit for CSM format, '0' for empty
        let first_char = csm.chars().next().unwrap();
        assert!(
            first_char == '>' || first_char == 's' || first_char.is_ascii_digit(),
            "CSM should start with '>', 's', or digit, got: '{}'",
            first_char
        );
    }
}
