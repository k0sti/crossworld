use super::world_cube::WorldCube as WorldCubeInternal;
use super::WorldCube;

#[test]
fn test_basic_voxel_operations() {
    // Create WorldCube directly (no WASM wrapper needed for tests)
    let mut world_cube = WorldCubeInternal::new(3, 2, 0); // macro_depth=3, micro_depth=2

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
    let mut world_cube = WorldCubeInternal::new(3, 2, 0);

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
    let mut world_cube = WorldCubeInternal::new(3, 2, 0);

    // Test at depth 5, max coord should be 31 (2^5 - 1)
    world_cube.set_voxel_at_depth(31, 31, 31, 5, 52); // Should work
    world_cube.set_voxel_at_depth(0, 0, 0, 5, 52);    // Should work

    let _geometry = world_cube.generate_mesh();
}

#[test]
fn test_nested_depths() {
    let mut world_cube = WorldCubeInternal::new(3, 2, 0);

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
    let world_cube = WorldCube::new(3, 2, 0);

    // Try to borrow mutably while holding an immutable borrow
    // This simulates what might happen if generate_frame is called
    // while set_voxel_at_depth is still executing
    let _borrow1 = world_cube.inner.borrow();
    let _borrow2 = world_cube.inner.borrow_mut(); // This should panic
}
