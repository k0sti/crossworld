//! Conversion utilities for XCube point clouds to Crossworld voxel formats
//!
//! This module provides functions to convert XCube inference results
//! (point clouds with normals) into Crossworld's voxel formats like CSM.

use crate::types::{XCubeError, XCubeResult};

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
}
