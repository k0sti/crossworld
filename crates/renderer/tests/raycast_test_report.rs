//! Raycast Test Report Generator
//!
//! Generates a comprehensive test report table comparing raycast behavior
//! across CPU, GL, and GPU tracers with ANSI colored output.
//!
//! Uses a generic tracer tester interface to validate any tracer implementation.

use cube::Cube;
use glam::Vec3;
use std::fmt;
use std::rc::Rc;

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
    entry_count: Option<u32>,
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

    fn raycast(&self, pos: Vec3, dir: Vec3, _max_depth: u32) -> Option<RaycastHit> {
        let mut debug = cube::RaycastDebugState::default();
        let hit = cube::raycast(&self.cube, pos, dir, Some(&mut debug))?;

        Some(RaycastHit {
            position: hit.pos,
            normal: hit.normal.as_vec3(),
            value: hit.value,
            entry_count: Some(debug.entry_count),
        })
    }
}

// ============================================================================
// GL Tracer Implementation
// ============================================================================

use renderer::gl_tracer::GlCubeTracer;

struct GlTracer {
    tracer: GlCubeTracer,
}

impl GlTracer {
    fn new(cube: Cube<i32>) -> Self {
        use std::rc::Rc;
        Self {
            tracer: GlCubeTracer::new(Rc::new(cube)),
        }
    }
}

impl Tracer for GlTracer {
    fn name(&self) -> &str {
        "GL"
    }

    fn raycast(&self, pos: Vec3, dir: Vec3, max_depth: u32) -> Option<RaycastHit> {
        let hit = self.tracer.raycast_octree(pos, dir, max_depth).ok()??;

        Some(RaycastHit {
            position: hit.pos,
            normal: hit.normal.as_vec3(),
            value: hit.value,
            entry_count: None, // GL tracer doesn't provide debug state
        })
    }

    fn is_available(&self) -> bool {
        true // Now available with CPU-side octree raycast
    }
}

// ============================================================================
// GPU Tracer Implementation
// ============================================================================

use renderer::gpu_tracer::GpuTracer as GpuTracerImpl;

struct GpuTracer {
    tracer: GpuTracerImpl,
}

impl GpuTracer {
    fn new(cube: Cube<i32>) -> Self {
        use std::rc::Rc;
        Self {
            tracer: GpuTracerImpl::new(Rc::new(cube)),
        }
    }
}

impl Tracer for GpuTracer {
    fn name(&self) -> &str {
        "GPU"
    }

    fn raycast(&self, pos: Vec3, dir: Vec3, max_depth: u32) -> Option<RaycastHit> {
        let hit = self.tracer.raycast_octree(pos, dir, max_depth).ok()??;

        Some(RaycastHit {
            position: hit.pos,
            normal: hit.normal.as_vec3(),
            value: hit.value,
            entry_count: None, // GPU tracer doesn't provide debug state
        })
    }

