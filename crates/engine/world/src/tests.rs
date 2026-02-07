use super::WorldCube;
use super::world_cube::WorldCube as WorldCubeInternal;
use cube::Cube;

#[test]
fn test_mesh_generation_produces_output() {
    // Test that mesh generation actually produces vertices and indices
    let world_cube = WorldCubeInternal::new(3, 5, 1, 12345);

    let geometry = world_cube.generate_mesh();

    println!("Vertices: {}", geometry.vertices().len());
    println!("Indices: {}", geometry.indices().len());
    println!("Normals: {}", geometry.normals().len());
    println!("Colors: {}", geometry.colors().len());

    assert!(
        !geometry.vertices().is_empty(),
        "Mesh should generate vertices"
    );
    assert!(
        !geometry.indices().is_empty(),
        "Mesh should generate indices"
    );
    assert!(
        geometry.vertices().len() % 3 == 0,
        "Vertices should be in groups of 3 (xyz)"
    );
    assert!(
        geometry.indices().len() % 3 == 0,
        "Indices should be in groups of 3 (triangles)"
    );

    // Verify we have matching normals and colors
    assert_eq!(
        geometry.vertices().len(),
        geometry.normals().len(),
        "Should have same number of vertex and normal components"
    );
    assert_eq!(
        geometry.vertices().len(),
        geometry.colors().len(),
        "Should have same number of vertex and color components"
    );
}

#[test]
fn test_basic_voxel_operations() {
    // Create WorldCube directly (no WASM wrapper needed for tests)
    let mut world_cube = WorldCubeInternal::new(3, 2, 0, 12345); // macro_depth=3, micro_depth=2, seed=12345

    // Test setting a voxel at depth 5 (macro 3 + micro 2)
    world_cube.set_voxel_at_depth(9, 23, 13, 5, 52);

    // Generate mesh - this should work
    let _geometry = world_cube.generate_mesh();

    // Set another voxel
    world_cube.set_voxel_at_depth(10, 20, 12, 5, 53);

    // Generate mesh again
    let _geometry2 = world_cube.generate_mesh();
}

#[test]
fn test_rapid_updates() {
    let mut world_cube = WorldCubeInternal::new(3, 2, 0, 12345);

    // Simulate rapid voxel updates like user drawing
    for i in 0..10 {
        world_cube.set_voxel_at_depth(i, i, i, 5, 52);
    }

    // Generate mesh
    let _geometry = world_cube.generate_mesh();

    // More updates
    for i in 10..20 {
        world_cube.set_voxel_at_depth(i, i, i, 5, 52);
    }

    // Generate mesh again
    let _geometry2 = world_cube.generate_mesh();
}

#[test]
fn test_boundary_coordinates() {
    let mut world_cube = WorldCubeInternal::new(3, 2, 0, 12345);

    // Test at depth 5, max coord should be 31 (2^5 - 1)
    world_cube.set_voxel_at_depth(31, 31, 31, 5, 52); // Should work
    world_cube.set_voxel_at_depth(0, 0, 0, 5, 52); // Should work

    let _geometry = world_cube.generate_mesh();
}

#[test]
fn test_nested_depths() {
    let mut world_cube = WorldCubeInternal::new(3, 2, 0, 12345);

    // Test updating at different depths
    world_cube.set_voxel_at_depth(4, 4, 4, 3, 10); // Macro depth
    world_cube.set_voxel_at_depth(8, 8, 8, 4, 20); // Micro depth 1
    world_cube.set_voxel_at_depth(16, 16, 16, 5, 30); // Micro depth 2

    let _geometry = world_cube.generate_mesh();
}

#[test]
#[should_panic(expected = "already borrowed")]
fn test_refcell_borrow_conflict() {
    // Test the WASM wrapper's RefCell behavior
    let world_cube = WorldCube::new(3, 2, 0, 12345);

    // Try to borrow mutably while holding an immutable borrow
    // This simulates what might happen if generate_frame is called
    // while set_voxel_at_depth is still executing
    let _borrow1 = world_cube.inner.borrow();
    let _borrow2 = world_cube.inner.borrow_mut(); // This should panic
}

#[test]
fn test_border_layers() {
    // Create WorldCube with 1 border layer
    let world_cube = WorldCubeInternal::new(3, 2, 1, 12345);

    // The root should be a Cubes variant (octa) with border colors
    let root = world_cube.get_root();

    // Verify root is an octa (Cubes variant with 8 children)
    if let Cube::Cubes(octants) = root {
        // Verify we have 8 octants
        assert_eq!(octants.len(), 8);

        // With centered world at position (1,1,1) depth=2, all 8 octants at depth=1
        // will contain subdivisions (Cubes) that have the world and borders
        // So we can't simply check for Solid colors at depth=1

        // Instead, verify that the structure is subdivided (not all solid)
        let mut has_subdivisions = false;
        for octant in octants.iter() {
            if matches!(octant.as_ref(), Cube::Cubes(_)) {
                has_subdivisions = true;
                break;
            }
        }
        assert!(
            has_subdivisions,
            "Border layer should contain subdivisions for centered world"
        );
    } else {
        panic!("Root should be Cubes variant with border layer");
    }

    // Verify mesh can be generated without panic
    let _geometry = world_cube.generate_mesh();
}

#[test]
fn test_multiple_border_layers() {
    // Create WorldCube with 2 border layers
    let world_cube = WorldCubeInternal::new(3, 2, 2, 12345);

    // With 2 layers, the outer layer wraps the inner layer which wraps the world
    let root = world_cube.get_root();

    // Root should be an octa
    if let Cube::Cubes(outer_octants) = root {
        // Octant 0 of outer layer should contain another octa (the inner border layer)
        if let Cube::Cubes(inner_octants) = outer_octants[0].as_ref() {
            // The inner layer's octant 0 should contain the world
            match inner_octants[0].as_ref() {
                Cube::Solid(color) => {
                    assert!(
                        *color != 1 && *color != 63,
                        "Inner octant 0 should contain world"
                    );
                }
                _ => {
                    // Non-solid is good
                }
            }
        } else {
            panic!("Outer octant 0 should contain inner border layer (Cubes variant)");
        }
    } else {
        panic!("Root should be Cubes variant with border layers");
    }

    // Verify mesh can be generated
    let _geometry = world_cube.generate_mesh();
}

#[test]
fn test_no_border_layers() {
    // Create WorldCube with 0 border layers (original behavior)
    let world_cube = WorldCubeInternal::new(3, 2, 0, 12345);

    // Root should be the original world structure, not wrapped in an octa
    // It will be subdivided terrain, not a simple octa of solid colors
    let root = world_cube.get_root();

    // The root shouldn't be a simple solid border color
    if let Cube::Solid(color) = root {
        // If it happens to be solid (unlikely with terrain generation),
        // it shouldn't be our border colors
        assert!(
            *color != 1 && *color != 63,
            "Root should be generated world, not border colors"
        );
    }

    // Verify mesh can be generated
    let _geometry = world_cube.generate_mesh();
}
