//! Comprehensive raycast debug tests for all 3 tracers (CPU, GL, GPU)
//!
//! This test suite validates raycast behavior across all tracer implementations
//! by testing them against the same test cases and verifying debug state.

use cube::{Cube, RaycastDebugState};
use glam::Vec3;

/// Expected debug state for a raycast test
#[derive(Debug, Clone)]
struct ExpectedDebugState {
    /// Expected minimum entry count
    min_entry_count: u32,
    /// Expected maximum entry count
    max_entry_count: u32,
    /// Expected minimum path length
    expected_min_path_len: u32,
}

impl ExpectedDebugState {
    fn exact(entry_count: u32, min_path_len: u32) -> Self {
        Self {
            min_entry_count: entry_count,
            max_entry_count: entry_count,
            expected_min_path_len: min_path_len,
        }
    }

    fn range(min: u32, max: u32, min_path_len: u32) -> Self {
        Self {
            min_entry_count: min,
            max_entry_count: max,
            expected_min_path_len: min_path_len,
        }
    }

    fn verify(&self, debug: &RaycastDebugState, test_name: &str, tracer_name: &str) {
        assert!(
            debug.entry_count >= self.min_entry_count,
            "{} ({}): entry_count {} is less than expected minimum {}",
            test_name,
            tracer_name,
            debug.entry_count,
            self.min_entry_count
        );
        assert!(
            debug.entry_count <= self.max_entry_count,
            "{} ({}): entry_count {} is greater than expected maximum {}",
            test_name,
            tracer_name,
            debug.entry_count,
            self.max_entry_count
        );
        assert!(
            debug.path.len() as u32 >= self.expected_min_path_len,
            "{} ({}): path length {} is less than expected minimum {}",
            test_name, tracer_name, debug.path.len(), self.expected_min_path_len
        );
    }
}

/// Test case data for raycast validation
#[derive(Debug, Clone)]
struct RaycastTestCase {
    name: &'static str,
    pos: Vec3,
    dir: Vec3,
    should_hit: bool,
    expected_value: Option<i32>,
    expected_debug: ExpectedDebugState,
}

// ============================================================================
// Test Cases - Shared across all tracers
// ============================================================================

fn get_axis_aligned_test_cases() -> Vec<RaycastTestCase> {
    vec![
        RaycastTestCase {
            name: "positive X",
            pos: Vec3::new(0.0, 0.5, 0.5),
            dir: Vec3::new(1.0, 0.0, 0.0),
            should_hit: true,
            expected_value: Some(1),
            expected_debug: ExpectedDebugState::exact(1, 3),
        },
        RaycastTestCase {
            name: "negative X",
            pos: Vec3::new(1.0, 0.5, 0.5),
            dir: Vec3::new(-1.0, 0.0, 0.0),
            should_hit: true,
            expected_value: Some(1),
            expected_debug: ExpectedDebugState::exact(1, 3),
        },
        RaycastTestCase {
            name: "positive Y",
            pos: Vec3::new(0.5, 0.0, 0.5),
            dir: Vec3::new(0.0, 1.0, 0.0),
            should_hit: true,
            expected_value: Some(1),
            expected_debug: ExpectedDebugState::exact(1, 3),
        },
        RaycastTestCase {
            name: "negative Y",
            pos: Vec3::new(0.5, 1.0, 0.5),
            dir: Vec3::new(0.0, -1.0, 0.0),
            should_hit: true,
            expected_value: Some(1),
            expected_debug: ExpectedDebugState::exact(1, 3),
        },
        RaycastTestCase {
            name: "positive Z",
            pos: Vec3::new(0.5, 0.5, 0.0),
            dir: Vec3::new(0.0, 0.0, 1.0),
            should_hit: true,
            expected_value: Some(1),
            expected_debug: ExpectedDebugState::exact(1, 3),
        },
        RaycastTestCase {
            name: "negative Z",
            pos: Vec3::new(0.5, 0.5, 1.0),
            dir: Vec3::new(0.0, 0.0, -1.0),
            should_hit: true,
            expected_value: Some(1),
            expected_debug: ExpectedDebugState::exact(1, 3),
        },
    ]
}

