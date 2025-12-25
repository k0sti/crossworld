//! WGSL code generator for function expressions
//!
//! Converts AST expressions to WebGPU Shading Language (WGSL) compute shaders.
//! The generated shaders evaluate expressions at millions of voxel positions in parallel.

use super::GpuCompileError;
use crate::function::ast::{BinOpKind, BuiltinFunc, Expr, MatchPattern, UnaryOpKind, VarId};

/// WGSL code generator
pub struct WgslCodegen;

impl WgslCodegen {
    /// Generate a complete WGSL compute shader from an expression AST
    pub fn generate(expr: &Expr) -> Result<String, GpuCompileError> {
        let mut code = String::new();

        // Add noise functions if needed
        if expr.uses_noise() {
            code.push_str(&Self::noise_functions());
            code.push('\n');
        }

        // Add uniforms structure
        code.push_str(&Self::uniforms_struct());
        code.push('\n');

        // Add storage buffers
        code.push_str(&Self::storage_buffers());
        code.push('\n');

        // Add compute shader main function
        code.push_str(&Self::compute_main(expr)?);

        Ok(code)
    }

    /// Generate the uniforms structure
    fn uniforms_struct() -> String {
        r#"// Uniforms passed to the shader
struct Uniforms {
    time: f32,
    depth: u32,
    seed: u32,
    _padding: u32,  // Align to 16 bytes
    world_offset: vec3<f32>,
    size: u32,  // Cube resolution (e.g., 128)
}

@group(0) @binding(0) var<uniform> uniforms: Uniforms;"#
            .to_string()
    }

    /// Generate storage buffer declarations
    fn storage_buffers() -> String {
        r#"
// Output material buffer (flat array)
@group(0) @binding(1) var<storage, read_write> materials: array<u32>;"#
            .to_string()
    }

    /// Generate the compute shader main function
    fn compute_main(expr: &Expr) -> Result<String, GpuCompileError> {
        let expr_code = Self::expr_to_wgsl(expr, 1)?;

        Ok(format!(
            r#"
@compute @workgroup_size(8, 8, 8)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {{
    let size = uniforms.size;

    // Early exit if out of bounds
    if (global_id.x >= size || global_id.y >= size || global_id.z >= size) {{
        return;
    }}

    // Convert to normalized position [-1, 1]
    let pos = vec3<f32>(global_id) / f32(size) * 2.0 - 1.0;
    let x = pos.x;
    let y = pos.y;
    let z = pos.z;

    // World position
    let world_pos = pos + uniforms.world_offset;
    let wx = world_pos.x;
    let wy = world_pos.y;
    let wz = world_pos.z;

    // Time and other uniforms
    let time = uniforms.time;
    let depth = uniforms.depth;
    let seed = uniforms.seed;

    // Evaluate expression
    let result: f32 = {expr_code};

    // Clamp to material index range and write to buffer
    let material = u32(clamp(round(result), 0.0, 255.0));
    let idx = global_id.z * size * size + global_id.y * size + global_id.x;
    materials[idx] = material;
}}"#
        ))
    }

