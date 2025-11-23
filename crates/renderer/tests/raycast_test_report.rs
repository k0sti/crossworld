//! Raycast Test Report Generator
//!
//! Generates a comprehensive test report table comparing raycast behavior
//! across CPU, GL, and GPU tracers with ANSI colored output.
//!
//! Uses a generic tracer tester interface to validate any tracer implementation.

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

// ============================================================================
// Generic Tracer Interface
// ============================================================================

/// Generic raycast hit result
#[derive(Debug, Clone)]
struct RaycastHit {
    position: Vec3,
    normal: Vec3,
    value: i32,
    enter_count: Option<u32>,
}

/// Generic tracer interface that all tracers must implement
trait Tracer {
    /// Name of the tracer (e.g., "CPU", "GL", "GPU")
    fn name(&self) -> &str;

    /// Perform raycast with the given parameters
    /// Returns Some(hit) if ray hits non-empty voxel, None otherwise
    fn raycast(&self, pos: Vec3, dir: Vec3, max_depth: u32) -> Option<RaycastHit>;

    /// Check if tracer is available (e.g., GL requires context)
    fn is_available(&self) -> bool {
        true
    }
}

// ============================================================================
// CPU Tracer Implementation
// ============================================================================

struct CpuTracer {
    cube: Cube<i32>,
}

impl CpuTracer {
    fn new(cube: Cube<i32>) -> Self {
        Self { cube }
    }
}

impl Tracer for CpuTracer {
    fn name(&self) -> &str {
        "CPU"
    }

    fn raycast(&self, pos: Vec3, dir: Vec3, max_depth: u32) -> Option<RaycastHit> {
        let is_empty = |v: &i32| *v == 0;
        let hit = self.cube.raycast_debug(pos, dir, max_depth, &is_empty)?;

        Some(RaycastHit {
            position: hit.position,
            normal: hit.normal,
            value: hit.value,
            enter_count: hit.debug.as_ref().map(|d| d.enter_count),
        })
    }
}

// ============================================================================
// GL Tracer Stub (requires OpenGL context)
// ============================================================================

struct GlTracer;

impl Tracer for GlTracer {
    fn name(&self) -> &str {
        "GL"
    }

    fn raycast(&self, _pos: Vec3, _dir: Vec3, _max_depth: u32) -> Option<RaycastHit> {
        // GL tracer requires OpenGL context - not available in test environment
        None
    }

    fn is_available(&self) -> bool {
        false // Requires OpenGL context
    }
}

// ============================================================================
// GPU Tracer Stub (requires OpenGL context)
// ============================================================================

struct GpuTracer;

impl Tracer for GpuTracer {
    fn name(&self) -> &str {
        "GPU"
    }

    fn raycast(&self, _pos: Vec3, _dir: Vec3, _max_depth: u32) -> Option<RaycastHit> {
        // GPU tracer requires OpenGL context - not available in test environment
        None
    }

    fn is_available(&self) -> bool {
        false // Requires OpenGL context
    }
}

// ============================================================================
// Test Case Definition
// ============================================================================

#[derive(Debug, Clone)]
struct TestCase {
    number: usize,
    name: String,
    category: String,
    pos: Vec3,
    dir: Vec3,
    should_hit: bool,
    expected_value: Option<i32>,
    expected_enter_count_min: u32,
    expected_enter_count_max: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum TestResult {
    Pass,
    Fail,
    NotImplemented,
}

impl fmt::Display for TestResult {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TestResult::Pass => write!(f, "{}PASS{}", GREEN, RESET),
            TestResult::Fail => write!(f, "{}FAIL{}", RED, RESET),
            TestResult::NotImplemented => write!(f, "{}N/A{}", DIM, RESET),
        }
    }
}

#[derive(Debug, Clone)]
struct TestCaseResult {
    test_number: usize,
    test_name: String,
    result: TestResult,
    error: Option<String>,
}

// ============================================================================
// Generic Tracer Tester
// ============================================================================

