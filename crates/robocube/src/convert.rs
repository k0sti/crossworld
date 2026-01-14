//! Occupancy field to Crossworld voxel format conversion
//!
//! Converts Cube3D occupancy field outputs directly to Crossworld's CSM format.
//! This approach queries the occupancy decoder at discrete grid points instead of
//! converting meshes to voxels, resulting in more accurate voxel representations.

use crate::types::{OccupancyResult, Result, RobocubeError};
use cube::{serialize_csm, Cube, CubeBox, Voxel};
use glam::IVec3;

/// Default material index for occupied voxels (white in R2G3B2)
pub const DEFAULT_MATERIAL: u8 = 255;

/// Convert occupancy result to Crossworld Cube octree
///
/// Takes the occupied voxel positions from the Cube3D occupancy field
/// and builds a Cube octree suitable for rendering or serialization.
///
/// # Arguments
///
/// * `result` - The occupancy field result from Cube3D server
/// * `material` - Material index to assign to occupied voxels (default: 255 = white)
///
/// # Returns
///
/// A `CubeBox<u8>` containing the octree with size and depth metadata
///
/// # Example
///
/// ```no_run
/// use robocube::{RobocubeClient, OccupancyRequest};
/// use robocube::convert::occupancy_to_cube;
///
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     let client = RobocubeClient::new("http://localhost:8642");
///     let request = OccupancyRequest::new("A wooden chair")
///         .with_grid_resolution(64);
///
///     let result = client.generate_occupancy(&request).await?;
///     let cubebox = occupancy_to_cube(&result, None)?;
///
///     println!("Generated octree with depth {}", cubebox.depth);
///     Ok(())
/// }
/// ```
pub fn occupancy_to_cube(result: &OccupancyResult, material: Option<u8>) -> Result<CubeBox<u8>> {
    // Validate input
    result.validate()?;

    if result.occupied_voxels.is_empty() {
        return Err(RobocubeError::ConversionError(
            "No occupied voxels in result".to_string(),
        ));
    }

    let mat = material.unwrap_or(DEFAULT_MATERIAL);

    // Calculate depth from resolution (resolution = 2^depth)
    let depth = (result.resolution as f32).log2().ceil() as u32;

    // Convert occupied voxels to cube::Voxel format
    let voxels: Vec<Voxel> = result
        .occupied_voxels
        .iter()
        .map(|[x, y, z]| Voxel {
            pos: IVec3::new(*x as i32, *y as i32, *z as i32),
            material: mat,
        })
        .collect();

    // Build octree from voxels
    let cube = Cube::from_voxels(&voxels, depth, 0);

    // Create CubeBox with size metadata
    let size = IVec3::splat(result.resolution as i32);
    let cubebox = CubeBox { cube, size, depth };

    Ok(cubebox)
}

/// Convert occupancy result to CSM (CubeScript Model) format
///
/// This is a convenience function that converts the occupancy field
/// directly to Crossworld's human-readable CSM text format.
///
/// # Arguments
///
/// * `result` - The occupancy field result from Cube3D server
/// * `material` - Material index to assign to occupied voxels (default: 255 = white)
///
/// # Returns
///
/// A CSM-formatted string representation of the model
///
/// # Example
///
/// ```no_run
/// use robocube::{RobocubeClient, OccupancyRequest};
/// use robocube::convert::occupancy_to_csm;
///
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     let client = RobocubeClient::new("http://localhost:8642");
///     let request = OccupancyRequest::new("A wooden chair");
///
///     let result = client.generate_occupancy(&request).await?;
///     let csm = occupancy_to_csm(&result, None)?;
///
///     std::fs::write("chair.csm", csm)?;
///     Ok(())
/// }
/// ```
pub fn occupancy_to_csm(result: &OccupancyResult, material: Option<u8>) -> Result<String> {
    let cubebox = occupancy_to_cube(result, material)?;
    let csm = serialize_csm(&cubebox.cube);
    Ok(csm)
}

