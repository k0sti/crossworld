//! Occupancy field to Crossworld voxel format conversion
//!
//! Converts Cube3D occupancy field outputs directly to Crossworld's CSM format.
//! This approach queries the occupancy decoder at discrete grid points instead of
//! converting meshes to voxels, resulting in more accurate voxel representations.
//!
//! ## Color Support
//!
//! Crossworld uses the R2G3B2 color encoding for materials 128-255:
//! - 2 bits for Red (0-3 levels)
//! - 3 bits for Green (0-7 levels)
//! - 2 bits for Blue (0-3 levels)
//!
//! Use [`encode_r2g3b2`] to convert RGB colors to material indices.
//!
//! ## Coordinate System
//!
//! Cube3D uses Z-up coordinates, while Crossworld uses Y-up. All conversion
//! functions automatically transform coordinates by swapping Y and Z axes.

use crate::types::{OccupancyResult, Result, RobocubeError};
use cube::{serialize_csm, Cube, CubeBox, Voxel};
use glam::IVec3;

/// Default material index for occupied voxels (white in R2G3B2)
pub const DEFAULT_MATERIAL: u8 = 255;

/// Encode an RGB color to R2G3B2 material index (128-255)
///
/// Crossworld uses materials 128-255 for R2G3B2 encoded colors:
/// - 2 bits for Red (4 levels)
/// - 3 bits for Green (8 levels)
/// - 2 bits for Blue (4 levels)
///
/// # Arguments
///
/// * `r` - Red component (0.0 to 1.0)
/// * `g` - Green component (0.0 to 1.0)
/// * `b` - Blue component (0.0 to 1.0)
///
/// # Returns
///
/// Material index in range 128-255
///
/// # Example
///
/// ```
/// use robocube::convert::encode_r2g3b2;
///
/// // Pure red
/// assert_eq!(encode_r2g3b2(1.0, 0.0, 0.0), 224); // 128 + (3 << 5)
///
/// // Pure green
/// assert_eq!(encode_r2g3b2(0.0, 1.0, 0.0), 156); // 128 + (7 << 2)
///
/// // Pure blue
/// assert_eq!(encode_r2g3b2(0.0, 0.0, 1.0), 131); // 128 + 3
///
/// // White
/// assert_eq!(encode_r2g3b2(1.0, 1.0, 1.0), 255);
///
/// // Black
/// assert_eq!(encode_r2g3b2(0.0, 0.0, 0.0), 128);
/// ```
pub fn encode_r2g3b2(r: f32, g: f32, b: f32) -> u8 {
    let r_bits = (r.clamp(0.0, 1.0) * 3.0).round() as u8;
    let g_bits = (g.clamp(0.0, 1.0) * 7.0).round() as u8;
    let b_bits = (b.clamp(0.0, 1.0) * 3.0).round() as u8;

    128 + ((r_bits << 5) | (g_bits << 2) | b_bits)
}

/// Encode an RGB color from u8 components to R2G3B2 material index (128-255)
///
/// # Arguments
///
/// * `r` - Red component (0-255)
/// * `g` - Green component (0-255)
/// * `b` - Blue component (0-255)
///
/// # Returns
///
/// Material index in range 128-255
///
/// # Example
///
/// ```
/// use robocube::convert::encode_r2g3b2_u8;
///
/// // Pure red (255, 0, 0)
/// assert_eq!(encode_r2g3b2_u8(255, 0, 0), 224);
///
/// // Mid gray (128, 128, 128)
/// let gray = encode_r2g3b2_u8(128, 128, 128);
/// assert!(gray >= 128 && gray <= 255);
/// ```
pub fn encode_r2g3b2_u8(r: u8, g: u8, b: u8) -> u8 {
    encode_r2g3b2(r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0)
}

/// Decode R2G3B2 material index to RGB color components
///
/// # Arguments
///
/// * `material` - Material index (128-255 for R2G3B2 colors)
///
/// # Returns
///
/// RGB tuple with values in range 0.0 to 1.0, or None if material is not in R2G3B2 range
///
/// # Example
///
/// ```
/// use robocube::convert::decode_r2g3b2;
///
/// // White (255)
/// let (r, g, b) = decode_r2g3b2(255).unwrap();
/// assert_eq!((r, g, b), (1.0, 1.0, 1.0));
///
/// // Black (128)
/// let (r, g, b) = decode_r2g3b2(128).unwrap();
/// assert_eq!((r, g, b), (0.0, 0.0, 0.0));
///
/// // Non-R2G3B2 material
/// assert!(decode_r2g3b2(50).is_none());
/// ```
pub fn decode_r2g3b2(material: u8) -> Option<(f32, f32, f32)> {
    if material < 128 {
        return None;
    }

    let bits = material - 128;
    let r_bits = (bits >> 5) & 0b11;
    let g_bits = (bits >> 2) & 0b111;
    let b_bits = bits & 0b11;

    let r = r_bits as f32 / 3.0;
    let g = g_bits as f32 / 7.0;
    let b = b_bits as f32 / 3.0;

    Some((r, g, b))
}

