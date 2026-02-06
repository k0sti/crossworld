//! Test raycast directly on octa cube

use renderer::scenes::create_octa_cube;

#[test]
fn test_raycast_at_max_boundary_negative_dir() {
    // Test to isolate the bug with rays at max boundary going inward
    let cube = create_octa_cube();

    // Ray at Y=0.999 (near max boundary) going down (negative Y)
    println!("\n=== Test: Ray at max Y boundary with negative direction ===");
    let pos = glam::Vec3::new(0.25, 0.999, 0.25); // In solid octant 2
    let dir = glam::Vec3::new(0.0, -1.0, 0.0); // Going down

    println!("Position: {:?} (octant 2, which is solid)", pos);
    println!("Direction: {:?} (negative Y)", dir);

    let hit = cube::raycast(&cube, pos, dir, None);

    match &hit {
        Some(hit) => {
            println!("✓ HIT at {:?} with value {}", hit.pos, hit.value);
        }
        None => {
            println!("✗ MISS - This reveals the bug!");
        }
    }

    // This should hit, but it might fail due to the bit calculation bug
    // If it fails, we need to fix the raycast algorithm or work around it
}

#[test]
fn test_raycast_octa_cube_from_boundary() {
    let cube = create_octa_cube();

    println!("=== Testing Raycast on Octa Cube ===");
    println!("Octa cube structure: 2x2x2 with solid octants at 0,1,2,4,5,6 and empty at 3,7");

    // Test 1: Ray from boundary (Z=0) going up through solid octants
    println!("\n--- Test 1: From boundary into solid octants ---");
    let ray_origin = glam::Vec3::new(0.25, 0.25, 0.0); // Lower left quadrant, at boundary
    let ray_dir = glam::Vec3::new(0.0, 0.0, 1.0).normalize();

    println!(
        "Origin: {:?} (should be in octant 0, which is solid)",
        ray_origin
    );
    println!("Direction: {:?}", ray_dir);

    let hit = cube::raycast(&cube, ray_origin, ray_dir, None);

    match &hit {
        Some(hit) => {
            println!("✓ HIT!");
            println!("  Position: {:?}", hit.pos);
            println!("  Normal: {:?}", hit.normal.as_vec3());
            println!("  Value: {}", hit.value);
        }
        None => {
            println!("✗ MISS - Expected to hit solid octant 0!");
        }
    }

    assert!(hit.is_some(), "Should hit solid octant 0 from boundary");

    // Test 2: Ray from inside cube through solid region
    println!("\n--- Test 2: From inside solid octant ---");
    let ray_origin2 = glam::Vec3::new(0.25, 0.25, 0.25);
    let ray_dir2 = glam::Vec3::new(0.0, 0.0, 1.0).normalize();

    println!("Origin: {:?} (inside solid octant 0)", ray_origin2);
    println!("Direction: {:?}", ray_dir2);

    let hit2 = cube::raycast(&cube, ray_origin2, ray_dir2, None);

    match &hit2 {
        Some(hit) => {
            println!("✓ HIT!");
            println!("  Position: {:?}", hit.pos);
            println!("  Normal: {:?}", hit.normal.as_vec3());
            println!("  Value: {}", hit.value);
        }
        None => {
            println!("✗ MISS - Expected to hit when starting inside solid!");
        }
    }

    // Test 3: Ray into empty octant (should miss)
    println!("\n--- Test 3: Into empty octant 3 ---");
    let ray_origin3 = glam::Vec3::new(0.75, 0.75, 0.0); // Upper right quadrant
    let ray_dir3 = glam::Vec3::new(0.0, 0.0, 1.0).normalize();

    println!(
        "Origin: {:?} (should be in octant 3, which is empty)",
        ray_origin3
    );
    println!("Direction: {:?}", ray_dir3);

    let hit3 = cube::raycast(&cube, ray_origin3, ray_dir3, None);

    match &hit3 {
        Some(hit) => {
            println!("  Hit at {:?} with value {}", hit.pos, hit.value);
        }
        None => {
            println!("✓ MISS - Correctly missed empty octant 3");
        }
    }

    // Test 4: Ray that should traverse from octant 0 to octant 1
    println!("\n--- Test 4: Traverse from octant 0 to octant 1 ---");
    let ray_origin4 = glam::Vec3::new(0.25, 0.25, 0.4);
    let ray_dir4 = glam::Vec3::new(0.0, 0.0, 1.0).normalize();

    println!("Origin: {:?} (inside solid octant 0, z < 0.5)", ray_origin4);
    println!("Direction: {:?} (toward octant 1, also solid)", ray_dir4);

    let hit4 = cube::raycast(&cube, ray_origin4, ray_dir4, None);

    match &hit4 {
        Some(hit) => {
            println!("✓ HIT!");
            println!("  Position: {:?}", hit.pos);
            println!("  Value: {}", hit.value);
        }
        None => {
            println!("✗ MISS - Expected to hit solid voxel!");
        }
    }
}
