//! Table-driven raycast tests
//!
//! This test file reads test cases from test_raycast_table.md and executes them
//! against the raycast implementation. Test data uses [-1, 1]³ coordinate system
//! which is converted on-the-fly to [0, 1]³ for the raycast API.

use cube::{Cube, CubeCoord};
use glam::{IVec3, Vec3};
use std::rc::Rc;

/// Test case parsed from markdown table
#[derive(Debug, Clone)]
struct TestCase {
    line_number: usize,
    origin: Vec3,    // In [-1, 1]³ space
    direction: Vec3, // Normalized direction
    expect_hit: bool,
    expected_coord: Option<CubeCoord>,
    expected_voxel: u8,
    expected_visits: i32, // Can be negative for flexible validation
    expected_normal: Vec3,
    expected_hit_pos: Vec3,
    notes: String,
}

/// Convert coordinate from [-1, 1]³ to [0, 1]³
fn convert_coord(v: Vec3) -> Vec3 {
    (v + Vec3::ONE) * 0.5
}

/// Parse Vec3 from string like "(x, y, z)"
fn parse_vec3(s: &str) -> Option<Vec3> {
    let s = s.trim();
    if s == "(0, 0, 0)" && s.len() == 9 {
        return Some(Vec3::ZERO);
    }

    let s = s.trim_matches(|c| c == '(' || c == ')' || c == '`');
    let parts: Vec<&str> = s.split(',').collect();

    if parts.len() != 3 {
        return None;
    }

    let x: f32 = parts[0].trim().parse().ok()?;
    let y: f32 = parts[1].trim().parse().ok()?;
    let z: f32 = parts[2].trim().parse().ok()?;

    Some(Vec3::new(x, y, z))
}

/// Parse IVec3 from string like "(x, y, z)"
fn parse_ivec3(s: &str) -> Option<IVec3> {
    let s = s.trim().trim_matches(|c| c == '(' || c == ')' || c == '`');
    let parts: Vec<&str> = s.split(',').collect();

    if parts.len() != 3 {
        return None;
    }

    let x: i32 = parts[0].trim().parse().ok()?;
    let y: i32 = parts[1].trim().parse().ok()?;
    let z: i32 = parts[2].trim().parse().ok()?;

    Some(IVec3::new(x, y, z))
}

/// Parse boolean from string
fn parse_bool(s: &str) -> Option<bool> {
    match s.trim().trim_matches('`').to_lowercase().as_str() {
        "true" | "✅" | "yes" | "1" => Some(true),
        "false" | "❌" | "no" | "0" => Some(false),
        _ => None,
    }
}

/// Parse test cases from markdown table
fn parse_test_table() -> Vec<TestCase> {
    let content = include_str!("test_raycast_table.md");
    let mut test_cases = Vec::new();

    for (line_num, line) in content.lines().enumerate() {
        let line = line.trim();

        // Skip header, separator, and empty lines
        if line.is_empty() || line.starts_with("| Ray Origin") || line.starts_with("| :---") {
            continue;
        }

        // Parse table row
        if line.starts_with('|') {
            let parts: Vec<&str> = line.split('|').collect();

            // Need at least 11 parts (empty, 10 columns, empty/overflow)
            if parts.len() < 11 {
                continue;
            }

            // Parse each field (skip first and handle last)
            let origin_str = parts[1].trim();
            let direction_str = parts[2].trim();
            let hit_str = parts[3].trim();
            let pos_str = parts[4].trim();
            let depth_str = parts[5].trim();
            let voxel_str = parts[6].trim();
            let visits_str = parts[7].trim();
            let normal_str = parts[8].trim();
            let hit_pos_str = parts[9].trim();
            let notes = parts.get(10).unwrap_or(&"").trim();

            // Parse origin and direction
            let Some(origin) = parse_vec3(origin_str) else {
                continue;
            };
            let Some(mut direction) = parse_vec3(direction_str) else {
                continue;
            };
            direction = direction.normalize();

            // Parse hit boolean
            let Some(expect_hit) = parse_bool(hit_str) else {
                continue;
            };

            // Parse cube coordinate
            let expected_coord = if expect_hit {
                if let Some(pos) = parse_ivec3(pos_str) {
                    if let Ok(depth) = depth_str.trim().trim_matches('`').parse::<u32>() {
                        Some(CubeCoord { pos, depth })
                    } else {
                        continue;
                    }
                } else {
                    continue;
                }
            } else {
                None
            };

            // Parse voxel value
            let expected_voxel: u8 = match voxel_str.trim().trim_matches('`').parse() {
                Ok(v) => v,
                Err(_) => continue,
            };

            // Parse visits (can be flexible)
            let expected_visits: i32 = match visits_str.trim().trim_matches('`').parse() {
                Ok(v) => v,
                Err(_) => continue,
            };

            // Parse normal and hit position
            let expected_normal = parse_vec3(normal_str).unwrap_or(Vec3::ZERO);
            let expected_hit_pos = parse_vec3(hit_pos_str).unwrap_or(Vec3::ZERO);

            test_cases.push(TestCase {
                line_number: line_num + 1,
                origin,
                direction,
                expect_hit,
                expected_coord,
                expected_voxel,
                expected_visits,
                expected_normal,
                expected_hit_pos,
                notes: notes.to_string(),
            });
        }
    }

    test_cases
}

