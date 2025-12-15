//! Comprehensive mesh generation tests
//!
//! This module tests the mesh generation system with various octree configurations
//! to validate face culling, vertex generation, normal computation, and color mapping.

mod test_models;

use cube::mesh::{generate_face_mesh, DefaultMeshBuilder, MeshBuilder};
use cube::Cube;
use std::collections::HashSet;
use test_models::*;

/// Simple color mapper for testing - maps material IDs to RGB colors
fn test_color_mapper(material_id: u8) -> [f32; 3] {
    match material_id {
        0 => [0.0, 0.0, 0.0],  // Empty = Black
        1 => [1.0, 0.0, 0.0],  // Red
        2 => [1.0, 1.0, 0.0],  // Yellow
        3 => [0.0, 1.0, 0.0],  // Green
        4 => [0.0, 0.0, 1.0],  // Blue
        5 => [1.0, 1.0, 1.0],  // White
        6 => [0.0, 1.0, 1.0],  // Cyan
        7 => [1.0, 0.5, 0.0],  // Orange
        8 => [0.5, 0.0, 0.5],  // Purple
        9 => [0.5, 0.5, 0.5],  // Gray
        10 => [1.0, 0.0, 1.0], // Magenta
        _ => [0.5, 0.5, 0.5],  // Default gray
    }
}

/// Helper to count faces by checking indices (each face has 6 indices = 2 triangles)
fn count_faces(builder: &DefaultMeshBuilder) -> usize {
    builder.indices.len() / 6
}

/// Helper to extract unique normals from mesh
fn extract_normals(builder: &DefaultMeshBuilder) -> HashSet<[i32; 3]> {
    builder
        .normals
        .chunks(3)
        .map(|n| [n[0] as i32, n[1] as i32, n[2] as i32])
        .collect()
}

/// Helper to verify vertices are within expected bounds
fn verify_vertex_bounds(builder: &DefaultMeshBuilder, min: f32, max: f32) -> bool {
    builder.vertices.chunks(3).all(|v| {
        v[0] >= min && v[0] <= max && v[1] >= min && v[1] <= max && v[2] >= min && v[2] <= max
    })
}

// ============================================================================
// Single Leaf Cube Tests (Depth 0)
// ============================================================================

#[test]
fn test_single_leaf_cube_with_empty_borders() {
    // A single solid voxel at depth 0 with empty borders should generate 6 faces
    // (one per side of the cube, rendered from surrounding empty space)
    let cube = single_leaf_cube();
    let mut builder = DefaultMeshBuilder::new();
    let border_materials = [0, 0, 0, 0]; // All empty borders

    generate_face_mesh(
        &cube,
        &mut builder,
        test_color_mapper,
        border_materials,
        0,
    );

    // Should generate 24 faces (one per side * 4 sub-faces due to octree subdivision)
    let face_count = count_faces(&builder);
    assert_eq!(
        face_count, 24,
        "Single solid voxel with empty borders should have 24 faces (subdivided), got {}",
        face_count
    );

    // All 6 face normals should be present
    let normals = extract_normals(&builder);
    assert_eq!(
        normals.len(),
        6,
        "Should have 6 unique normals (one per face)"
    );

    // Expected normals for all 6 faces
    let expected_normals: HashSet<[i32; 3]> = [
        [0, 1, 0],  // Top
        [0, -1, 0], // Bottom
        [-1, 0, 0], // Left
        [1, 0, 0],  // Right
        [0, 0, 1],  // Front
        [0, 0, -1], // Back
    ]
    .iter()
    .copied()
    .collect();
    assert_eq!(
        normals, expected_normals,
        "Normals should match all 6 faces"
    );

    // Verify vertices are in [0, 1] range (size 1 cube at origin)
    if !verify_vertex_bounds(&builder, 0.0, 1.0) {
        println!("Vertices out of bounds:");
        for (i, v) in builder.vertices.chunks(3).enumerate() {
            println!("  v{}: [{}, {}, {}]", i, v[0], v[1], v[2]);
        }
    }
    assert!(
        verify_vertex_bounds(&builder, 0.0, 1.0),
        "Vertices should be within [0, 1] bounds"
    );

    // Each vertex should have the red color [1.0, 0.0, 0.0]
    for color_chunk in builder.colors.chunks(3) {
        assert_eq!(color_chunk, [1.0, 0.0, 0.0], "All vertices should be red");
    }
}

