//! Conversion utilities for XCube point clouds to Crossworld voxel formats
//!
//! This module provides functions to convert XCube inference results
//! (point clouds with normals) into Crossworld's voxel formats like CSM.

use crate::types::{XCubeError, XCubeResult};
use glam::IVec3;
use std::collections::HashSet;

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

    // TODO: Implement actual conversion logic
    // This is a placeholder that will be implemented in the next phase
    // The conversion will need to:
    // 1. Convert point cloud to voxel grid (discretize positions)
    // 2. Build an octree structure from the voxel grid
    // 3. Serialize it to CSM format (nested s[] and o[] syntax)
    // 4. Use normals to determine surface voxels

    let coarse_count = result.coarse_point_count();
    let fine_count = result.fine_point_count();

    let placeholder = format!(
        "s[/* XCube result: {} coarse points, {} fine points - conversion not yet implemented */]",
        coarse_count, fine_count
    );

    Ok(placeholder)
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
///
/// # Returns
///
/// A 3D grid where `Some(1)` indicates a voxel is present
pub fn xcube_to_grid(
    result: &XCubeResult,
    resolution: f32,
) -> Result<Vec<Vec<Vec<Option<u8>>>>, XCubeError> {
    if result.coarse_xyz.is_empty() {
        return Err(XCubeError::ParseError("Result has no points".to_string()));
    }

    // Use fine points if available, otherwise use coarse
    let points = result.fine_xyz.as_ref().unwrap_or(&result.coarse_xyz);

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

    // Fill grid with discretized points
    for [x, y, z] in points {
        let ix = ((x - min[0]) * resolution) as usize;
        let iy = ((y - min[1]) * resolution) as usize;
        let iz = ((z - min[2]) * resolution) as usize;

        if ix < width && iy < height && iz < depth {
            grid[ix][iy][iz] = Some(1); // Default color index
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
        let grid = xcube_to_grid(&result, 1.0).unwrap();

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

        assert!(csm.contains("3 coarse points"));
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
        assert!(xcube_to_grid(&empty_result, 1.0).is_err());
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
}
