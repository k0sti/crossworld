//! Raycast Test Report Generator
//!
//! Generates a comprehensive test report table comparing raycast behavior
//! across CPU, GL, and GPU tracers with ANSI colored output.

use cube::Cube;
use glam::Vec3;
use std::fmt;

// ANSI color codes
const RESET: &str = "\x1b[0m";
const BOLD: &str = "\x1b[1m";
const GREEN: &str = "\x1b[32m";
const RED: &str = "\x1b[31m";
const YELLOW: &str = "\x1b[33m";
const BLUE: &str = "\x1b[34m";
const CYAN: &str = "\x1b[36m";
const DIM: &str = "\x1b[2m";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TestResult {
    Pass,
    Fail,
    Skip,
    NotImplemented,
}

impl fmt::Display for TestResult {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TestResult::Pass => write!(f, "{}✓ PASS{}", GREEN, RESET),
            TestResult::Fail => write!(f, "{}✗ FAIL{}", RED, RESET),
            TestResult::Skip => write!(f, "{}○ SKIP{}", YELLOW, RESET),
            TestResult::NotImplemented => write!(f, "{}− N/A{}", DIM, RESET),
        }
    }
}

#[derive(Debug, Clone)]
struct TestCase {
    name: String,
    category: String,
    pos: Vec3,
    dir: Vec3,
    should_hit: bool,
    expected_value: Option<i32>,
    expected_enter_count_min: u32,
    expected_enter_count_max: u32,
}

struct TestReport {
    test_name: String,
    cpu_result: TestResult,
    gl_result: TestResult,
    gpu_result: TestResult,
    details: String,
}

fn run_cube_test(cube: &Cube<i32>, test: &TestCase) -> (TestResult, String) {
    let is_empty = |v: &i32| *v == 0;
    let hit = cube.raycast_debug(test.pos, test.dir, 3, &is_empty);

    match (hit, test.should_hit) {
        (Some(hit_result), true) => {
            // Expected hit and got hit
            if let Some(expected_val) = test.expected_value {
                if hit_result.value != expected_val {
                    return (
                        TestResult::Fail,
                        format!(
                            "Value mismatch: got {}, expected {}",
                            hit_result.value, expected_val
                        ),
                    );
                }
            }

            // Check debug state
            if let Some(debug) = hit_result.debug {
                if debug.enter_count < test.expected_enter_count_min
                    || debug.enter_count > test.expected_enter_count_max
                {
                    return (
                        TestResult::Fail,
                        format!(
                            "Enter count {} outside range [{}, {}]",
                            debug.enter_count,
                            test.expected_enter_count_min,
                            test.expected_enter_count_max
                        ),
                    );
                }
                (
                    TestResult::Pass,
                    format!(
                        "Hit at ({:.2}, {:.2}, {:.2}), enters: {}",
                        hit_result.position.x,
                        hit_result.position.y,
                        hit_result.position.z,
                        debug.enter_count
                    ),
                )
            } else {
                (TestResult::Pass, "Hit (no debug)".to_string())
            }
        }
        (None, false) => {
            // Expected miss and got miss
            (TestResult::Pass, "Miss as expected".to_string())
        }
        (Some(_), false) => (
            TestResult::Fail,
            "Unexpected hit (should miss)".to_string(),
        ),
        (None, true) => (TestResult::Fail, "Unexpected miss (should hit)".to_string()),
    }
}

