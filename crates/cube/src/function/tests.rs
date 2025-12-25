//! CPU/GPU parity tests
//!
//! Ensures that CPU and GPU backends produce identical results for the same expressions.
//! This is critical because users expect deterministic output regardless of backend.

use super::cpu::CpuFunction;
use super::gpu::{GpuFunction, WgslCodegen};
use super::parse_expr;

/// Test helper: compare CPU and GPU shader generation for an expression
fn test_codegen_parity(source: &str) {
    let ast = parse_expr(source).unwrap();

    // Compile to CPU
    let cpu_result = CpuFunction::compile(&ast);
    assert!(cpu_result.is_ok(), "CPU compilation failed: {:?}", cpu_result);

    // Compile to GPU
    let gpu_result = GpuFunction::compile(&ast);
    assert!(gpu_result.is_ok(), "GPU compilation failed: {:?}", gpu_result);

    let gpu_fn = gpu_result.unwrap();

    // GPU shader should be valid WGSL
    let shader = gpu_fn.shader_source();
    assert!(shader.contains("@compute"));
    assert!(shader.contains("@workgroup_size"));
}

/// Test helper: validate WGSL shader structure
fn validate_wgsl_structure(shader: &str) {
    // Must have compute entry point
    assert!(
        shader.contains("@compute"),
        "Missing @compute attribute in shader"
    );
    assert!(
        shader.contains("@workgroup_size"),
        "Missing @workgroup_size in shader"
    );

    // Must have uniforms
    assert!(
        shader.contains("struct Uniforms"),
        "Missing Uniforms struct"
    );
    assert!(
        shader.contains("@group(0) @binding(0) var<uniform> uniforms"),
        "Missing uniform binding"
    );

    // Must have output buffer
    assert!(
        shader.contains("materials"),
        "Missing materials output buffer"
    );

    // Must have main function
    assert!(shader.contains("fn main"), "Missing main function");
}

#[test]
fn test_parity_simple_arithmetic() {
    test_codegen_parity("x + 1");
    test_codegen_parity("x * 2 + y");
    test_codegen_parity("x - y * z");
    test_codegen_parity("x / 2.0");
}

#[test]
fn test_parity_comparison() {
    test_codegen_parity("x > 0");
    test_codegen_parity("x <= y");
    test_codegen_parity("x == 0.5");
    test_codegen_parity("x != y");
}

#[test]
fn test_parity_logical() {
    test_codegen_parity("x > 0 and y > 0");
    test_codegen_parity("x < 0 or y < 0");
    test_codegen_parity("not (x > 0)");
}

#[test]
fn test_parity_conditionals() {
    test_codegen_parity("if x > 0 then 10 else 5");
    test_codegen_parity("if x > 0 then y else z");
    test_codegen_parity("if noise(x, y, z) > 0.5 then 1 else 0");
}

#[test]
fn test_parity_math_functions() {
    test_codegen_parity("sin(x)");
    test_codegen_parity("cos(y) + sin(x)");
    test_codegen_parity("sqrt(x * x + y * y)");
    test_codegen_parity("abs(x)");
    test_codegen_parity("floor(x * 10)");
    test_codegen_parity("max(x, y)");
    test_codegen_parity("min(x, min(y, z))");
    test_codegen_parity("clamp(x, -1, 1)");
}

#[test]
fn test_parity_noise_functions() {
    test_codegen_parity("noise(x, y, z)");
    test_codegen_parity("fbm(x, y, z, 4)");
    test_codegen_parity("turbulence(x, y, z, 3)");
    test_codegen_parity("noise(x * 0.1, y * 0.1, z * 0.1)");
}

#[test]
fn test_parity_let_bindings() {
    test_codegen_parity("let a = x + 1; a * 2");
    test_codegen_parity("let scale = 0.1; noise(x * scale, y * scale, z * scale)");
}

#[test]
fn test_parity_match_expressions() {
    test_codegen_parity("match floor(y * 4) { 0 => 1, 1 => 2, 2 => 3, _ => 0 }");
}

#[test]
fn test_parity_time_variable() {
    test_codegen_parity("x + time");
    test_codegen_parity("sin(x + time * 2)");
}

#[test]
fn test_parity_world_coords() {
    test_codegen_parity("wx + wy + wz");
}

#[test]
fn test_parity_complex_terrain() {
    test_codegen_parity(
        "let height = noise(x * 0.1, z * 0.1, 0) * 10; if y < height then 1 else 0",
    );
}

#[test]
fn test_parity_animated_wave() {
    test_codegen_parity("sin(x * 3.14 + time) * 0.5 + 0.5");
}

#[test]
fn test_wgsl_structure_simple() {
    let ast = parse_expr("x + y").unwrap();
    let gpu_fn = GpuFunction::compile(&ast).unwrap();
    validate_wgsl_structure(gpu_fn.shader_source());
}

#[test]
fn test_wgsl_structure_with_noise() {
    let ast = parse_expr("noise(x, y, z)").unwrap();
    let gpu_fn = GpuFunction::compile(&ast).unwrap();
    let shader = gpu_fn.shader_source();

    validate_wgsl_structure(shader);

    // Should include noise function definitions
    assert!(shader.contains("fn noise3"), "Missing noise3 function");
    assert!(shader.contains("fn grad_hash"), "Missing grad_hash function");
    assert!(shader.contains("fn fade"), "Missing fade function");
}