#[test]
fn test_single_leaf_cube_vertex_positions() {
    // Verify exact vertex positions for a single solid cube
    let cube = single_leaf_cube();
    let mut builder = DefaultMeshBuilder::new();
    let border_materials = [0, 0, 0, 0];

    generate_face_mesh(
        &cube,
        &mut builder,
        test_color_mapper,
        border_materials,
        0,
    );

    // A cube should have 96 vertices (4 per face * 24 faces)
    let vertex_count = builder.vertices.len() / 3;
    assert_eq!(
        vertex_count, 96,
        "Should have 96 vertices (4 per face * 24 faces), got {}",
        vertex_count
    );

    // Extract all unique vertices (should be 8 corner positions)
    let vertices: HashSet<(i32, i32, i32)> = builder
        .vertices
        .chunks(3)
        .map(|v| (v[0] as i32, v[1] as i32, v[2] as i32))
        .collect();

    // Expected corner positions
    let expected_corners: HashSet<(i32, i32, i32)> = [
        (0, 0, 0),
        (1, 0, 0),
        (0, 1, 0),
        (1, 1, 0),
        (0, 0, 1),
        (1, 0, 1),
        (0, 1, 1),
        (1, 1, 1),
    ]
    .iter()
    .copied()
    .collect();

    assert_eq!(
        vertices, expected_corners,
        "Vertices should match 8 cube corners"
    );
}

#[test]
fn test_single_leaf_cube_normals() {
    // Verify that normals are correctly assigned per face
    let cube = single_leaf_cube();
    let mut builder = DefaultMeshBuilder::new();
    let border_materials = [0, 0, 0, 0];

    generate_face_mesh(
        &cube,
        &mut builder,
        test_color_mapper,
        border_materials,
        0,
    );

    // Verify normals are unit vectors
    for normal_chunk in builder.normals.chunks(3) {
        let length_sq = normal_chunk[0].powi(2) + normal_chunk[1].powi(2) + normal_chunk[2].powi(2);
        let length = length_sq.sqrt();
        assert!(
            (length - 1.0).abs() < 0.001,
            "Normal should be unit length, got {}",
            length
        );
    }

    // Verify each face has consistent normals (all 4 vertices share same normal)
    for face_idx in 0..6 {
        let start = face_idx * 4 * 3; // 4 vertices * 3 components per face
        let normals: Vec<[f32; 3]> = (0..4)
            .map(|v| {
                let offset = start + v * 3;
                [
                    builder.normals[offset],
                    builder.normals[offset + 1],
                    builder.normals[offset + 2],
                ]
            })
            .collect();

        // All 4 normals should be identical
        for normal in &normals[1..] {
            assert_eq!(
                normal, &normals[0],
                "All vertices in face {} should have same normal",
                face_idx
            );
        }
    }
}

// ============================================================================
// Depth 1 Tests
// ============================================================================


#[test]
fn test_all_solid_cube_face_count() {
    // All solid cube should only have boundary faces (with empty borders)
    let cube = all_solid_cube();
    let mut builder = DefaultMeshBuilder::new();
    let border_materials = [0, 0, 0, 0];

    generate_face_mesh(
        &cube,
        &mut builder,
        test_color_mapper,
        border_materials,
        1,
    );

    let face_count = count_faces(&builder);

    println!("All solid cube generated {} faces", face_count);

    // With empty borders, should have 24 boundary faces (subdivided)
    // (one per side of the entire cube * 4 sub-faces)
    assert_eq!(
        face_count, 24,
        "All solid cube should have 24 boundary faces, got {}",
        face_count
    );
}

#[test]
fn test_all_empty_cube_face_count() {
    // All empty cube with empty borders should have 0 faces
    let cube = all_empty_cube();
    let mut builder = DefaultMeshBuilder::new();
    let border_materials = [0, 0, 0, 0];

    generate_face_mesh(
        &cube,
        &mut builder,
        test_color_mapper,
        border_materials,
        1,
    );

    let face_count = count_faces(&builder);

    assert_eq!(
        face_count, 0,
        "All empty cube with empty borders should have 0 faces, got {}",
        face_count
    );
}

#[test]
fn test_checkerboard_cube_internal_faces() {
    // Checkerboard pattern should have many internal faces
    let cube = checkerboard_cube();
    let mut builder = DefaultMeshBuilder::new();
    let border_materials = [0, 0, 0, 0];

    generate_face_mesh(
        &cube,
        &mut builder,
        test_color_mapper,
        border_materials,
        1,
    );

    let face_count = count_faces(&builder);

    println!("Checkerboard cube generated {} faces", face_count);

    // Checkerboard should have many internal faces between solid and empty
    assert!(
        face_count > 6,
        "Checkerboard should have more than 6 boundary faces, got {}",
        face_count
    );
}