fn create_test_cases() -> Vec<TestCase> {
    vec![
        // Axis-aligned rays
        TestCase {
            name: "Axis +X".to_string(),
            category: "Axis-Aligned".to_string(),
            pos: Vec3::new(0.0, 0.5, 0.5),
            dir: Vec3::new(1.0, 0.0, 0.0),
            should_hit: true,
            expected_value: Some(1),
            expected_enter_count_min: 1,
            expected_enter_count_max: 1,
        },
        TestCase {
            name: "Axis -X".to_string(),
            category: "Axis-Aligned".to_string(),
            pos: Vec3::new(1.0, 0.5, 0.5),
            dir: Vec3::new(-1.0, 0.0, 0.0),
            should_hit: true,
            expected_value: Some(1),
            expected_enter_count_min: 1,
            expected_enter_count_max: 1,
        },
        TestCase {
            name: "Axis +Y".to_string(),
            category: "Axis-Aligned".to_string(),
            pos: Vec3::new(0.5, 0.0, 0.5),
            dir: Vec3::new(0.0, 1.0, 0.0),
            should_hit: true,
            expected_value: Some(1),
            expected_enter_count_min: 1,
            expected_enter_count_max: 1,
        },
        TestCase {
            name: "Axis -Y".to_string(),
            category: "Axis-Aligned".to_string(),
            pos: Vec3::new(0.5, 1.0, 0.5),
            dir: Vec3::new(0.0, -1.0, 0.0),
            should_hit: true,
            expected_value: Some(1),
            expected_enter_count_min: 1,
            expected_enter_count_max: 1,
        },
        TestCase {
            name: "Axis +Z".to_string(),
            category: "Axis-Aligned".to_string(),
            pos: Vec3::new(0.5, 0.5, 0.0),
            dir: Vec3::new(0.0, 0.0, 1.0),
            should_hit: true,
            expected_value: Some(1),
            expected_enter_count_min: 1,
            expected_enter_count_max: 1,
        },
        TestCase {
            name: "Axis -Z".to_string(),
            category: "Axis-Aligned".to_string(),
            pos: Vec3::new(0.5, 0.5, 1.0),
            dir: Vec3::new(0.0, 0.0, -1.0),
            should_hit: true,
            expected_value: Some(1),
            expected_enter_count_min: 1,
            expected_enter_count_max: 1,
        },
        // Diagonal rays
        TestCase {
            name: "Diagonal (+++).to_string()".to_string(),
            category: "Diagonal".to_string(),
            pos: Vec3::new(0.0, 0.0, 0.0),
            dir: Vec3::new(1.0, 1.0, 1.0).normalize(),
            should_hit: true,
            expected_value: Some(1),
            expected_enter_count_min: 1,
            expected_enter_count_max: 1,
        },
        TestCase {
            name: "Diagonal (-++)".to_string(),
            category: "Diagonal".to_string(),
            pos: Vec3::new(1.0, 0.0, 0.0),
            dir: Vec3::new(-1.0, 1.0, 1.0).normalize(),
            should_hit: true,
            expected_value: Some(1),
            expected_enter_count_min: 1,
            expected_enter_count_max: 1,
        },
        TestCase {
            name: "Diagonal (+-+)".to_string(),
            category: "Diagonal".to_string(),
            pos: Vec3::new(0.0, 1.0, 0.0),
            dir: Vec3::new(1.0, -1.0, 1.0).normalize(),
            should_hit: true,
            expected_value: Some(1),
            expected_enter_count_min: 1,
            expected_enter_count_max: 1,
        },
        TestCase {
            name: "Diagonal (++-)".to_string(),
            category: "Diagonal".to_string(),
            pos: Vec3::new(0.0, 0.0, 1.0),
            dir: Vec3::new(1.0, 1.0, -1.0).normalize(),
            should_hit: true,
            expected_value: Some(1),
            expected_enter_count_min: 1,
            expected_enter_count_max: 1,
        },
        // Miss cases
        TestCase {
            name: "Miss +X outside".to_string(),
            category: "Boundary Miss".to_string(),
            pos: Vec3::new(2.0, 0.5, 0.5),
            dir: Vec3::new(1.0, 0.0, 0.0),
            should_hit: false,
            expected_value: None,
            expected_enter_count_min: 0,
            expected_enter_count_max: 0,
        },
        TestCase {
            name: "Miss -Y outside".to_string(),
            category: "Boundary Miss".to_string(),
            pos: Vec3::new(0.5, -1.0, 0.5),
            dir: Vec3::new(0.0, -1.0, 0.0),
            should_hit: false,
            expected_value: None,
            expected_enter_count_min: 0,
            expected_enter_count_max: 0,
        },
        TestCase {
            name: "Miss +Z outside".to_string(),
            category: "Boundary Miss".to_string(),
            pos: Vec3::new(0.5, 0.5, 2.0),
            dir: Vec3::new(0.0, 0.0, 1.0),
            should_hit: false,
            expected_value: None,
            expected_enter_count_min: 0,
            expected_enter_count_max: 0,
        },
        // Edge cases
        TestCase {
            name: "Center hit".to_string(),
            category: "Edge Cases".to_string(),
            pos: Vec3::new(0.5, 0.5, 0.0),
            dir: Vec3::new(0.0, 0.0, 1.0),
            should_hit: true,
            expected_value: Some(1),
            expected_enter_count_min: 1,
            expected_enter_count_max: 1,
        },
        TestCase {
            name: "Corner entry".to_string(),
            category: "Edge Cases".to_string(),
            pos: Vec3::new(0.0, 0.0, 0.0),
            dir: Vec3::new(1.0, 1.0, 1.0).normalize(),
            should_hit: true,
            expected_value: Some(1),
            expected_enter_count_min: 1,
            expected_enter_count_max: 1,
        },
        TestCase {
            name: "Boundary entry".to_string(),
            category: "Edge Cases".to_string(),
            pos: Vec3::new(0.5, 0.5, 0.5),
            dir: Vec3::new(0.0, 0.0, 1.0),
            should_hit: true,
            expected_value: Some(1),
            expected_enter_count_min: 1,
            expected_enter_count_max: 1,
        },
    ]
}

