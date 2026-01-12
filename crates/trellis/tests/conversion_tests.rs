//! Integration tests for Trellis mesh to voxel conversion pipeline
//!
//! These tests verify the full pipeline: mesh -> voxels -> CSM

use trellis::convert::{trellis_to_csm, trellis_to_cube, voxelize_mesh, VoxelizeConfig};
use trellis::types::TrellisResult;

/// Create a simple unit cube mesh for testing
fn create_unit_cube() -> TrellisResult {
    let vertices = vec![
        // Front face
        [-0.5, -0.5, 0.5],
        [0.5, -0.5, 0.5],
        [0.5, 0.5, 0.5],
        [-0.5, 0.5, 0.5],
        // Back face
        [-0.5, -0.5, -0.5],
        [0.5, -0.5, -0.5],
        [0.5, 0.5, -0.5],
        [-0.5, 0.5, -0.5],
    ];

    let faces = vec![
        // Front
        [0, 1, 2],
        [0, 2, 3],
        // Back
        [4, 6, 5],
        [4, 7, 6],
        // Left
        [0, 3, 7],
        [0, 7, 4],
        // Right
        [1, 5, 6],
        [1, 6, 2],
        // Top
        [3, 2, 6],
        [3, 6, 7],
        // Bottom
        [0, 4, 5],
        [0, 5, 1],
    ];

    let vertex_colors = Some(vec![
        [1.0, 0.0, 0.0], // Red
        [0.0, 1.0, 0.0], // Green
        [0.0, 0.0, 1.0], // Blue
        [1.0, 1.0, 0.0], // Yellow
        [1.0, 0.0, 1.0], // Magenta
        [0.0, 1.0, 1.0], // Cyan
        [1.0, 1.0, 1.0], // White
        [0.5, 0.5, 0.5], // Gray
    ]);

    TrellisResult {
        vertices,
        faces,
        vertex_colors,
        vertex_normals: None,
        glb_data: None,
    }
}

/// Create a simple pyramid mesh for testing
fn create_pyramid() -> TrellisResult {
    let vertices = vec![
        // Base (square)
        [-0.5, 0.0, -0.5],
        [0.5, 0.0, -0.5],
        [0.5, 0.0, 0.5],
        [-0.5, 0.0, 0.5],
        // Apex
        [0.0, 1.0, 0.0],
    ];

    let faces = vec![
        // Base (2 triangles)
        [0, 1, 2],
        [0, 2, 3],
        // Sides (4 triangles)
        [0, 4, 1],
        [1, 4, 2],
        [2, 4, 3],
        [3, 4, 0],
    ];

    let vertex_colors = Some(vec![
        [1.0, 0.0, 0.0], // Red
        [0.0, 1.0, 0.0], // Green
        [0.0, 0.0, 1.0], // Blue
        [1.0, 1.0, 0.0], // Yellow
        [1.0, 1.0, 1.0], // White (apex)
    ]);

    TrellisResult {
        vertices,
        faces,
        vertex_colors,
        vertex_normals: None,
        glb_data: None,
    }
}

/// Create a simple flat quad (two triangles) for testing
fn create_flat_quad() -> TrellisResult {
    let vertices = vec![
        [-0.5, 0.0, -0.5],
        [0.5, 0.0, -0.5],
        [0.5, 0.0, 0.5],
        [-0.5, 0.0, 0.5],
    ];

    let faces = vec![[0, 1, 2], [0, 2, 3]];

    TrellisResult {
        vertices,
        faces,
        vertex_colors: None,
        vertex_normals: None,
        glb_data: None,
    }
}

// =============================================================================
// Full Pipeline Tests
// =============================================================================

#[test]
fn test_full_pipeline_cube() {
    let mesh = create_unit_cube();

    // Step 1: Voxelize mesh
    let config = VoxelizeConfig::new(5);
    let voxels = voxelize_mesh(&mesh.vertices, &mesh.faces, &config);

    assert!(!voxels.is_empty(), "Voxelization should produce voxels");

    // Step 2: Convert to Cube octree
    let _cube = trellis_to_cube(&mesh, 5).expect("Cube conversion should succeed");

    // Step 3: Convert to CSM
    let csm = trellis_to_csm(&mesh).expect("CSM conversion should succeed");

    assert!(!csm.is_empty(), "CSM should not be empty");
}

