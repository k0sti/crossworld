//! Test to understand which positions cause raycast failures

use renderer::scenes::create_octa_cube;

#[test]
fn test_raycast_at_various_positions() {
    let cube = create_octa_cube();

    println!("\n=== Testing Raycast at Various Positions ===");
    println!("Octa cube: solid at octants 0,1,2,4,5,6; empty at octants 3,7\n");

    // Test grid of positions
    let test_positions = vec![
        // Center of each octant
        (0.25, 0.25, 0.25, 0, "center of octant 0 (solid)"),
        (0.25, 0.25, 0.75, 1, "center of octant 1 (solid)"),
        (0.25, 0.75, 0.25, 2, "center of octant 2 (solid)"),
        (0.25, 0.75, 0.75, 3, "center of octant 3 (empty)"),
        (0.75, 0.25, 0.25, 4, "center of octant 4 (solid)"),
        (0.75, 0.25, 0.75, 5, "center of octant 5 (solid)"),
        (0.75, 0.75, 0.25, 6, "center of octant 6 (solid)"),
        (0.75, 0.75, 0.75, 7, "center of octant 7 (empty)"),
        // Near boundaries
        (0.49, 0.49, 0.49, 0, "near center from octant 0"),
        (0.51, 0.49, 0.49, 4, "near center from octant 4"),
        (0.49, 0.51, 0.49, 2, "near center from octant 2"),
        (0.49, 0.49, 0.51, 1, "near center from octant 1"),
        // At clamped boundaries
        (0.01, 0.25, 0.25, 0, "min X boundary"),
        (0.99, 0.25, 0.25, 4, "max X boundary"),
        (0.25, 0.01, 0.25, 0, "min Y boundary"),
        (0.25, 0.99, 0.25, 2, "max Y boundary"),
        (0.25, 0.25, 0.01, 0, "min Z boundary"),
        (0.25, 0.25, 0.99, 1, "max Z boundary"),
    ];

    // Test with various ray directions
    let directions = vec![
        (0.0, 0.0, 1.0, "+Z"),
        (0.0, 0.0, -1.0, "-Z"),
        (1.0, 0.0, 0.0, "+X"),
        (-1.0, 0.0, 0.0, "-X"),
        (0.0, 1.0, 0.0, "+Y"),
        (0.0, -1.0, 0.0, "-Y"),
    ];

    let mut total_tests = 0;
    let mut passed_tests = 0;
    let mut failed_tests = 0;

    for (x, y, z, expected_octant, desc) in &test_positions {
        let pos = glam::Vec3::new(*x, *y, *z);

        for (dx, dy, dz, dir_name) in &directions {
            let dir = glam::Vec3::new(*dx, *dy, *dz).normalize();
            let result = cube::raycast(&cube, pos, dir, None);

            total_tests += 1;

            // For positions in solid octants, we should hit
            let should_hit = ![3, 7].contains(expected_octant);

            match (&result, should_hit) {
                (Some(_), true) => {
                    passed_tests += 1;
                }
                (None, false) => {
                    passed_tests += 1; // Correctly missed empty octant
                }
                (Some(_), false) => {
                    println!("  UNEXPECTED HIT at {} ({}), dir {}", pos, desc, dir_name);
                    passed_tests += 1; // Still a hit, just unexpected
                }
                (None, true) => {
                    println!(
                        "  ✗ MISS at {} ({}), dir {} - EXPECTED HIT",
                        pos, desc, dir_name
                    );
                    failed_tests += 1;
                }
            }
        }
    }

    println!("\n=== Summary ===");
    println!("Total tests: {}", total_tests);
    println!(
        "Passed: {} ({:.1}%)",
        passed_tests,
        (passed_tests as f32 / total_tests as f32) * 100.0
    );
    println!(
        "Failed: {} ({:.1}%)",
        failed_tests,
        (failed_tests as f32 / total_tests as f32) * 100.0
    );

    if failed_tests > 0 {
        println!(
            "\n⚠ Some raycasts failed - positions near boundaries or with certain ray directions are problematic"
        );
    } else {
        println!("\n✓ All raycasts succeeded!");
    }
}