/// Convert raw occupancy logits to voxels with custom threshold
///
/// If the occupancy result includes raw logits, this function allows
/// re-thresholding them to generate different voxel sets without
/// re-querying the server.
///
/// # Arguments
///
/// * `result` - The occupancy field result (must have logits)
/// * `threshold` - Occupancy threshold (values > threshold are occupied)
///
/// # Returns
///
/// Vector of occupied voxel positions [x, y, z]
pub fn threshold_logits(result: &OccupancyResult, threshold: f32) -> Result<Vec<[u32; 3]>> {
    let logits = result.logits.as_ref().ok_or_else(|| {
        RobocubeError::ConversionError("Result does not contain raw logits".to_string())
    })?;

    let res = result.resolution;
    let mut occupied = Vec::new();

    for (i, &logit) in logits.iter().enumerate() {
        if logit > threshold {
            // Convert flat index to 3D coordinates (row-major: x fastest, then y, then z)
            let x = (i % res as usize) as u32;
            let y = ((i / res as usize) % res as usize) as u32;
            let z = (i / (res as usize * res as usize)) as u32;
            occupied.push([x, y, z]);
        }
    }

    Ok(occupied)
}

/// Convert occupancy result to Cube with custom threshold from logits
///
/// Re-thresholds raw logits and builds a new octree.
///
/// # Arguments
///
/// * `result` - The occupancy field result (must have logits)
/// * `threshold` - Occupancy threshold
/// * `material` - Material index for occupied voxels
pub fn occupancy_to_cube_with_threshold(
    result: &OccupancyResult,
    threshold: f32,
    material: Option<u8>,
) -> Result<CubeBox<u8>> {
    let occupied = threshold_logits(result, threshold)?;

    if occupied.is_empty() {
        return Err(RobocubeError::ConversionError(
            "No voxels above threshold".to_string(),
        ));
    }

    let mat = material.unwrap_or(DEFAULT_MATERIAL);
    let depth = (result.resolution as f32).log2().ceil() as u32;

    let voxels: Vec<Voxel> = occupied
        .iter()
        .map(|[x, y, z]| Voxel {
            pos: IVec3::new(*x as i32, *y as i32, *z as i32),
            material: mat,
        })
        .collect();

    let cube = Cube::from_voxels(&voxels, depth, 0);
    let size = IVec3::splat(result.resolution as i32);

    Ok(CubeBox { cube, size, depth })
}

/// Compute occupancy statistics from logits
///
/// Returns min, max, mean, and percentiles of the occupancy logits.
pub fn compute_logit_statistics(result: &OccupancyResult) -> Result<LogitStatistics> {
    let logits = result.logits.as_ref().ok_or_else(|| {
        RobocubeError::ConversionError("Result does not contain raw logits".to_string())
    })?;

    if logits.is_empty() {
        return Err(RobocubeError::ConversionError(
            "Empty logits array".to_string(),
        ));
    }

    let mut sorted = logits.clone();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

    let len = sorted.len();
    let sum: f32 = logits.iter().sum();

    Ok(LogitStatistics {
        min: sorted[0],
        max: sorted[len - 1],
        mean: sum / len as f32,
        median: sorted[len / 2],
        p25: sorted[len / 4],
        p75: sorted[3 * len / 4],
        p90: sorted[9 * len / 10],
        p95: sorted[19 * len / 20],
    })
}

/// Statistics about occupancy logit values
#[derive(Debug, Clone, Copy)]
pub struct LogitStatistics {
    pub min: f32,
    pub max: f32,
    pub mean: f32,
    pub median: f32,
    pub p25: f32,
    pub p75: f32,
    pub p90: f32,
    pub p95: f32,
}