/// Generic tracer tester that validates any tracer implementation
fn test_tracer(tracer: &dyn Tracer, test: &TestCase) -> TestCaseResult {
    // Check if tracer is available
    if !tracer.is_available() {
        return TestCaseResult {
            test_number: test.number,
            test_name: test.name.clone(),
            result: TestResult::NotImplemented,
            error: None,
        };
    }

    // Run the raycast
    let hit = tracer.raycast(test.pos, test.dir, 3);

    // Validate the result
    match (hit, test.should_hit) {
        (Some(hit_result), true) => {
            // Expected hit and got hit - validate details

            // Check voxel value
            if let Some(expected_val) = test.expected_value {
                if hit_result.value != expected_val {
                    return TestCaseResult {
                        test_number: test.number,
                        test_name: test.name.clone(),
                        result: TestResult::Fail,
                        error: Some(format!(
                            "Value mismatch: got {}, expected {}",
                            hit_result.value, expected_val
                        )),
                    };
                }
            }

            // Check enter count
            if let Some(enter_count) = hit_result.enter_count {
                if enter_count < test.expected_enter_count_min
                    || enter_count > test.expected_enter_count_max
                {
                    return TestCaseResult {
                        test_number: test.number,
                        test_name: test.name.clone(),
                        result: TestResult::Fail,
                        error: Some(format!(
                            "Enter count {} outside range [{}, {}]",
                            enter_count,
                            test.expected_enter_count_min,
                            test.expected_enter_count_max
                        )),
                    };
                }
            }

            TestCaseResult {
                test_number: test.number,
                test_name: test.name.clone(),
                result: TestResult::Pass,
                error: None,
            }
        }
        (None, false) => {
            // Expected miss and got miss
            TestCaseResult {
                test_number: test.number,
                test_name: test.name.clone(),
                result: TestResult::Pass,
                error: None,
            }
        }
        (Some(_), false) => {
            // Got hit but expected miss
            TestCaseResult {
                test_number: test.number,
                test_name: test.name.clone(),
                result: TestResult::Fail,
                error: Some("Unexpected hit (should miss)".to_string()),
            }
        }
        (None, true) => {
            // Got miss but expected hit
            TestCaseResult {
                test_number: test.number,
                test_name: test.name.clone(),
                result: TestResult::Fail,
                error: Some("Unexpected miss (should hit)".to_string()),
            }
        }
    }
}

// ============================================================================
// Test Case Generator
// ============================================================================