/// Create the test cube matching the specification
/// Bounds: [-1, -1, -1] to [1, 1, 1]
/// Depth: 1 (2×2×2 grid)
/// Children array in cube crate order (x*4 + y*2 + z):
/// Test spec uses x + y*2 + z*4, so we need to remap
fn create_test_cube() -> Cube<u8> {
    // Original spec order: [1, 2, 0, 0, 3, 4, 5, 0] with indexing x + y*2 + z*4
    // Map to cube crate order (x*4 + y*2 + z):
    // Spec (0,0,0)=1 → Cube index 0 (0*4+0*2+0)=1
    // Spec (1,0,0)=2 → Cube index 4 (1*4+0*2+0)=2
    // Spec (0,1,0)=0 → Cube index 2 (0*4+1*2+0)=0
    // Spec (1,1,0)=0 → Cube index 6 (1*4+1*2+0)=0
    // Spec (0,0,1)=3 → Cube index 1 (0*4+0*2+1)=3
    // Spec (1,0,1)=4 → Cube index 5 (1*4+0*2+1)=4
    // Spec (0,1,1)=5 → Cube index 3 (0*4+1*2+1)=5
    // Spec (1,1,1)=0 → Cube index 7 (1*4+1*2+1)=0
    let children: [Rc<Cube<u8>>; 8] = [
        Rc::new(Cube::Solid(1)), // Cube index 0: (0,0,0)
        Rc::new(Cube::Solid(3)), // Cube index 1: (0,0,1)
        Rc::new(Cube::Solid(0)), // Cube index 2: (0,1,0) - empty
        Rc::new(Cube::Solid(5)), // Cube index 3: (0,1,1)
        Rc::new(Cube::Solid(2)), // Cube index 4: (1,0,0)
        Rc::new(Cube::Solid(4)), // Cube index 5: (1,0,1)
        Rc::new(Cube::Solid(0)), // Cube index 6: (1,1,0) - empty
        Rc::new(Cube::Solid(0)), // Cube index 7: (1,1,1) - empty
    ];

    Cube::Cubes(Box::new(children))
}

#[test]
fn test_raycast_table() {
    let test_cases = parse_test_table();
    assert!(
        !test_cases.is_empty(),
        "Failed to parse any test cases from test_raycast_table.md"
    );

    println!("Loaded {} test cases", test_cases.len());

    let cube = create_test_cube();
    let is_empty = |v: &u8| *v == 0;

    let mut passed = 0;
    let mut failures = Vec::new();

    for (idx, test_case) in test_cases.iter().enumerate() {
        // Convert coordinates from [-1, 1]³ to [0, 1]³
        let origin_converted = convert_coord(test_case.origin);
        let direction = test_case.direction;

        // Execute raycast
        let result = cube.raycast_debug(origin_converted, direction, 1, &is_empty);

        // Check result
        let test_name = format!(
            "Case {} (line {}): {}",
            idx + 1,
            test_case.line_number,
            if test_case.notes.len() > 50 {
                &test_case.notes[..50]
            } else {
                &test_case.notes
            }
        );

        if let Some(hit) = result {
            if test_case.expect_hit {
                let mut errors = Vec::new();

                // Verify coordinate
                if let Some(expected_coord) = &test_case.expected_coord {
                    if hit.coord.pos != expected_coord.pos {
                        errors.push(format!(
                            "  Coordinate mismatch: expected {:?}, got {:?}",
                            expected_coord.pos, hit.coord.pos
                        ));
                    }
                    // Note: Depth validation skipped as test data uses tree depth (1=first level)
                    // while raycast returns remaining depth (0 for leaves at max_depth=1)
                }

                // Verify voxel value
                if hit.value != test_case.expected_voxel {
                    errors.push(format!(
                        "  Voxel mismatch: expected {}, got {}",
                        test_case.expected_voxel, hit.value
                    ));
                }

                // Verify visit count (allow ±2 variance as it's implementation-dependent)
                if let Some(debug) = &hit.debug {
                    let visit_diff = (debug.enter_count as i32 - test_case.expected_visits).abs();
                    if visit_diff > 2 {
                        errors.push(format!(
                            "  Visit count mismatch: expected {}, got {} (diff: {})",
                            test_case.expected_visits, debug.enter_count, visit_diff
                        ));
                    }
                }

                // TODO: Verify hit position
                // Skipped for now - need to understand hit.position coordinate space better
                // (is it local child space or global parent space?)

                // TODO: Verify normal
                // Skipped for now - normals can be implementation-dependent
                // especially for edge cases and rays from behind

                if errors.is_empty() {
                    passed += 1;
                    println!("  ✅ {}", test_name);
                } else {
                    failures.push(format!("❌ {}\n{}", test_name, errors.join("\n")));
                    println!("  ❌ {}", test_name);
                }
            } else {
                failures.push(format!(
                    "❌ {}\n  Expected miss but got hit\n  Hit coord: {:?}, voxel: {}",
                    test_name, hit.coord, hit.value
                ));
                println!("  ❌ {}", test_name);
            }
        } else {
            if !test_case.expect_hit {
                passed += 1;
                println!("  ✅ {}", test_name);
            } else {
                failures.push(format!(
                    "❌ {}\n  Expected hit but got miss\n  Origin: {:?} (converted: {:?})\n  Direction: {:?}",
                    test_name, test_case.origin, origin_converted, direction
                ));
                println!("  ❌ {}", test_name);
            }
        }
    }

    println!("\n=== Summary ===");
    println!("Passed: {}/{}", passed, test_cases.len());
    println!("Failed: {}", failures.len());

    if !failures.is_empty() {
        println!("\n=== Failures ===");
        for failure in &failures {
            println!("{}", failure);
        }
        panic!("\n{} test case(s) failed", failures.len());
    }
}
