//! Conversion utilities for XCube models to Crossworld formats

use crate::types::{XCubeError, XCubeModel};

/// Convert XCube model to CSM (CubeScript Model) format
///
/// This function converts an XCube voxel model into Crossworld's
/// CSM text format, which can be parsed and rendered by the cube crate.
///
/// # Arguments
///
/// * `model` - The XCube model to convert
///
/// # Returns
///
/// A CSM-formatted string representation of the model
///
/// # Example
///
/// ```ignore
/// let model = client.fetch_model("abc123").await?;
/// let csm_string = xcube_to_csm(&model)?;
/// ```
pub fn xcube_to_csm(model: &XCubeModel) -> Result<String, XCubeError> {
    if model.voxels.is_empty() {
        return Err(XCubeError::ConversionError(
            "Model has no voxels".to_string()
        ));
    }

    // TODO: Implement actual conversion logic
    // This is a placeholder that will be implemented in the next phase
    // The conversion will need to:
    // 1. Build an octree structure from the voxel list
    // 2. Serialize it to CSM format (nested s[] and o[] syntax)
    // 3. Handle color palette mapping

    let placeholder = format!(
        "s[/* XCube model: {} ({} voxels) - conversion not yet implemented */]",
        model.name,
        model.voxels.len()
    );

    Ok(placeholder)
}

/// Convert XCube model to a simplified voxel grid
///
/// This creates a 3D array representation of the model,
/// which can be useful for intermediate processing.
pub fn xcube_to_grid(model: &XCubeModel) -> Result<Vec<Vec<Vec<Option<u8>>>>, XCubeError> {
    let dims = model.dimensions;
    let width = dims.width as usize;
    let height = dims.height as usize;
    let depth = dims.depth as usize;

    // Initialize empty grid
    let mut grid = vec![
        vec![
            vec![None; depth];
            height
        ];
        width
    ];

    // Fill grid with voxels
    for voxel in &model.voxels {
        let x = voxel.x as usize;
        let y = voxel.y as usize;
        let z = voxel.z as usize;

        if x < width && y < height && z < depth {
            grid[x][y][z] = Some(voxel.color_index);
        }
    }

    Ok(grid)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{Dimensions, Voxel};

    fn create_test_model() -> XCubeModel {
        XCubeModel {
            id: "test123".to_string(),
            name: "Test Model".to_string(),
            author: Some("Test Author".to_string()),
            description: None,
            voxels: vec![
                Voxel { x: 0, y: 0, z: 0, color_index: 1 },
                Voxel { x: 1, y: 0, z: 0, color_index: 2 },
            ],
            dimensions: Dimensions {
                width: 8,
                height: 8,
                depth: 8,
            },
            palette: None,
        }
    }

    #[test]
    fn test_xcube_to_grid() {
        let model = create_test_model();
        let grid = xcube_to_grid(&model).unwrap();

        assert_eq!(grid.len(), 8);
        assert_eq!(grid[0].len(), 8);
        assert_eq!(grid[0][0].len(), 8);

        assert_eq!(grid[0][0][0], Some(1));
        assert_eq!(grid[1][0][0], Some(2));
        assert_eq!(grid[2][0][0], None);
    }

    #[test]
    fn test_xcube_to_csm_placeholder() {
        let model = create_test_model();
        let csm = xcube_to_csm(&model).unwrap();

        assert!(csm.contains("Test Model"));
        assert!(csm.contains("2 voxels"));
    }
}
