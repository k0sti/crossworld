//! Test to verify 3D texture data generation for GL tracer

use cube::Cube;
use glam::Vec3;
use renderer::scenes::create_octa_cube;

/// Sample cube at a normalized position [0,1] in each axis
fn sample_cube_at_position(cube: &Cube<u8>, pos: Vec3, max_depth: u32) -> u8 {
    fn sample_recursive(cube: &Cube<u8>, pos: Vec3, depth: u32) -> u8 {
        match cube {
            Cube::Solid(value) => *value,
            Cube::Cubes(children) if depth > 0 => {
                // Determine which octant the position falls into
                let octant_x = if pos.x >= 0.5 { 1 } else { 0 };
                let octant_y = if pos.y >= 0.5 { 1 } else { 0 };
                let octant_z = if pos.z >= 0.5 { 1 } else { 0 };
                let octant = octant_x * 4 + octant_y * 2 + octant_z;

                // Recursively sample within the child octant
                let child_pos = Vec3::new(
                    (pos.x - octant_x as f32 * 0.5) * 2.0,
                    (pos.y - octant_y as f32 * 0.5) * 2.0,
                    (pos.z - octant_z as f32 * 0.5) * 2.0,
                );
                sample_recursive(&children[octant], child_pos, depth - 1)
            }
            _ => cube.id(), // For Planes/Slices or depth 0, return representative ID
        }
    }

    sample_recursive(cube, pos, max_depth)
}

#[test]
fn test_3d_texture_data_generation() {
    println!("\n========================================");
    println!("3D TEXTURE DATA GENERATION TEST");
    println!("========================================\n");

    let cube = create_octa_cube();
    const SIZE: usize = 8;
    let mut voxel_data = vec![0u8; SIZE * SIZE * SIZE];

    println!("Generating 8x8x8 voxel grid from octree...\n");

    let mut solid_count = 0;
    let mut empty_count = 0;

    for z in 0..SIZE {
        for y in 0..SIZE {
            for x in 0..SIZE {
                let pos = Vec3::new(
                    (x as f32 + 0.5) / SIZE as f32,
                    (y as f32 + 0.5) / SIZE as f32,
                    (z as f32 + 0.5) / SIZE as f32,
                );

                let value = sample_cube_at_position(&*cube, pos, 3);
                let idx = x + y * SIZE + z * SIZE * SIZE;
                voxel_data[idx] = if value != 0 { 255 } else { 0 };

                if value != 0 {
                    solid_count += 1;
                } else {
                    empty_count += 1;
                }
            }
        }
    }

    println!("Voxel Statistics:");
    println!("  Solid voxels: {}", solid_count);
    println!("  Empty voxels: {}", empty_count);
    println!("  Total voxels: {}", SIZE * SIZE * SIZE);
    println!(
        "  Percentage solid: {:.1}%",
        (solid_count as f32 / (SIZE * SIZE * SIZE) as f32) * 100.0
    );

    // Print a slice through the middle (z=4)
    println!("\nMiddle slice (z=4, Y goes down, X goes right):");
    println!("    0 1 2 3 4 5 6 7");
    for y in 0..SIZE {
        print!("  {}: ", y);
        for x in 0..SIZE {
            let idx = x + y * SIZE + 4 * SIZE * SIZE;
            if voxel_data[idx] != 0 {
                print!("█ ");
            } else {
                print!("· ");
            }
        }
        println!();
    }

    // Expected: octants 3 and 7 should be empty
    // Octant 3 is (x:0-3, y:4-7, z:4-7)
    // Octant 7 is (x:4-7, y:4-7, z:4-7)
    println!("\nExpected pattern:");
    println!("  Octants 0,1,2,4,5,6 should be SOLID (█)");
    println!("  Octants 3,7 should be EMPTY (·)");
    println!("  Octant 3: x:0-3, y:4-7, z:4-7");
    println!("  Octant 7: x:4-7, y:4-7, z:4-7");

    assert!(solid_count > 0, "No solid voxels found!");
    assert!(empty_count > 0, "No empty voxels found!");

    println!("\n✓ Texture data generation verified");
    println!("========================================\n");
}

#[test]
fn test_specific_octants() {
    println!("\n========================================");
    println!("OCTANT SAMPLING TEST");
    println!("========================================\n");

    let cube = create_octa_cube();

    // Test all 8 octants (octant indexing: x*4 + y*2 + z)
    let octant_tests = [
        (0, Vec3::new(0.125, 0.125, 0.125), 1), // Octant 0: Red
        (1, Vec3::new(0.125, 0.125, 0.625), 5), // Octant 1: White
        (2, Vec3::new(0.125, 0.625, 0.125), 2), // Octant 2: Green
        (3, Vec3::new(0.125, 0.625, 0.625), 0), // Octant 3: Empty
        (4, Vec3::new(0.625, 0.125, 0.125), 3), // Octant 4: Blue
        (5, Vec3::new(0.625, 0.125, 0.625), 5), // Octant 5: White
        (6, Vec3::new(0.625, 0.625, 0.125), 4), // Octant 6: Yellow
        (7, Vec3::new(0.625, 0.625, 0.625), 0), // Octant 7: Empty
    ];

    println!("Testing octant centers:");
    for (octant_id, pos, expected) in octant_tests {
        let value = sample_cube_at_position(&*cube, pos, 3);
        let status = if value == expected { "✓" } else { "✗" };
        println!(
            "  {} Octant {}: pos={:.3},{:.3},{:.3} -> value={} (expected {})",
            status, octant_id, pos.x, pos.y, pos.z, value, expected
        );
        assert_eq!(value, expected, "Octant {} has incorrect value", octant_id);
    }

    println!("\n✓ All octants sampled correctly");
    println!("========================================\n");
}