fn print_header() {
    println!("\n{}{}", BOLD, CYAN);
    println!("╔══════════════════════════════════════════════════════════════════════════════╗");
    println!("║                      RAYCAST TEST REPORT - ALL TRACERS                       ║");
    println!("╚══════════════════════════════════════════════════════════════════════════════╝");
    println!("{}", RESET);
}

fn print_openspec_status() {
    println!("\n{}{}OpenSpec Change Status:{}", BOLD, BLUE, RESET);
    println!("{}┌────────────────────────────────────────────┬──────────────┐{}", DIM, RESET);
    println!("{}│{} Change                                    {}│{} Status       {}│{}", DIM, RESET, DIM, RESET, DIM, RESET);
    println!("{}├────────────────────────────────────────────┼──────────────┤{}", DIM, RESET);

    let changes = vec![
        ("reimplement-raycast", "89/97 tasks", YELLOW),
        ("implement-gpu-raytracer", "46/51 tasks", YELLOW),
        ("refactor-gl-hierarchical-traversal", "0/61 tasks", RED),
        ("standardize-material-system", "55/84 tasks", YELLOW),
        ("integrate-cube-raycast", "✓ Complete", GREEN),
        ("add-octa-cube-rendering", "✓ Complete", GREEN),
    ];

    for (name, status, color) in changes {
        println!(
            "{}│{} {:<42} {}│{} {}{:>12}{} {}│{}",
            DIM, RESET, name, DIM, RESET, color, status, RESET, DIM, RESET
        );
    }

    println!("{}└────────────────────────────────────────────┴──────────────┘{}", DIM, RESET);
}

fn print_test_table(reports: &[TestReport]) {
    println!("\n{}{}Test Results by Category:{}", BOLD, BLUE, RESET);
    println!("{}┌─────────────────────────────┬──────────┬──────────┬──────────┬─────────────────{}", DIM, RESET);
    println!("{}│{} Test Case                  {}│{} CPU      {}│{} GL       {}│{} GPU      {}│{} Details",
        DIM, RESET, DIM, RESET, DIM, RESET, DIM, RESET, DIM, RESET);
    println!("{}├─────────────────────────────┼──────────┼──────────┼──────────┼─────────────────{}", DIM, RESET);

    let mut current_category = String::new();
    for report in reports {
        // Extract category from test name (format: "Category: Test")
        let parts: Vec<&str> = report.test_name.split(':').collect();
        let (category, name) = if parts.len() > 1 {
            (parts[0].trim(), parts[1].trim())
        } else {
            ("", report.test_name.as_str())
        };

        // Print category header if changed
        if category != current_category && !category.is_empty() {
            if !current_category.is_empty() {
                println!("{}├─────────────────────────────┼──────────┼──────────┼──────────┼─────────────────{}", DIM, RESET);
            }
            println!(
                "{}│{} {}{:<27}{} {}│          │          │          │                 {}",
                DIM, RESET, BOLD, category, RESET, DIM, RESET
            );
            current_category = category.to_string();
        }

        // Truncate details if too long
        let details = if report.details.len() > 17 {
            format!("{}...", &report.details[..14])
        } else {
            format!("{:<17}", report.details)
        };

        println!(
            "{}│{} {:<27} {}│{} {} {}│{} {} {}│{} {} {}│{} {}",
            DIM,
            RESET,
            name,
            DIM,
            RESET,
            report.cpu_result,
            DIM,
            RESET,
            report.gl_result,
            DIM,
            RESET,
            report.gpu_result,
            DIM,
            RESET,
            details
        );
    }

    println!("{}└─────────────────────────────┴──────────┴──────────┴──────────┴─────────────────{}", DIM, RESET);
}