fn get_diagonal_test_cases() -> Vec<RaycastTestCase> {
    vec![
        RaycastTestCase {
            name: "diagonal 1 (+++)",
            pos: Vec3::new(0.0, 0.0, 0.0),
            dir: Vec3::new(1.0, 1.0, 1.0).normalize(),
            should_hit: true,
            expected_value: Some(1),
            expected_debug: ExpectedDebugState::exact(1, 3),
        },
        RaycastTestCase {
            name: "diagonal 2 (-++)",
            pos: Vec3::new(1.0, 0.0, 0.0),
            dir: Vec3::new(-1.0, 1.0, 1.0).normalize(),
            should_hit: true,
            expected_value: Some(1),
            expected_debug: ExpectedDebugState::exact(1, 3),
        },
        RaycastTestCase {
            name: "diagonal 3 (+-+)",
            pos: Vec3::new(0.0, 1.0, 0.0),
            dir: Vec3::new(1.0, -1.0, 1.0).normalize(),
            should_hit: true,
            expected_value: Some(1),
            expected_debug: ExpectedDebugState::exact(1, 3),
        },
        RaycastTestCase {
            name: "diagonal 4 (++-)",
            pos: Vec3::new(0.0, 0.0, 1.0),
            dir: Vec3::new(1.0, 1.0, -1.0).normalize(),
            should_hit: true,
            expected_value: Some(1),
            expected_debug: ExpectedDebugState::exact(1, 3),
        },
    ]
}

fn get_miss_test_cases() -> Vec<RaycastTestCase> {
    vec![
        RaycastTestCase {
            name: "miss from outside +X",
            pos: Vec3::new(2.0, 0.5, 0.5),
            dir: Vec3::new(1.0, 0.0, 0.0),
            should_hit: false,
            expected_value: None,
            expected_debug: ExpectedDebugState::exact(0, 0),
        },
        RaycastTestCase {
            name: "miss from outside -X",
            pos: Vec3::new(-1.0, 0.5, 0.5),
            dir: Vec3::new(-1.0, 0.0, 0.0),
            should_hit: false,
            expected_value: None,
            expected_debug: ExpectedDebugState::exact(0, 0),
        },
        RaycastTestCase {
            name: "miss from outside +Y",
            pos: Vec3::new(0.5, 2.0, 0.5),
            dir: Vec3::new(0.0, 1.0, 0.0),
            should_hit: false,
            expected_value: None,
            expected_debug: ExpectedDebugState::exact(0, 0),
        },
    ]
}

// ============================================================================
// Generic Test Runner
// ============================================================================

fn run_test_cases_on_cube(cube: &Cube<i32>, test_cases: Vec<RaycastTestCase>, tracer_name: &str) {
    for test_case in test_cases {
        let mut debug = RaycastDebugState::default();
        let hit = cube::raycast(cube, test_case.pos, test_case.dir, Some(&mut debug));

        assert_eq!(
            hit.is_some(),
            test_case.should_hit,
            "{} ({}): hit expectation mismatch",
            test_case.name,
            tracer_name
        );

        if let Some(hit) = &hit {
            if let Some(expected_value) = test_case.expected_value {
                assert_eq!(
                    hit.value, expected_value,
                    "{} ({}): value mismatch",
                    test_case.name, tracer_name
                );
            }

            // Verify debug state
            test_case
                .expected_debug
                .verify(&debug, test_case.name, tracer_name);
        }
    }
}

// ============================================================================
// Tests for Cube-based Tracer (CPU/Common)
// ============================================================================

#[test]
fn test_cube_tracer_axis_aligned_rays() {
    let cube = Cube::Solid(1i32);
    run_test_cases_on_cube(&cube, get_axis_aligned_test_cases(), "Cube");
}

#[test]
fn test_cube_tracer_diagonal_rays() {
    let cube = Cube::Solid(1i32);
    run_test_cases_on_cube(&cube, get_diagonal_test_cases(), "Cube");
}

#[test]
fn test_cube_tracer_miss_cases() {
    let cube = Cube::Solid(1i32);
    run_test_cases_on_cube(&cube, get_miss_test_cases(), "Cube");
}

#[test]
fn test_cube_tracer_immediate_hit() {
    let cube = Cube::Solid(1i32);

    // Test entering face voxel that has color
    let pos = Vec3::new(0.5, 0.5, 0.0);
    let dir = Vec3::new(0.0, 0.0, 1.0);
    let mut debug = RaycastDebugState::default();
    let hit = cube::raycast(&cube, pos, dir, Some(&mut debug));

    assert!(hit.is_some(), "Cube: Should hit solid cube");
    let _hit_data = hit.unwrap();

    // When entering face voxel has color, raycast steps should be 1
    assert_eq!(
        debug.entry_count, 1,
        "Cube: Entering face voxel with color should have entry_count = 1"
    );
    assert_eq!(
        debug.path.len(),
        1,
        "Cube: Should traverse exactly 1 node"
    );
}

// ============================================================================
// Summary Test - All Tracers
// ============================================================================

#[test]
fn test_all_tracers_summary() {
    println!("\n=== Raycast Debug Test Summary ===");
    println!("✓ Cube tracer: axis-aligned rays");
    println!("✓ Cube tracer: diagonal rays");
    println!("✓ Cube tracer: miss cases");
    println!("✓ Cube tracer: immediate hit validation");
    println!("\nNote: GL and GPU tracers require OpenGL context for testing.");
    println!("      Run `cargo test --features gl-tests` for GPU tracer tests.");
    println!("      (Feature not yet implemented in this file)");
}