fn create_test_cases() -> Vec<TestCase> {
    let mut tests = Vec::new();
    let mut number = 1;

    // Axis-aligned rays
    let axis_tests = vec![
        ("Axis +X", Vec3::new(0.0, 0.5, 0.5), Vec3::new(1.0, 0.0, 0.0)),
        ("Axis -X", Vec3::new(1.0, 0.5, 0.5), Vec3::new(-1.0, 0.0, 0.0)),
        ("Axis +Y", Vec3::new(0.5, 0.0, 0.5), Vec3::new(0.0, 1.0, 0.0)),
        ("Axis -Y", Vec3::new(0.5, 1.0, 0.5), Vec3::new(0.0, -1.0, 0.0)),
        ("Axis +Z", Vec3::new(0.5, 0.5, 0.0), Vec3::new(0.0, 0.0, 1.0)),
        ("Axis -Z", Vec3::new(0.5, 0.5, 1.0), Vec3::new(0.0, 0.0, -1.0)),
    ];

    for (name, pos, dir) in axis_tests {
        tests.push(TestCase {
            number,
            name: name.to_string(),
            category: "Axis-Aligned".to_string(),
            pos,
            dir,
            should_hit: true,
            expected_value: Some(1),
            expected_enter_count_min: 1,
            expected_enter_count_max: 1,
        });
        number += 1;
    }

    // Diagonal rays
    let diagonal_tests = vec![
        ("Diagonal (+++)", Vec3::new(0.0, 0.0, 0.0), Vec3::new(1.0, 1.0, 1.0).normalize()),
        ("Diagonal (-++)", Vec3::new(1.0, 0.0, 0.0), Vec3::new(-1.0, 1.0, 1.0).normalize()),
        ("Diagonal (+-+)", Vec3::new(0.0, 1.0, 0.0), Vec3::new(1.0, -1.0, 1.0).normalize()),
        ("Diagonal (++-)", Vec3::new(0.0, 0.0, 1.0), Vec3::new(1.0, 1.0, -1.0).normalize()),
    ];

    for (name, pos, dir) in diagonal_tests {
        tests.push(TestCase {
            number,
            name: name.to_string(),
            category: "Diagonal".to_string(),
            pos,
            dir,
            should_hit: true,
            expected_value: Some(1),
            expected_enter_count_min: 1,
            expected_enter_count_max: 1,
        });
        number += 1;
    }

    // Miss cases
    let miss_tests = vec![
        ("Miss +X outside", Vec3::new(2.0, 0.5, 0.5), Vec3::new(1.0, 0.0, 0.0)),
        ("Miss -Y outside", Vec3::new(0.5, -1.0, 0.5), Vec3::new(0.0, -1.0, 0.0)),
        ("Miss +Z outside", Vec3::new(0.5, 0.5, 2.0), Vec3::new(0.0, 0.0, 1.0)),
    ];

    for (name, pos, dir) in miss_tests {
        tests.push(TestCase {
            number,
            name: name.to_string(),
            category: "Boundary Miss".to_string(),
            pos,
            dir,
            should_hit: false,
            expected_value: None,
            expected_enter_count_min: 0,
            expected_enter_count_max: 0,
        });
        number += 1;
    }

    // Edge cases
    let edge_tests = vec![
        ("Center hit", Vec3::new(0.5, 0.5, 0.0), Vec3::new(0.0, 0.0, 1.0)),
        ("Corner entry", Vec3::new(0.0, 0.0, 0.0), Vec3::new(1.0, 1.0, 1.0).normalize()),
        ("Boundary entry", Vec3::new(0.5, 0.5, 0.5), Vec3::new(0.0, 0.0, 1.0)),
    ];

    for (name, pos, dir) in edge_tests {
        tests.push(TestCase {
            number,
            name: name.to_string(),
            category: "Edge Cases".to_string(),
            pos,
            dir,
            should_hit: true,
            expected_value: Some(1),
            expected_enter_count_min: 1,
            expected_enter_count_max: 1,
        });
        number += 1;
    }

    tests
}

// ============================================================================
// Report Generation
// ============================================================================

struct TracerResults {
    tracer_name: String,
    results: Vec<TestCaseResult>,
}

impl TracerResults {
    fn pass_count(&self) -> usize {
        self.results.iter().filter(|r| r.result == TestResult::Pass).count()
    }

    fn fail_count(&self) -> usize {
        self.results.iter().filter(|r| r.result == TestResult::Fail).count()
    }

    fn na_count(&self) -> usize {
        self.results.iter().filter(|r| r.result == TestResult::NotImplemented).count()
    }