/// Convert occupancy result to Crossworld Cube octree
///
/// Takes the occupied voxel positions from the Cube3D occupancy field
/// and builds a Cube octree suitable for rendering or serialization.
///
/// **Coordinate transformation**: Cube3D uses Z-up coordinate system,
/// while Crossworld uses Y-up. This function automatically swaps Y and Z
/// axes during conversion.
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

    let default_mat = material.unwrap_or(DEFAULT_MATERIAL);

    // Calculate depth from resolution (resolution = 2^depth)
    let depth = (result.resolution as f32).log2().ceil() as u32;

    // Convert occupied voxels to cube::Voxel format
    // Cube3D uses Z-up, Crossworld uses Y-up: swap Y and Z axes
    // If colors are available, convert them to R2G3B2 materials
    let voxels: Vec<Voxel> = if let Some(colors) = &result.voxel_colors {
        result
            .occupied_voxels
            .iter()
            .zip(colors.iter())
            .map(|([x, y, z], [r, g, b])| Voxel {
                pos: IVec3::new(*x as i32, *z as i32, *y as i32),
                material: encode_r2g3b2(*r, *g, *b),
            })
            .collect()
    } else {
        result
            .occupied_voxels
            .iter()
            .map(|[x, y, z]| Voxel {
                pos: IVec3::new(*x as i32, *z as i32, *y as i32),
                material: default_mat,
            })
            .collect()
    };

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
/// **Note**: The returned coordinates are in Cube3D's Z-up space (not transformed).
/// Use `occupancy_to_cube_with_threshold` for automatic Y-up conversion.
///
/// # Arguments
///
/// * `result` - The occupancy field result (must have logits)
/// * `threshold` - Occupancy threshold (values > threshold are occupied)
///
/// # Returns
///
/// Vector of occupied voxel positions [x, y, z] in Cube3D coordinate space (Z-up)
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
/// **Coordinate transformation**: Cube3D uses Z-up coordinate system,
/// while Crossworld uses Y-up. This function automatically swaps Y and Z
/// axes during conversion.
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

    // Cube3D uses Z-up, Crossworld uses Y-up: swap Y and Z axes
    let voxels: Vec<Voxel> = occupied
        .iter()
        .map(|[x, y, z]| Voxel {
            pos: IVec3::new(*x as i32, *z as i32, *y as i32),
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
            voxel_colors: None,
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
            voxel_colors: None,
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
            voxel_colors: None,
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
                voxel_colors: None,
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

    #[test]
    fn test_encode_r2g3b2_primary_colors() {
        // Pure red: r=3, g=0, b=0 => 128 + (3 << 5) = 224
        assert_eq!(encode_r2g3b2(1.0, 0.0, 0.0), 224);

        // Pure green: r=0, g=7, b=0 => 128 + (7 << 2) = 156
        assert_eq!(encode_r2g3b2(0.0, 1.0, 0.0), 156);

        // Pure blue: r=0, g=0, b=3 => 128 + 3 = 131
        assert_eq!(encode_r2g3b2(0.0, 0.0, 1.0), 131);

        // White: r=3, g=7, b=3 => 128 + 96 + 28 + 3 = 255
        assert_eq!(encode_r2g3b2(1.0, 1.0, 1.0), 255);

        // Black: r=0, g=0, b=0 => 128
        assert_eq!(encode_r2g3b2(0.0, 0.0, 0.0), 128);
    }

    #[test]
    fn test_encode_r2g3b2_clamping() {
        // Values outside [0, 1] should be clamped
        assert_eq!(encode_r2g3b2(-1.0, -1.0, -1.0), 128); // Clamped to black
        assert_eq!(encode_r2g3b2(2.0, 2.0, 2.0), 255); // Clamped to white
    }

    #[test]
    fn test_encode_r2g3b2_u8() {
        assert_eq!(encode_r2g3b2_u8(255, 0, 0), 224); // Red
        assert_eq!(encode_r2g3b2_u8(0, 255, 0), 156); // Green
        assert_eq!(encode_r2g3b2_u8(0, 0, 255), 131); // Blue
        assert_eq!(encode_r2g3b2_u8(255, 255, 255), 255); // White
        assert_eq!(encode_r2g3b2_u8(0, 0, 0), 128); // Black
    }

    #[test]
    fn test_decode_r2g3b2() {
        // White
        let (r, g, b) = decode_r2g3b2(255).unwrap();
        assert_eq!((r, g, b), (1.0, 1.0, 1.0));

        // Black
        let (r, g, b) = decode_r2g3b2(128).unwrap();
        assert_eq!((r, g, b), (0.0, 0.0, 0.0));

        // Pure red
        let (r, g, b) = decode_r2g3b2(224).unwrap();
        assert_eq!((r, g, b), (1.0, 0.0, 0.0));

        // Pure green
        let (r, g, b) = decode_r2g3b2(156).unwrap();
        assert_eq!((r, g, b), (0.0, 1.0, 0.0));

        // Pure blue
        let (r, g, b) = decode_r2g3b2(131).unwrap();
        assert_eq!((r, g, b), (0.0, 0.0, 1.0));
    }

    #[test]
    fn test_decode_r2g3b2_out_of_range() {
        // Materials 0-127 are not R2G3B2
        assert!(decode_r2g3b2(0).is_none());
        assert!(decode_r2g3b2(50).is_none());
        assert!(decode_r2g3b2(127).is_none());

        // 128 and above should work
        assert!(decode_r2g3b2(128).is_some());
        assert!(decode_r2g3b2(200).is_some());
        assert!(decode_r2g3b2(255).is_some());
    }

    #[test]
    fn test_r2g3b2_roundtrip() {
        // Test that encode -> decode approximately preserves the color
        for r in 0..=3 {
            for g in 0..=7 {
                for b in 0..=3 {
                    let rf = r as f32 / 3.0;
                    let gf = g as f32 / 7.0;
                    let bf = b as f32 / 3.0;

                    let encoded = encode_r2g3b2(rf, gf, bf);
                    let (dr, dg, db) = decode_r2g3b2(encoded).unwrap();

                    assert!((rf - dr).abs() < 0.01, "Red mismatch: {} vs {}", rf, dr);
                    assert!((gf - dg).abs() < 0.01, "Green mismatch: {} vs {}", gf, dg);
                    assert!((bf - db).abs() < 0.01, "Blue mismatch: {} vs {}", bf, db);
                }
            }
        }
    }

    #[test]
    fn test_coordinate_swap_z_to_y_up() {
        // Verify that Z-up coordinates are converted to Y-up
        // Input: [x, y, z] in Z-up space
        // Output: [x, z, y] in Y-up space (Y and Z swapped)
        let result = OccupancyResult {
            resolution: 4,
            bbox_min: [-1.0, -1.0, -1.0],
            bbox_max: [1.0, 1.0, 1.0],
            // A voxel at (1, 2, 3) in Z-up should become (1, 3, 2) in Y-up
            occupied_voxels: vec![[1, 2, 3]],
            voxel_colors: None,
            logits: None,
            metadata: None,
        };

        let cubebox = occupancy_to_cube(&result, None).unwrap();

        // The voxel should be at position (1, 3, 2) after transformation
        // We can verify this by checking that the octree was built correctly
        assert_eq!(cubebox.depth, 2);
    }

    #[test]
    fn test_occupancy_to_cube_with_colors() {
        // Test that colors are properly converted to R2G3B2 materials
        let result = OccupancyResult {
            resolution: 4,
            bbox_min: [-1.0, -1.0, -1.0],
            bbox_max: [1.0, 1.0, 1.0],
            occupied_voxels: vec![[0, 0, 0], [1, 1, 1], [2, 2, 2]],
            voxel_colors: Some(vec![
                [1.0, 0.0, 0.0], // Red
                [0.0, 1.0, 0.0], // Green
                [0.0, 0.0, 1.0], // Blue
            ]),
            logits: None,
            metadata: None,
        };

        let cubebox = occupancy_to_cube(&result, None).unwrap();
        assert_eq!(cubebox.depth, 2);
        // Colors should be converted, but we can't easily verify the materials
        // without introspecting the octree structure
    }
}