    fn is_available(&self) -> bool {
        true // Now available with CPU-side octree raycast
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

            // Check entry count
            if let Some(entry_count) = hit_result.entry_count {
                if entry_count < test.expected_enter_count_min
                    || entry_count > test.expected_enter_count_max
                {
                    return TestCaseResult {
                        test_number: test.number,
                        test_name: test.name.clone(),
                        result: TestResult::Fail,
                        error: Some(format!(
                            "Entry count {} outside range [{}, {}]",
                            entry_count,
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
    // All tests now loaded from markdown table
    load_table_driven_tests(1)
}

// ============================================================================
// Table-Driven Test Loading
// ============================================================================

/// Convert coordinate from [-1, 1]³ to [0, 1]³
fn convert_coord_from_table(v: Vec3) -> Vec3 {
    (v + Vec3::ONE) * 0.5
}

/// Parse Vec3 from markdown table string
fn parse_vec3_table(s: &str) -> Option<Vec3> {
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

/// Parse boolean from markdown table string
fn parse_bool_table(s: &str) -> Option<bool> {
    match s.trim().trim_matches('`').to_lowercase().as_str() {
        "true" | "✅" | "yes" | "1" => Some(true),
        "false" | "❌" | "no" | "0" => Some(false),
        _ => None,
    }
}

/// Load test cases from markdown table in test_raycast_table.md
fn load_table_driven_tests(starting_number: usize) -> Vec<TestCase> {
    // Include the markdown file from cube tests directory
    let content = include_str!("../../cube/tests/test_raycast_table.md");
    let mut test_cases = Vec::new();
    let mut number = starting_number;

    for (line_num, line) in content.lines().enumerate() {
        let line = line.trim();

        // Skip header, separator, and empty lines
        if line.is_empty() || line.starts_with("| Ray Origin") || line.starts_with("| :---") {
            continue;
        }

        // Parse table row
        if line.starts_with('|') {
            let parts: Vec<&str> = line.split('|').collect();

            // Need at least 11 parts
            if parts.len() < 11 {
                continue;
            }

            let origin_str = parts[1].trim();
            let direction_str = parts[2].trim();
            let hit_str = parts[3].trim();
            let voxel_str = parts[6].trim();
            let visits_str = parts[7].trim();
            let notes = parts.get(10).unwrap_or(&"").trim();

            // Parse origin and direction
            let Some(origin) = parse_vec3_table(origin_str) else {
                continue;
            };
            let Some(mut direction) = parse_vec3_table(direction_str) else {
                continue;
            };
            direction = direction.normalize();

            // Parse hit boolean
            let Some(expect_hit) = parse_bool_table(hit_str) else {
                continue;
            };

            // Parse voxel value
            let expected_voxel: Option<i32> = voxel_str
                .trim()
                .trim_matches('`')
                .parse()
                .ok()
                .map(|v: u8| v as i32);

            // Parse visits
            let expected_visits: i32 = visits_str.trim().trim_matches('`').parse().unwrap_or(1);

            // Detect coordinate system and convert if needed
            // If any coordinate is negative or > 1, it's in [-1,1]³ space (octa-cube tests)
            // Otherwise it's already in [0,1]³ space (solid cube tests)
            let origin_converted = if origin.x < 0.0
                || origin.y < 0.0
                || origin.z < 0.0
                || origin.x > 1.0
                || origin.y > 1.0
                || origin.z > 1.0
            {
                // Octa-cube test in [-1,1]³ space - convert to [0,1]³
                convert_coord_from_table(origin)
            } else {
                // Solid cube test already in [0,1]³ space - use as-is
                origin
            };

            let category = "Table-Driven".to_string();
            let name = if notes.is_empty() {
                format!("Table case {}", line_num + 1)
            } else {
                notes.to_string()
            };

            test_cases.push(TestCase {
                number,
                name,
                category,
                pos: origin_converted,
                dir: direction,
                should_hit: expect_hit,
                expected_value: expected_voxel,
                expected_enter_count_min: expected_visits.saturating_sub(2).max(0) as u32,
                expected_enter_count_max: (expected_visits + 2) as u32,
            });
            number += 1;
        }
    }

    test_cases
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
        self.results
            .iter()
            .filter(|r| r.result == TestResult::Pass)
            .count()
    }

    fn fail_count(&self) -> usize {
        self.results
            .iter()
            .filter(|r| r.result == TestResult::Fail)
            .count()
    }

    fn na_count(&self) -> usize {
        self.results
            .iter()
            .filter(|r| r.result == TestResult::NotImplemented)
            .count()
    }

    fn failures(&self) -> Vec<&TestCaseResult> {
        self.results
            .iter()
            .filter(|r| r.result == TestResult::Fail)
            .collect()
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
    println!(
        "{}┌────────────────────────────────────────────┬──────────────┐{}",
        DIM, RESET
    );
    println!(
        "{}│{} Change                                    {}│{} Status       {}│{}",
        DIM, RESET, DIM, RESET, DIM, RESET
    );
    println!(
        "{}├────────────────────────────────────────────┼──────────────┤{}",
        DIM, RESET
    );

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

    println!(
        "{}└────────────────────────────────────────────┴──────────────┘{}",
        DIM, RESET
    );
}

fn print_test_table(test_cases: &[TestCase], all_results: &[TracerResults]) {
    println!("\n{}{}Test Results by Category:{}", BOLD, BLUE, RESET);
    println!(
        "{}┌────┬─────────────────────────────┬──────┬──────┬──────┐{}",
        DIM, RESET
    );
    println!(
        "{}│{}  # {}│{} Test Case                  {}│{} CPU  {}│{} GL   {}│{} GPU  {}│{}",
        DIM, RESET, DIM, RESET, DIM, RESET, DIM, RESET, DIM, RESET, DIM, RESET
    );
    println!(
        "{}├────┼─────────────────────────────┼──────┼──────┼──────┤{}",
        DIM, RESET
    );

    let mut current_category = String::new();
    for test in test_cases {
        // Print category header if changed
        if test.category != current_category {
            if !current_category.is_empty() {
                println!(
                    "{}├────┼─────────────────────────────┼──────┼──────┼──────┤{}",
                    DIM, RESET
                );
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

        print!(
            "{}│{} {:>2} {}│{} {:<27} {}│ ",
            DIM, RESET, test.number, DIM, RESET, test.name, DIM
        );
        print!("{} ", cpu_result.result);
        print!("{}│ ", DIM);
        print!("{} ", gl_result.result);
        print!("{}│ ", DIM);
        print!("{} ", gpu_result.result);
        println!("{}│{}", DIM, RESET);
    }

    println!(
        "{}└────┴─────────────────────────────┴──────┴──────┴──────┘{}",
        DIM, RESET
    );
}

fn print_failures(all_results: &[TracerResults]) {
    let mut has_failures = false;

    for tracer_results in all_results {
        let failures = tracer_results.failures();
        if !failures.is_empty() {
            has_failures = true;
            println!(
                "\n{}{}Failures for {} Tracer:{}",
                BOLD, RED, tracer_results.tracer_name, RESET
            );
            for failure in failures {
                println!(
                    "  {}#{} {}{}: {}{}{}",
                    DIM,
                    failure.test_number,
                    RESET,
                    failure.test_name,
                    RED,
                    failure.error.as_deref().unwrap_or("Unknown error"),
                    RESET
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
    println!(
        "{}│{} Tracer   {}│{} Pass {}│{} Fail {}│{} N/A  {}│",
        DIM, RESET, DIM, RESET, DIM, RESET, DIM, RESET, DIM
    );
    println!("{}├──────────┼──────┼──────┼──────┤", DIM);

    for tracer_results in all_results {
        print!(
            "{}│{} {:<8} {}│ ",
            DIM, RESET, tracer_results.tracer_name, DIM
        );
        print!("{}{:>4}{} ", GREEN, tracer_results.pass_count(), RESET);
        print!("{}│ ", DIM);
        print!("{}{:>4}{} ", RED, tracer_results.fail_count(), RESET);
        print!("{}│ ", DIM);
        print!("{}{:>4}{} ", YELLOW, tracer_results.na_count(), RESET);
        println!("{}│{}", DIM, RESET);
    }

    println!("{}└──────────┴──────┴──────┴──────┘{}", DIM, RESET);

    // Print per-tracer stats
    print!("\n{}Total tests: {}{}{}", BOLD, CYAN, total_tests, RESET);
    for tracer_results in all_results {
        print!(
            " | {}: {}{}/{}{}",
            tracer_results.tracer_name,
            GREEN,
            tracer_results.pass_count(),
            total_tests,
            RESET
        );
    }
    println!(" passed");
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Create the octa-cube for table-driven tests
/// Bounds: [0, 0, 0] to [1, 1, 1] (after conversion from [-1,-1,-1] to [1,1,1])
/// Depth: 1 (2×2×2 grid)
fn create_octa_cube() -> Cube<i32> {
    // Children array in cube crate order (x*4 + y*2 + z):
    // Node 0: (0,0,0) = value 1
    // Node 1: (0,0,1) = value 3
    // Node 2: (0,1,0) = value 0 (empty)
    // Node 3: (0,1,1) = value 5
    // Node 4: (1,0,0) = value 2
    // Node 5: (1,0,1) = value 4
    // Node 6: (1,1,0) = value 0 (empty)
    // Node 7: (1,1,1) = value 0 (empty)
    let children: [Rc<Cube<i32>>; 8] = [
        Rc::new(Cube::Solid(1)), // Node 0
        Rc::new(Cube::Solid(3)), // Node 1
        Rc::new(Cube::Solid(0)), // Node 2 - empty
        Rc::new(Cube::Solid(5)), // Node 3
        Rc::new(Cube::Solid(2)), // Node 4
        Rc::new(Cube::Solid(4)), // Node 5
        Rc::new(Cube::Solid(0)), // Node 6 - empty
        Rc::new(Cube::Solid(0)), // Node 7 - empty
    ];

    Cube::Cubes(Box::new(children))
}

// ============================================================================
// Main Test
// ============================================================================

#[test]
fn test_raycast_report() {
    // Create both cubes for different test types
    let solid_cube = Cube::Solid(1i32);
    let octa_cube = create_octa_cube();

    let test_cases = create_test_cases();

    // Create tracers for both cubes
    let solid_tracers: Vec<Box<dyn Tracer>> = vec![
        Box::new(CpuTracer::new(solid_cube.clone())),
        Box::new(GlTracer::new(solid_cube.clone())),
        Box::new(GpuTracer::new(solid_cube)),
    ];

    let octa_tracers: Vec<Box<dyn Tracer>> = vec![
        Box::new(CpuTracer::new(octa_cube.clone())),
        Box::new(GlTracer::new(octa_cube.clone())),
        Box::new(GpuTracer::new(octa_cube)),
    ];

    // Run all tests for all tracers
    // Use the appropriate cube based on expected voxel value
    // (simple solid cube tests expect value 1, octa-cube tests expect values 0-5)
    let mut all_results = Vec::new();

    for tracer_idx in 0..3 {
        let mut results = Vec::new();
        for test in &test_cases {
            // Choose tracer based on coordinate system
            // Solid cube tests: coordinates in [0,1]³ and expect value 1 or 0 (miss)
            // Octa-cube tests: other values (2,3,4,5) or coordinates needed conversion
            let use_solid = test.pos.x >= 0.0
                && test.pos.x <= 1.0
                && test.pos.y >= 0.0
                && test.pos.y <= 1.0
                && test.pos.z >= 0.0
                && test.pos.z <= 1.0
                && (test.expected_value == Some(1)
                    || test.expected_value == Some(0)
                    || test.expected_value.is_none());

            let tracer: &dyn Tracer = if use_solid {
                solid_tracers[tracer_idx].as_ref()
            } else {
                octa_tracers[tracer_idx].as_ref()
            };

            results.push(test_tracer(tracer, test));
        }

        let tracer_name = if tracer_idx == 0 {
            "CPU"
        } else if tracer_idx == 1 {
            "GL"
        } else {
            "GPU"
        };

        all_results.push(TracerResults {
            tracer_name: tracer_name.to_string(),
            results,
        });
    }

    // Generate report
    print_header();
    print_openspec_status();
    print_test_table(&test_cases, &all_results);
    print_failures(&all_results);
    print_summary(&all_results, test_cases.len());

    println!(
        "\n{}{}Note:{} All tracers use CPU-side octree raycast for testing.",
        BOLD, YELLOW, RESET
    );
    println!(
        "{}      GL/GPU tracers also have shader-based implementations for rendering.{}\n",
        DIM, RESET
    );

    // Assert that all tracers pass
    for tracer_results in &all_results {
        assert_eq!(
            tracer_results.fail_count(),
            0,
            "{} tracer has failures",
            tracer_results.tracer_name
        );
    }
}