impl LogitStatistics {
    /// Suggest a threshold based on the logit distribution
    ///
    /// Returns a threshold that would include approximately the given
    /// fraction of the total volume as occupied.
    pub fn suggest_threshold(&self, target_occupancy: f32) -> f32 {
        // Simple linear interpolation between p25 and p75
        // For more accurate results, keep the full distribution
        let target = target_occupancy.clamp(0.0, 1.0);

        if target < 0.25 {
            self.p75 + (self.max - self.p75) * (0.25 - target) / 0.25
        } else if target < 0.5 {
            self.median + (self.p75 - self.median) * (0.5 - target) / 0.25
        } else if target < 0.75 {
            self.p25 + (self.median - self.p25) * (0.75 - target) / 0.25
        } else {
            self.min + (self.p25 - self.min) * (1.0 - target) / 0.25
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::OccupancyResult;

    fn create_test_result() -> OccupancyResult {
        OccupancyResult {
            resolution: 4,
            bbox_min: [-1.0, -1.0, -1.0],
            bbox_max: [1.0, 1.0, 1.0],
            occupied_voxels: vec![[0, 0, 0], [1, 1, 1], [2, 2, 2], [3, 3, 3]],
            logits: None,
            metadata: None,
        }
    }

    fn create_test_result_with_logits() -> OccupancyResult {
        let mut logits = vec![-1.0f32; 64]; // 4^3 = 64
                                            // Set some voxels as occupied (positive logits)
        logits[0] = 0.5; // [0,0,0]
        logits[21] = 1.0; // [1,1,1]
        logits[42] = 2.0; // [2,2,2]
        logits[63] = 0.1; // [3,3,3]

        OccupancyResult {
            resolution: 4,
            bbox_min: [-1.0, -1.0, -1.0],
            bbox_max: [1.0, 1.0, 1.0],
            occupied_voxels: vec![[0, 0, 0], [1, 1, 1], [2, 2, 2], [3, 3, 3]],
            logits: Some(logits),
            metadata: None,
        }
    }

    #[test]
    fn test_occupancy_to_cube() {
        let result = create_test_result();
        let cubebox = occupancy_to_cube(&result, None).unwrap();

        assert_eq!(cubebox.depth, 2); // 4 = 2^2
        assert_eq!(cubebox.size, IVec3::splat(4));
    }

    #[test]
    fn test_occupancy_to_cube_custom_material() {
        let result = create_test_result();
        let cubebox = occupancy_to_cube(&result, Some(128)).unwrap();

        assert_eq!(cubebox.depth, 2);
    }

    #[test]
    fn test_occupancy_to_csm() {
        let result = create_test_result();
        let csm = occupancy_to_csm(&result, None).unwrap();

        // CSM should contain path expressions
        assert!(csm.contains(">"));
    }

    #[test]
    fn test_occupancy_to_cube_empty() {
        let result = OccupancyResult {
            resolution: 4,
            bbox_min: [-1.0, -1.0, -1.0],
            bbox_max: [1.0, 1.0, 1.0],
            occupied_voxels: vec![],
            logits: None,
            metadata: None,
        };

        let err = occupancy_to_cube(&result, None);
        assert!(err.is_err());
    }

    #[test]
    fn test_threshold_logits() {
        let result = create_test_result_with_logits();

        // Threshold at 0.0 should give 4 voxels
        let occupied = threshold_logits(&result, 0.0).unwrap();
        assert_eq!(occupied.len(), 4);

        // Threshold at 0.5 should give 2 voxels (logits 1.0 and 2.0)
        let occupied = threshold_logits(&result, 0.5).unwrap();
        assert_eq!(occupied.len(), 2);

        // Threshold at 2.0 should give 0 voxels
        let occupied = threshold_logits(&result, 2.0).unwrap();
        assert_eq!(occupied.len(), 0);
    }

    #[test]
    fn test_threshold_logits_no_logits() {
        let result = create_test_result(); // No logits
        let err = threshold_logits(&result, 0.0);
        assert!(err.is_err());
    }

    #[test]
    fn test_occupancy_to_cube_with_threshold() {
        let result = create_test_result_with_logits();

        let cubebox = occupancy_to_cube_with_threshold(&result, 0.0, None).unwrap();
        assert_eq!(cubebox.depth, 2);
    }

    #[test]
    fn test_compute_logit_statistics() {
        let result = create_test_result_with_logits();
        let stats = compute_logit_statistics(&result).unwrap();

        assert!(stats.min <= stats.p25);
        assert!(stats.p25 <= stats.median);
        assert!(stats.median <= stats.p75);
        assert!(stats.p75 <= stats.max);
    }

    #[test]
    fn test_logit_statistics_suggest_threshold() {
        let stats = LogitStatistics {
            min: -2.0,
            max: 2.0,
            mean: 0.0,
            median: 0.0,
            p25: -0.5,
            p75: 0.5,
            p90: 1.0,
            p95: 1.5,
        };

        // Higher target occupancy should give lower threshold
        let t1 = stats.suggest_threshold(0.1);
        let t2 = stats.suggest_threshold(0.5);
        let t3 = stats.suggest_threshold(0.9);

        assert!(t1 > t2);
        assert!(t2 > t3);
    }

    #[test]
    fn test_depth_calculation() {
        // Test various resolutions map to correct depths
        let test_cases = [(8, 3), (16, 4), (32, 5), (64, 6), (128, 7)];

        for (resolution, expected_depth) in test_cases {
            let result = OccupancyResult {
                resolution,
                bbox_min: [-1.0, -1.0, -1.0],
                bbox_max: [1.0, 1.0, 1.0],
                occupied_voxels: vec![[0, 0, 0]],
                logits: None,
                metadata: None,
            };

            let cubebox = occupancy_to_cube(&result, None).unwrap();
            assert_eq!(
                cubebox.depth, expected_depth,
                "Resolution {} should give depth {}",
                resolution, expected_depth
            );
        }
    }
}