    /// Convert an expression to WGSL code
    pub fn expr_to_wgsl(expr: &Expr, indent: usize) -> Result<String, GpuCompileError> {
        match expr {
            Expr::Number(n) => Ok(format!("{}", n)),

            Expr::Var(var) => Ok(Self::var_to_wgsl(*var)),

            Expr::UserVar(name) => Ok(name.clone()),

            Expr::BinOp { op, left, right } => {
                let left_code = Self::expr_to_wgsl(left, indent)?;
                let right_code = Self::expr_to_wgsl(right, indent)?;
                let op_code = Self::binop_to_wgsl(*op);
                Ok(format!("({} {} {})", left_code, op_code, right_code))
            }

            Expr::UnaryOp { op, expr: e } => {
                let expr_code = Self::expr_to_wgsl(e, indent)?;
                match op {
                    UnaryOpKind::Neg => Ok(format!("(-{})", expr_code)),
                    UnaryOpKind::Not => Ok(format!("(1.0 - {})", expr_code)),
                }
            }

            Expr::Call { func, args } => {
                let arg_codes: Vec<String> = args
                    .iter()
                    .map(|a| Self::expr_to_wgsl(a, indent))
                    .collect::<Result<_, _>>()?;

                let func_code = Self::func_to_wgsl(*func, &arg_codes)?;
                Ok(func_code)
            }

            Expr::If {
                cond,
                then_expr,
                else_expr,
            } => {
                let cond_code = Self::expr_to_wgsl(cond, indent)?;
                let then_code = Self::expr_to_wgsl(then_expr, indent)?;
                let else_code = Self::expr_to_wgsl(else_expr, indent)?;

                // Use select() for simple conditional (more GPU-friendly than if/else)
                Ok(format!(
                    "select({}, {}, {})",
                    else_code, then_code, cond_code
                ))
            }

            Expr::Let { name, value, body } => {
                let value_code = Self::expr_to_wgsl(value, indent)?;
                let body_code = Self::expr_to_wgsl(body, indent)?;

                // WGSL doesn't support expression-level let bindings directly,
                // so we inline the value. For complex expressions, this could be
                // optimized by hoisting to statement level, but that requires
                // changing the function signature to return statements instead of expressions.
                // For now, we use a simple substitution approach similar to CPU backend.
                Ok(body_code.replace(name, &format!("({})", value_code)))
            }

            Expr::Match {
                expr,
                cases,
                default,
            } => {
                let expr_code = Self::expr_to_wgsl(expr, indent)?;
                let default_code = Self::expr_to_wgsl(default, indent)?;

                // Build nested select() chain for match
                let mut result = default_code;
                for (pattern, case_expr) in cases.iter().rev() {
                    let case_code = Self::expr_to_wgsl(case_expr, indent)?;
                    let cond_code = match pattern {
                        MatchPattern::Number(n) => format!("({} == {})", expr_code, n),
                        MatchPattern::Range { low, high } => {
                            format!("(({} >= {}) && ({} < {}))", expr_code, low, expr_code, high)
                        }
                    };
                    result = format!("select({}, {}, {})", result, case_code, cond_code);
                }
                Ok(result)
            }
        }
    }

    /// Convert a variable to WGSL
    fn var_to_wgsl(var: VarId) -> String {
        match var {
            VarId::X => "x".to_string(),
            VarId::Y => "y".to_string(),
            VarId::Z => "z".to_string(),
            VarId::WorldX => "wx".to_string(),
            VarId::WorldY => "wy".to_string(),
            VarId::WorldZ => "wz".to_string(),
            VarId::Time => "time".to_string(),
            VarId::Depth => "f32(depth)".to_string(),
            VarId::Seed => "f32(seed)".to_string(),
        }
    }