fn print_summary(reports: &[TestReport]) {
    let cpu_pass = reports.iter().filter(|r| r.cpu_result == TestResult::Pass).count();
    let cpu_fail = reports.iter().filter(|r| r.cpu_result == TestResult::Fail).count();
    let gl_na = reports.iter().filter(|r| r.gl_result == TestResult::NotImplemented).count();
    let gpu_na = reports.iter().filter(|r| r.gpu_result == TestResult::NotImplemented).count();
    let total = reports.len();

    println!("\n{}{}Summary:", BOLD, BLUE);
    println!("{}┌──────────┬──────┬──────┬──────┐", DIM);
    println!("{}│{} Tracer   {}│{} Pass {}│{} Fail {}│{} N/A  {}│", DIM, RESET, DIM, RESET, DIM, RESET, DIM, RESET, DIM);
    println!("{}├──────────┼──────┼──────┼──────┤", DIM);

    // CPU row
    print!("{}│{} CPU      {}│ ", DIM, RESET, DIM);
    print!("{}{:>4}{} ", GREEN, cpu_pass, RESET);
    print!("{}│ ", DIM);
    print!("{}{:>4}{} ", RED, cpu_fail, RESET);
    print!("{}│ ", DIM);
    println!("{:>4} {}│", 0, DIM);

    // GL row
    print!("{}│{} GL       {}│ ", DIM, RESET, DIM);
    print!("{:>4} ", 0);
    print!("{}│ ", DIM);
    print!("{:>4} ", 0);
    print!("{}│ ", DIM);
    print!("{}{:>4}{} ", YELLOW, gl_na, RESET);
    println!("{}│", DIM);

    // GPU row
    print!("{}│{} GPU      {}│ ", DIM, RESET, DIM);
    print!("{:>4} ", 0);
    print!("{}│ ", DIM);
    print!("{:>4} ", 0);
    print!("{}│ ", DIM);
    print!("{}{:>4}{} ", YELLOW, gpu_na, RESET);
    println!("{}│", DIM);

    println!("{}└──────────┴──────┴──────┴──────┘{}", DIM, RESET);

    println!(
        "\n{}Total tests: {}{}{} | CPU: {}{}/{}{} passed",
        BOLD, CYAN, total, RESET, GREEN, cpu_pass, total, RESET
    );
}

#[test]
fn test_raycast_report() {
    let cube = Cube::Solid(1i32);
    let test_cases = create_test_cases();
    let mut reports = Vec::new();

    for test in &test_cases {
        let (cpu_result, details) = run_cube_test(&cube, test);

        reports.push(TestReport {
            test_name: format!("{}: {}", test.category, test.name),
            cpu_result,
            gl_result: TestResult::NotImplemented,
            gpu_result: TestResult::NotImplemented,
            details,
        });
    }

    print_header();
    print_openspec_status();
    print_test_table(&reports);
    print_summary(&reports);

    println!("\n{}{}Note:{} GL and GPU tracers require OpenGL context for testing.", BOLD, YELLOW, RESET);
    println!("{}      Run renderer integration tests for full coverage.{}\n", DIM, RESET);
}