#[test]
fn test_full_pipeline_pyramid() {
    let mesh = create_pyramid();

    let cube = trellis_to_cube(&mesh, 5).expect("Pyramid should convert to cube");
    let csm = trellis_to_csm(&mesh).expect("Pyramid should convert to CSM");

    assert!(!csm.is_empty());

    // Pyramid is asymmetric - verify conversion completed
    match cube {
        cube::Cube::Solid(_) | cube::Cube::Cubes(_) | _ => {
            // Valid structure
        }
    }
}

#[test]
fn test_full_pipeline_flat_quad() {
    let mesh = create_flat_quad();

    let cube = trellis_to_cube(&mesh, 5).expect("Flat quad should convert to cube");
    let csm = trellis_to_csm(&mesh).expect("Flat quad should convert to CSM");

    assert!(!csm.is_empty());

    // Flat quad is 2D - should still produce valid voxels
    match cube {
        cube::Cube::Solid(_) | cube::Cube::Cubes(_) | _ => {
            // Valid structure
        }
    }
}

// =============================================================================
// Voxelization Tests
// =============================================================================

#[test]
fn test_voxelization_produces_bounded_output() {
    let mesh = create_unit_cube();

    for depth in [3, 4, 5, 6] {
        let config = VoxelizeConfig::new(depth);
        let voxels = voxelize_mesh(&mesh.vertices, &mesh.faces, &config);

        let grid_size = 1 << depth;

        for voxel in &voxels {
            assert!(
                voxel.x >= 0 && voxel.x < grid_size,
                "Voxel x out of bounds at depth {}",
                depth
            );
            assert!(
                voxel.y >= 0 && voxel.y < grid_size,
                "Voxel y out of bounds at depth {}",
                depth
            );
            assert!(
                voxel.z >= 0 && voxel.z < grid_size,
                "Voxel z out of bounds at depth {}",
                depth
            );
        }
    }
}

#[test]
fn test_voxelization_higher_depth_more_voxels() {
    let mesh = create_unit_cube();

    let config_low = VoxelizeConfig::new(4);
    let config_high = VoxelizeConfig::new(6);

    let voxels_low = voxelize_mesh(&mesh.vertices, &mesh.faces, &config_low);
    let voxels_high = voxelize_mesh(&mesh.vertices, &mesh.faces, &config_high);

    // Higher depth should generally produce more voxels
    assert!(
        voxels_high.len() >= voxels_low.len(),
        "Higher depth should produce at least as many voxels"
    );
}

#[test]
fn test_voxelization_with_interior_fill() {
    let mesh = create_unit_cube();

    let config_surface = VoxelizeConfig::new(5).with_fill_interior(false);
    let config_filled = VoxelizeConfig::new(5).with_fill_interior(true);

    let voxels_surface = voxelize_mesh(&mesh.vertices, &mesh.faces, &config_surface);
    let voxels_filled = voxelize_mesh(&mesh.vertices, &mesh.faces, &config_filled);

    // Interior fill should produce more voxels for a solid cube
    assert!(
        voxels_filled.len() >= voxels_surface.len(),
        "Interior fill should produce at least as many voxels"
    );
}

// =============================================================================
// CSM Format Tests
// =============================================================================

#[test]
fn test_csm_output_is_valid_format() {
    let mesh = create_unit_cube();
    let csm = trellis_to_csm(&mesh).expect("CSM conversion should succeed");

    // CSM should contain valid characters only
    for ch in csm.chars() {
        assert!(
            ch.is_alphanumeric()
                || ch.is_whitespace()
                || ch == '['
                || ch == ']'
                || ch == '>'
                || ch == 's'
                || ch == 'o',
            "CSM contains invalid character: {:?}",
            ch
        );
    }
}

#[test]
fn test_csm_can_be_parsed_by_cube_crate() {
    let mesh = create_unit_cube();
    let csm = trellis_to_csm(&mesh).expect("CSM conversion should succeed");

    // The CSM format should be parseable back into a Cube structure
    let parsed_result = cube::parse_csm(&csm);
    assert!(parsed_result.is_ok(), "CSM should be parseable");
}

// =============================================================================
// Edge Cases and Error Handling
// =============================================================================

#[test]
fn test_conversion_empty_mesh_fails() {
    let empty = TrellisResult {
        vertices: vec![],
        faces: vec![],
        vertex_colors: None,
        vertex_normals: None,
        glb_data: None,
    };

    let result = trellis_to_cube(&empty, 5);
    assert!(result.is_err(), "Empty mesh should fail conversion");
}