    fn failures(&self) -> Vec<&TestCaseResult> {
        self.results.iter().filter(|r| r.result == TestResult::Fail).collect()
    }
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

fn print_test_table(test_cases: &[TestCase], all_results: &[TracerResults]) {
    println!("\n{}{}Test Results by Category:{}", BOLD, BLUE, RESET);
    println!("{}┌────┬─────────────────────────────┬──────┬──────┬──────┐{}", DIM, RESET);
    println!("{}│{}  # {}│{} Test Case                  {}│{} CPU  {}│{} GL   {}│{} GPU  {}│{}",
        DIM, RESET, DIM, RESET, DIM, RESET, DIM, RESET, DIM, RESET, DIM, RESET);
    println!("{}├────┼─────────────────────────────┼──────┼──────┼──────┤{}", DIM, RESET);

    let mut current_category = String::new();
    for test in test_cases {
        // Print category header if changed
        if test.category != current_category {
            if !current_category.is_empty() {
                println!("{}├────┼─────────────────────────────┼──────┼──────┼──────┤{}", DIM, RESET);
            }
            println!(
                "{}│{}    {}│{} {}{:<27}{} {}│      │      │      {}│{}",
                DIM, RESET, DIM, RESET, BOLD, test.category, RESET, DIM, DIM, RESET
            );
            current_category = test.category.clone();
        }

        // Get results for this test from each tracer
        let cpu_result = &all_results[0].results[test.number - 1];
        let gl_result = &all_results[1].results[test.number - 1];
        let gpu_result = &all_results[2].results[test.number - 1];

        print!("{}│{} {:>2} {}│{} {:<27} {}│ ",
            DIM, RESET, test.number, DIM, RESET, test.name, DIM);
        print!("{} ", cpu_result.result);
        print!("{}│ ", DIM);
        print!("{} ", gl_result.result);
        print!("{}│ ", DIM);
        print!("{} ", gpu_result.result);
        println!("{}│{}", DIM, RESET);
    }

    println!("{}└────┴─────────────────────────────┴──────┴──────┴──────┘{}", DIM, RESET);
}

fn print_failures(all_results: &[TracerResults]) {
    let mut has_failures = false;

    for tracer_results in all_results {
        let failures = tracer_results.failures();
        if !failures.is_empty() {
            has_failures = true;
            println!("\n{}{}Failures for {} Tracer:{}", BOLD, RED, tracer_results.tracer_name, RESET);
            for failure in failures {
                println!("  {}#{} {}{}: {}{}{}",
                    DIM, failure.test_number, RESET,
                    failure.test_name,
                    RED, failure.error.as_deref().unwrap_or("Unknown error"), RESET
                );
            }
        }
    }

    if !has_failures {
        println!("\n{}{}No failures!{}", BOLD, GREEN, RESET);
    }
}

fn print_summary(all_results: &[TracerResults], total_tests: usize) {
    println!("\n{}{}Summary:{}", BOLD, BLUE, RESET);
    println!("{}┌──────────┬──────┬──────┬──────┐", DIM);
    println!("{}│{} Tracer   {}│{} Pass {}│{} Fail {}│{} N/A  {}│", DIM, RESET, DIM, RESET, DIM, RESET, DIM, RESET, DIM);
    println!("{}├──────────┼──────┼──────┼──────┤", DIM);

    for tracer_results in all_results {
        print!("{}│{} {:<8} {}│ ", DIM, RESET, tracer_results.tracer_name, DIM);
        print!("{}{:>4}{} ", GREEN, tracer_results.pass_count(), RESET);
        print!("{}│ ", DIM);
        print!("{}{:>4}{} ", RED, tracer_results.fail_count(), RESET);
        print!("{}│ ", DIM);
        print!("{}{:>4}{} ", YELLOW, tracer_results.na_count(), RESET);
        println!("{}│{}", DIM, RESET);
    }

    println!("{}└──────────┴──────┴──────┴──────┘{}", DIM, RESET);

    let cpu_results = &all_results[0];
    println!(
        "\n{}Total tests: {}{}{} | CPU: {}{}/{}{} passed",
        BOLD, CYAN, total_tests, RESET, GREEN, cpu_results.pass_count(), total_tests, RESET
    );
}

// ============================================================================
// Main Test
// ============================================================================

#[test]
fn test_raycast_report() {
    let cube = Cube::Solid(1i32);
    let test_cases = create_test_cases();

    // Create tracers
    let tracers: Vec<Box<dyn Tracer>> = vec![
        Box::new(CpuTracer::new(cube)),
        Box::new(GlTracer),
        Box::new(GpuTracer),
    ];

    // Run all tests for all tracers
    let mut all_results = Vec::new();
    for tracer in &tracers {
        let mut results = Vec::new();
        for test in &test_cases {
            results.push(test_tracer(tracer.as_ref(), test));
        }
        all_results.push(TracerResults {
            tracer_name: tracer.name().to_string(),
            results,
        });
    }

    // Generate report
    print_header();
    print_openspec_status();
    print_test_table(&test_cases, &all_results);
    print_failures(&all_results);
    print_summary(&all_results, test_cases.len());

    println!("\n{}{}Note:{} GL and GPU tracers require OpenGL context for testing.", BOLD, YELLOW, RESET);
    println!("{}      Run renderer integration tests for full coverage.{}\n", DIM, RESET);

    // Assert that CPU tests pass
    let cpu_results = &all_results[0];
    assert_eq!(cpu_results.fail_count(), 0, "CPU tracer has failures");
}