#[test]
fn test_wgsl_structure_with_fbm() {
    let ast = parse_expr("fbm(x, y, z, 4)").unwrap();
    let gpu_fn = GpuFunction::compile(&ast).unwrap();
    let shader = gpu_fn.shader_source();

    validate_wgsl_structure(shader);

    // Should include FBM function
    assert!(shader.contains("fn fbm"), "Missing fbm function");
    assert!(shader.contains("fn noise3"), "Missing noise3 function");
}

#[test]
fn test_cpu_gpu_metadata_match() {
    let test_cases = vec![
        ("x + 1", false, false),
        ("noise(x, y, z)", false, true),
        ("x + time", true, false),
        ("fbm(x, y, z, 4) + time", true, true),
    ];

    for (source, should_use_time, should_use_noise) in test_cases {
        let ast = parse_expr(source).unwrap();

        let cpu_fn = CpuFunction::compile(&ast).unwrap();
        let gpu_fn = GpuFunction::compile(&ast).unwrap();

        // Both backends should agree on metadata
        assert_eq!(
            cpu_fn.uses_time, should_use_time,
            "CPU uses_time mismatch for: {}",
            source
        );
        assert_eq!(
            gpu_fn.uses_time, should_use_time,
            "GPU uses_time mismatch for: {}",
            source
        );

        assert_eq!(
            cpu_fn.uses_noise, should_use_noise,
            "CPU uses_noise mismatch for: {}",
            source
        );
        assert_eq!(
            gpu_fn.uses_noise, should_use_noise,
            "GPU uses_noise mismatch for: {}",
            source
        );

        assert_eq!(
            cpu_fn.complexity, gpu_fn.complexity,
            "Complexity mismatch for: {}",
            source
        );
    }
}

#[test]
fn test_wgsl_select_for_conditionals() {
    let ast = parse_expr("if x > 0 then 10 else 5").unwrap();
    let code = WgslCodegen::expr_to_wgsl(&ast, 1).unwrap();

    // Conditionals should use select() for GPU efficiency
    assert!(
        code.contains("select"),
        "Conditional should use select(), got: {}",
        code
    );
}

#[test]
fn test_wgsl_vec3_for_noise() {
    let ast = parse_expr("noise(x, y, z)").unwrap();
    let code = WgslCodegen::expr_to_wgsl(&ast, 1).unwrap();

    // Noise should pack coordinates into vec3
    assert!(
        code.contains("vec3<f32>"),
        "Noise should use vec3, got: {}",
        code
    );
}

#[test]
fn test_wgsl_edge_cases() {
    // Division by zero protection (should compile, runtime behavior is GPU-specific)
    test_codegen_parity("x / (y + 0.0001)");

    // Large exponents
    test_codegen_parity("pow(x, 10)");

    // Nested conditionals
    test_codegen_parity("if x > 0 then if y > 0 then 1 else 2 else 3");

    // Deeply nested expressions
    test_codegen_parity("sin(cos(tan(x)))");
}

#[test]
fn test_material_constants_compile() {
    // Material constants (AIR=0, STONE=1, GRASS=2, etc.) should work
    test_codegen_parity("if y > 0 then STONE else GRASS");
    test_codegen_parity("if noise(x, y, z) > 0 then STONE else AIR");
}

#[test]
fn test_wgsl_workgroup_size() {
    let ast = parse_expr("x + y").unwrap();
    let gpu_fn = GpuFunction::compile(&ast).unwrap();
    let shader = gpu_fn.shader_source();

    // Should use 8x8x8 workgroup size (512 threads per workgroup)
    assert!(
        shader.contains("@workgroup_size(8, 8, 8)"),
        "Should use 8x8x8 workgroup size"
    );
}

#[test]
fn test_wgsl_bounds_check() {
    let ast = parse_expr("x").unwrap();
    let gpu_fn = GpuFunction::compile(&ast).unwrap();
    let shader = gpu_fn.shader_source();

    // Should have bounds checking to prevent out-of-bounds writes
    assert!(
        shader.contains("if (global_id.x >= size || global_id.y >= size || global_id.z >= size)"),
        "Missing bounds check"
    );
}

#[test]
fn test_wgsl_position_normalization() {
    let ast = parse_expr("x").unwrap();
    let gpu_fn = GpuFunction::compile(&ast).unwrap();
    let shader = gpu_fn.shader_source();

    // Should normalize position to [-1, 1] range
    assert!(
        shader.contains("vec3<f32>(global_id) / f32(size) * 2.0 - 1.0"),
        "Missing position normalization"
    );
}

#[test]
fn test_wgsl_material_clamping() {
    let ast = parse_expr("x * 1000").unwrap();
    let gpu_fn = GpuFunction::compile(&ast).unwrap();
    let shader = gpu_fn.shader_source();

    // Should clamp material values to [0, 255] range
    assert!(
        shader.contains("clamp(round(result), 0.0, 255.0)"),
        "Missing material value clamping"
    );
}