#[test]
fn test_conversion_vertices_no_faces_fails() {
    let no_faces = TrellisResult {
        vertices: vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]],
        faces: vec![],
        vertex_colors: None,
        vertex_normals: None,
        glb_data: None,
    };

    let result = trellis_to_cube(&no_faces, 5);
    assert!(result.is_err(), "Mesh with no faces should fail conversion");
}

#[test]
fn test_conversion_degenerate_mesh_fails() {
    // All vertices at the same point
    let degenerate = TrellisResult {
        vertices: vec![[0.5, 0.5, 0.5], [0.5, 0.5, 0.5], [0.5, 0.5, 0.5]],
        faces: vec![[0, 1, 2]],
        vertex_colors: None,
        vertex_normals: None,
        glb_data: None,
    };

    let result = trellis_to_cube(&degenerate, 5);
    assert!(result.is_err(), "Degenerate mesh should fail conversion");
}

#[test]
fn test_conversion_without_vertex_colors() {
    let no_colors = TrellisResult {
        vertices: vec![
            [-0.5, -0.5, 0.0],
            [0.5, -0.5, 0.0],
            [0.5, 0.5, 0.0],
            [-0.5, 0.5, 0.0],
        ],
        faces: vec![[0, 1, 2], [0, 2, 3]],
        vertex_colors: None, // No colors provided
        vertex_normals: None,
        glb_data: None,
    };

    // Should still succeed with default material
    let result = trellis_to_cube(&no_colors, 5);
    assert!(
        result.is_ok(),
        "Conversion should succeed without vertex colors"
    );
}

// =============================================================================
// Mesh Transformation Tests
// =============================================================================

#[test]
fn test_scaled_mesh_produces_same_relative_structure() {
    // Original mesh
    let original = create_unit_cube();

    // Scaled mesh (2x larger)
    let scaled = TrellisResult {
        vertices: original
            .vertices
            .iter()
            .map(|[x, y, z]| [x * 2.0, y * 2.0, z * 2.0])
            .collect(),
        faces: original.faces.clone(),
        vertex_colors: original.vertex_colors.clone(),
        vertex_normals: None,
        glb_data: None,
    };

    let config = VoxelizeConfig::new(5);
    let voxels_original = voxelize_mesh(&original.vertices, &original.faces, &config);
    let voxels_scaled = voxelize_mesh(&scaled.vertices, &scaled.faces, &config);

    // Both should produce similar voxel counts (normalization should handle scale)
    // Allow some tolerance due to sampling differences
    let ratio = voxels_scaled.len() as f32 / voxels_original.len() as f32;
    assert!(
        ratio > 0.5 && ratio < 2.0,
        "Scaled mesh should produce similar voxel count (ratio: {})",
        ratio
    );
}

#[test]
fn test_translated_mesh_produces_same_structure() {
    // Original mesh
    let original = create_unit_cube();

    // Translated mesh (offset by 100 units)
    let translated = TrellisResult {
        vertices: original
            .vertices
            .iter()
            .map(|[x, y, z]| [x + 100.0, y + 100.0, z + 100.0])
            .collect(),
        faces: original.faces.clone(),
        vertex_colors: original.vertex_colors.clone(),
        vertex_normals: None,
        glb_data: None,
    };

    let config = VoxelizeConfig::new(5);
    let voxels_original = voxelize_mesh(&original.vertices, &original.faces, &config);
    let voxels_translated = voxelize_mesh(&translated.vertices, &translated.faces, &config);

    // Translation should not affect voxel count significantly
    assert_eq!(
        voxels_original.len(),
        voxels_translated.len(),
        "Translation should not change voxel count"
    );
}

// =============================================================================
// Depth Variation Tests
// =============================================================================

#[test]
fn test_all_depths_produce_valid_output() {
    let mesh = create_unit_cube();

    for depth in 2..=7 {
        let result = trellis_to_cube(&mesh, depth);
        assert!(
            result.is_ok(),
            "Depth {} should produce valid output",
            depth
        );
    }
}

#[test]
fn test_depth_affects_voxel_precision() {
    let mesh = create_unit_cube();

    let config_d4 = VoxelizeConfig::new(4); // 16x16x16
    let config_d6 = VoxelizeConfig::new(6); // 64x64x64

    let voxels_d4 = voxelize_mesh(&mesh.vertices, &mesh.faces, &config_d4);
    let voxels_d6 = voxelize_mesh(&mesh.vertices, &mesh.faces, &config_d6);

    // Higher depth should have more precision (more voxels for curved/detailed surfaces)
    assert!(
        voxels_d6.len() >= voxels_d4.len(),
        "Higher depth should have at least as many voxels"
    );
}