    /// Convert a binary operator to WGSL
    fn binop_to_wgsl(op: BinOpKind) -> &'static str {
        match op {
            BinOpKind::Add => "+",
            BinOpKind::Sub => "-",
            BinOpKind::Mul => "*",
            BinOpKind::Div => "/",
            BinOpKind::Mod => "%",
            BinOpKind::Pow => "^", // Will be converted to pow() in func_to_wgsl
            BinOpKind::Lt => "<",
            BinOpKind::Le => "<=",
            BinOpKind::Gt => ">",
            BinOpKind::Ge => ">=",
            BinOpKind::Eq => "==",
            BinOpKind::Ne => "!=",
            BinOpKind::And => "&&",
            BinOpKind::Or => "||",
        }
    }

    /// Convert a function call to WGSL
    fn func_to_wgsl(func: BuiltinFunc, args: &[String]) -> Result<String, GpuCompileError> {
        let func_name = match func {
            // Noise functions (custom implementations)
            BuiltinFunc::Noise => {
                if args.len() != 3 {
                    return Err(GpuCompileError::CodegenError(format!(
                        "noise() requires 3 arguments, got {}",
                        args.len()
                    )));
                }
                return Ok(format!(
                    "noise3(vec3<f32>({}, {}, {}), seed)",
                    args[0], args[1], args[2]
                ));
            }
            BuiltinFunc::Fbm => {
                if args.len() != 4 {
                    return Err(GpuCompileError::CodegenError(format!(
                        "fbm() requires 4 arguments, got {}",
                        args.len()
                    )));
                }
                return Ok(format!(
                    "fbm(vec3<f32>({}, {}, {}), u32({}), seed)",
                    args[0], args[1], args[2], args[3]
                ));
            }
            BuiltinFunc::Turbulence => {
                if args.len() != 4 {
                    return Err(GpuCompileError::CodegenError(format!(
                        "turbulence() requires 4 arguments, got {}",
                        args.len()
                    )));
                }
                return Ok(format!(
                    "turbulence(vec3<f32>({}, {}, {}), u32({}), seed)",
                    args[0], args[1], args[2], args[3]
                ));
            }

            // WGSL built-ins (direct mapping)
            BuiltinFunc::Sin => "sin",
            BuiltinFunc::Cos => "cos",
            BuiltinFunc::Tan => "tan",
            BuiltinFunc::Asin => "asin",
            BuiltinFunc::Acos => "acos",
            BuiltinFunc::Atan => "atan",
            BuiltinFunc::Atan2 => "atan2",
            BuiltinFunc::Sqrt => "sqrt",
            BuiltinFunc::Pow => "pow",
            BuiltinFunc::Exp => "exp",
            BuiltinFunc::Ln => "log", // WGSL uses 'log' for natural log
            BuiltinFunc::Log2 => "log2",
            BuiltinFunc::Log10 => {
                // WGSL doesn't have log10, use log(x) / log(10)
                if args.len() != 1 {
                    return Err(GpuCompileError::CodegenError(
                        "log10() requires 1 argument".to_string(),
                    ));
                }
                return Ok(format!("(log({}) / 2.302585)", args[0]));
            }
            BuiltinFunc::Floor => "floor",
            BuiltinFunc::Ceil => "ceil",
            BuiltinFunc::Round => "round",
            BuiltinFunc::Trunc => "trunc",
            BuiltinFunc::Fract => "fract",
            BuiltinFunc::Abs => "abs",
            BuiltinFunc::Sign => "sign",
            BuiltinFunc::Min => "min",
            BuiltinFunc::Max => "max",
            BuiltinFunc::Clamp => "clamp",
            BuiltinFunc::Lerp => "mix", // WGSL uses 'mix' for lerp
            BuiltinFunc::Smoothstep => "smoothstep",
            BuiltinFunc::Step => "step",
        };

        Ok(format!("{}({})", func_name, args.join(", ")))
    }

    /// Generate WGSL noise function implementations
    fn noise_functions() -> String {
        r#"// ============================================================================
// Noise Function Library (matches CPU implementation)
// ============================================================================

// Hash function for gradient lookup
fn grad_hash(x: i32, y: i32, z: i32, seed: u32) -> f32 {
    var h = x + y * 57 + z * 113 + i32(seed);
    h = i32(u32(h) * 0x27d4eb2d);
    h = h ^ (h >> 15);
    return f32(u32(h) & 0x7FFFFFFF) / f32(0x7FFFFFFF);
}

// Fade function for smooth interpolation: 6t^5 - 15t^4 + 10t^3
fn fade(t: f32) -> f32 {
    return t * t * t * (t * (t * 6.0 - 15.0) + 10.0);
}

// Value noise 3D - deterministic noise function
// Returns values in the range [-1, 1]
fn noise3(p: vec3<f32>, seed: u32) -> f32 {
    // Integer coordinates
    let xi = i32(floor(p.x));
    let yi = i32(floor(p.y));
    let zi = i32(floor(p.z));

    // Fractional coordinates
    let xf = p.x - floor(p.x);
    let yf = p.y - floor(p.y);
    let zf = p.z - floor(p.z);

    // Fade curves
    let u = fade(xf);
    let v = fade(yf);
    let w = fade(zf);

    // Hash cube corners
    let h000 = grad_hash(xi, yi, zi, seed);
    let h001 = grad_hash(xi, yi, zi + 1, seed);
    let h010 = grad_hash(xi, yi + 1, zi, seed);
    let h011 = grad_hash(xi, yi + 1, zi + 1, seed);
    let h100 = grad_hash(xi + 1, yi, zi, seed);
    let h101 = grad_hash(xi + 1, yi, zi + 1, seed);
    let h110 = grad_hash(xi + 1, yi + 1, zi, seed);
    let h111 = grad_hash(xi + 1, yi + 1, zi + 1, seed);

    // Trilinear interpolation
    let x00 = mix(h000, h100, u);
    let x01 = mix(h001, h101, u);
    let x10 = mix(h010, h110, u);
    let x11 = mix(h011, h111, u);

    let y0 = mix(x00, x10, v);
    let y1 = mix(x01, x11, v);

    let result = mix(y0, y1, w);

    // Map from [0, 1] to [-1, 1]
    return result * 2.0 - 1.0;
}

// Fractal Brownian Motion (FBM) - layered noise
// Returns values in approximately [-1, 1]
fn fbm(p: vec3<f32>, octaves: u32, seed: u32) -> f32 {
    var value = 0.0;
    var amplitude = 0.5;
    var frequency = 1.0;
    var max_value = 0.0;

    for (var i = 0u; i < min(octaves, 8u); i++) {
        let octave_seed = seed + i;
        value += amplitude * noise3(p * frequency, octave_seed);
        max_value += amplitude;
        amplitude *= 0.5;
        frequency *= 2.0;
    }

    return value / max_value;
}

// Turbulence noise - absolute value of FBM layers
// Returns values in [0, 1]
fn turbulence(p: vec3<f32>, octaves: u32, seed: u32) -> f32 {
    var value = 0.0;
    var amplitude = 0.5;
    var frequency = 1.0;
    var max_value = 0.0;

    for (var i = 0u; i < min(octaves, 8u); i++) {
        let octave_seed = seed + i;
        value += amplitude * abs(noise3(p * frequency, octave_seed));
        max_value += amplitude;
        amplitude *= 0.5;
        frequency *= 2.0;
    }

    return value / max_value;
}"#
            .to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::function::ast::{Expr, VarId};

    #[test]
    fn test_simple_expr() {
        let expr = Expr::binop(BinOpKind::Add, Expr::var(VarId::X), Expr::number(1.0));
        let code = WgslCodegen::expr_to_wgsl(&expr, 1).unwrap();
        assert_eq!(code, "(x + 1)");
    }

    #[test]
    fn test_function_call() {
        let expr = Expr::call(BuiltinFunc::Sin, vec![Expr::var(VarId::X)]);
        let code = WgslCodegen::expr_to_wgsl(&expr, 1).unwrap();
        assert_eq!(code, "sin(x)");
    }

    #[test]
    fn test_noise_call() {
        let expr = Expr::call(
            BuiltinFunc::Noise,
            vec![Expr::var(VarId::X), Expr::var(VarId::Y), Expr::var(VarId::Z)],
        );
        let code = WgslCodegen::expr_to_wgsl(&expr, 1).unwrap();
        assert!(code.contains("noise3"));
        assert!(code.contains("vec3"));
    }

    #[test]
    fn test_conditional() {
        let expr = Expr::if_then_else(
            Expr::binop(BinOpKind::Gt, Expr::var(VarId::X), Expr::number(0.0)),
            Expr::number(10.0),
            Expr::number(5.0),
        );
        let code = WgslCodegen::expr_to_wgsl(&expr, 1).unwrap();
        assert!(code.contains("select"));
    }

    #[test]
    fn test_complete_shader() {
        let expr = Expr::binop(BinOpKind::Add, Expr::var(VarId::X), Expr::var(VarId::Y));
        let shader = WgslCodegen::generate(&expr).unwrap();

        // Should contain key components
        assert!(shader.contains("@compute"));
        assert!(shader.contains("@workgroup_size"));
        assert!(shader.contains("Uniforms"));
        assert!(shader.contains("materials"));
        assert!(shader.contains("x + y"));
    }

    #[test]
    fn test_noise_shader() {
        let expr = Expr::call(
            BuiltinFunc::Noise,
            vec![Expr::var(VarId::X), Expr::var(VarId::Y), Expr::var(VarId::Z)],
        );
        let shader = WgslCodegen::generate(&expr).unwrap();

        // Should include noise functions
        assert!(shader.contains("fn noise3"));
        assert!(shader.contains("fn grad_hash"));
        assert!(shader.contains("fn fade"));
    }

    #[test]
    fn test_time_variable() {
        let expr = Expr::binop(BinOpKind::Mul, Expr::var(VarId::X), Expr::var(VarId::Time));
        let code = WgslCodegen::expr_to_wgsl(&expr, 1).unwrap();
        assert!(code.contains("time"));
    }

    #[test]
    fn test_world_coords() {
        let expr = Expr::var(VarId::WorldX);
        let code = WgslCodegen::expr_to_wgsl(&expr, 1).unwrap();
        assert_eq!(code, "wx");
    }
}