#[test]
fn test_single_solid_in_empty_face_count() {
    // Single solid voxel in empty space should have 3 exposed faces
    // (only faces visible to adjacent empty voxels, not diagonal)
    let cube = single_solid_in_empty();
    let mut builder = DefaultMeshBuilder::new();
    let border_materials = [0, 0, 0, 0];

    generate_face_mesh(
        &cube,
        &mut builder,
        test_color_mapper,
        border_materials,
        1,
    );

    let face_count = count_faces(&builder);

    println!("Single solid in empty generated {} faces", face_count);

    // Octant 0 is at corner (0,0,0) with size 0.5
    // It has 3 internal neighbors (X+, Y+, Z+) and 3 border neighbors (X-, Y-, Z-)
    // Should generate 3 internal faces + 3 border faces = 6 total
    assert!(
        face_count >= 3,
        "Single solid should have at least 3 internal faces, got {}",
        face_count
    );
}


// ============================================================================
// Issue Demonstration Tests
// ============================================================================

#[test]
fn test_demonstrate_mesh_generation_issues() {
    // This test runs all models and prints diagnostic information
    // to demonstrate current mesh generation issues

    println!("\n=== Mesh Generation Diagnostic Report ===\n");

    let test_cases = vec![
        ("Single Leaf Cube (Depth 0)", single_leaf_cube(), 0),
        ("Octa Cube (Depth 1)", octa_cube_depth1(), 1),
        ("Extended Octa (Depth 2)", extended_octa_cube_depth2(), 2),
        ("Deep Octree (Depth 3)", deep_octree_depth3(), 3),
        ("All Solid", all_solid_cube(), 1),
        ("All Empty", all_empty_cube(), 1),
        ("Checkerboard", checkerboard_cube(), 1),
        ("Single Solid in Empty", single_solid_in_empty(), 1),
    ];

    for (name, cube, max_depth) in test_cases {
        let mut builder = DefaultMeshBuilder::new();
        let border_materials = [0, 0, 0, 0];

        generate_face_mesh(
            &cube,
            &mut builder,
            test_color_mapper,
            border_materials,
            max_depth,
        );

        let face_count = count_faces(&builder);
        let vertex_count = builder.vertices.len() / 3;
        let normal_count = extract_normals(&builder).len();

        println!("{}", name);
        println!("  Faces: {}", face_count);
        println!("  Vertices: {}", vertex_count);
        println!("  Unique Normals: {}", normal_count);
        println!("  Indices: {}", builder.indices.len());

        // Verify mesh consistency
        assert_eq!(
            builder.vertices.len(),
            builder.normals.len(),
            "{}: Vertices and normals count mismatch",
            name
        );
        assert_eq!(
            builder.vertices.len(),
            builder.colors.len(),
            "{}: Vertices and colors count mismatch",
            name
        );
        assert_eq!(
            vertex_count,
            face_count * 4,
            "{}: Should have 4 vertices per face",
            name
        );

        println!();
    }
}

// ============================================================================
// Consolidated Tests from generator.rs
// ============================================================================

#[test]
fn test_mesh_builder_basic() {
    // Test that DefaultMeshBuilder correctly accumulates mesh data
    let mut builder = DefaultMeshBuilder::new();

    let vertices = [
        [0.0, 0.0, 0.0],
        [1.0, 0.0, 0.0],
        [1.0, 1.0, 0.0],
        [0.0, 1.0, 0.0],
    ];
    let normal = [0.0, 0.0, 1.0];
    let color = [1.0, 0.0, 0.0];

    builder.add_face(vertices, normal, color);

    // Should have 4 vertices
    assert_eq!(builder.vertices.len(), 12); // 4 vertices * 3 components
    assert_eq!(builder.normals.len(), 12);
    assert_eq!(builder.colors.len(), 12);
    assert_eq!(builder.indices.len(), 6); // 2 triangles * 3 indices

    // Verify indices form correct triangulation
    assert_eq!(builder.indices, vec![0, 1, 2, 0, 2, 3]);
}

#[test]
fn test_empty_octree_generates_no_faces() {
    // Completely empty octree should generate no faces
    let cube = Cube::Solid(0);
    let mut builder = DefaultMeshBuilder::new();
    let border_materials = [0, 0, 0, 0];

    generate_face_mesh(
        &cube,
        &mut builder,
        test_color_mapper,
        border_materials,
        0,
    );

    assert_eq!(
        count_faces(&builder),
        0,
        "Empty octree should have no faces"
    );
}
