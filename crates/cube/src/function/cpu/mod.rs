//! CPU backend for function expression evaluation
//!
//! Uses fasteval for compiled expression evaluation with custom noise functions.

mod noise;

use fasteval::{Compiler, Evaler, Instruction, Slab};
use thiserror::Error;

use super::ast::{BinOpKind, BuiltinFunc, Expr, MatchPattern, UnaryOpKind};
use noise::{fbm, noise3, turbulence};

/// Errors that can occur during CPU compilation
#[derive(Debug, Error)]
pub enum CpuCompileError {
    #[error("Fasteval error: {0}")]
    FastevalError(String),

    #[error("Unsupported expression: {0}")]
    UnsupportedExpr(String),
}

/// Evaluation context providing runtime values
#[derive(Debug, Clone)]
pub struct EvalContext {
    /// Animation time in seconds
    pub time: f64,
    /// Current octree depth
    pub depth: u32,
    /// Random seed for noise
    pub seed: u32,
    /// World position offset
    pub world_offset: (f64, f64, f64),
}

impl EvalContext {
    /// Create a new evaluation context
    pub fn new(time: f64, depth: u32, seed: u32) -> Self {
        Self {
            time,
            depth,
            seed,
            world_offset: (0.0, 0.0, 0.0),
        }
    }

    /// Set world position offset
    pub fn with_world_offset(mut self, x: f64, y: f64, z: f64) -> Self {
        self.world_offset = (x, y, z);
        self
    }
}

impl Default for EvalContext {
    fn default() -> Self {
        Self::new(0.0, 0, 0)
    }
}

/// Compiled CPU function for expression evaluation
#[derive(Debug)]
pub struct CpuFunction {
    /// The fasteval expression string (converted from AST)
    expr_string: String,
    /// Compiled fasteval instruction
    compiled: Instruction,
    /// Fasteval slab for evaluation
    slab: Slab,
    /// Whether the expression uses time
    pub uses_time: bool,
    /// Whether the expression uses noise functions
    pub uses_noise: bool,
    /// Estimated complexity
    pub complexity: u32,
}

impl CpuFunction {
    /// Compile an AST expression to a CPU-evaluable function
    pub fn compile(ast: &Expr) -> Result<Self, CpuCompileError> {
        // Collect info about the expression
        let uses_time = ast.uses_time();
        let uses_noise = ast.uses_noise();
        let complexity = ast.estimate_complexity();

        // Convert AST to fasteval expression string
        let expr_string = ast_to_fasteval(ast)?;

        // Compile with fasteval
        let parser = fasteval::Parser::new();
        let mut slab = Slab::new();

        let compiled = parser
            .parse(&expr_string, &mut slab.ps)
            .map_err(|e| CpuCompileError::FastevalError(e.to_string()))?
            .from(&slab.ps)
            .compile(&slab.ps, &mut slab.cs);

        Ok(Self {
            expr_string,
            compiled,
            slab,
            uses_time,
            uses_noise,
            complexity,
        })
    }

    /// Evaluate the function at a single point
    pub fn eval(&self, x: f64, y: f64, z: f64, ctx: &EvalContext) -> f64 {
        let mut ns = |name: &str, args: Vec<f64>| -> Option<f64> {
            match name {
                // Built-in variables
                "x" => Some(x),
                "y" => Some(y),
                "z" => Some(z),
                "wx" => Some(x + ctx.world_offset.0),
                "wy" => Some(y + ctx.world_offset.1),
                "wz" => Some(z + ctx.world_offset.2),
                "time" => Some(ctx.time),
                "depth" => Some(ctx.depth as f64),
                "seed" => Some(ctx.seed as f64),

                // Noise functions
                "noise" if args.len() == 3 => Some(noise3(args[0], args[1], args[2], ctx.seed)),
                "fbm" if args.len() == 4 => {
                    Some(fbm(args[0], args[1], args[2], args[3] as u32, ctx.seed))
                }
                "turbulence" if args.len() == 4 => {
                    Some(turbulence(args[0], args[1], args[2], args[3] as u32, ctx.seed))
                }

                // Additional math functions not in fasteval
                "step" if args.len() == 2 => Some(if args[1] < args[0] { 0.0 } else { 1.0 }),
                "smoothstep" if args.len() == 3 => {
                    let (edge0, edge1, x) = (args[0], args[1], args[2]);
                    let t = ((x - edge0) / (edge1 - edge0)).clamp(0.0, 1.0);
                    Some(t * t * (3.0 - 2.0 * t))
                }
                "lerp" | "mix" if args.len() == 3 => {
                    let (a, b, t) = (args[0], args[1], args[2]);
                    Some(a + (b - a) * t)
                }
                "clamp" if args.len() == 3 => {
                    let (x, min, max) = (args[0], args[1], args[2]);
                    Some(x.clamp(min, max))
                }
                "fract" if args.len() == 1 => Some(args[0].fract()),
                "sign" if args.len() == 1 => Some(args[0].signum()),
                "trunc" if args.len() == 1 => Some(args[0].trunc()),

                // User variables are handled by fasteval's variable system
                _ => None,
            }
        };

        self.compiled
            .eval(&self.slab, &mut ns)
            .unwrap_or(0.0)
    }

    /// Evaluate to a material index (clamped to u8 range)
    pub fn eval_material(&self, x: f64, y: f64, z: f64, ctx: &EvalContext) -> u8 {
        let value = self.eval(x, y, z, ctx);
        value.round().clamp(0.0, 255.0) as u8
    }

    /// Get the fasteval expression string (for debugging)
    pub fn expr_string(&self) -> &str {
        &self.expr_string
    }
}

/// Convert AST to fasteval expression string
fn ast_to_fasteval(expr: &Expr) -> Result<String, CpuCompileError> {
    match expr {
        Expr::Number(n) => Ok(format!("{}", n)),

        Expr::Var(var) => Ok(var.name().to_string()),

        Expr::UserVar(name) => Ok(name.clone()),

        Expr::BinOp { op, left, right } => {
            let left_str = ast_to_fasteval(left)?;
            let right_str = ast_to_fasteval(right)?;
            let op_str = match op {
                BinOpKind::Add => "+",
                BinOpKind::Sub => "-",
                BinOpKind::Mul => "*",
                BinOpKind::Div => "/",
                BinOpKind::Mod => "%",
                BinOpKind::Pow => "^",
                BinOpKind::Lt => "<",
                BinOpKind::Le => "<=",
                BinOpKind::Gt => ">",
                BinOpKind::Ge => ">=",
                BinOpKind::Eq => "==",
                BinOpKind::Ne => "!=",
                BinOpKind::And => "&&",
                BinOpKind::Or => "||",
            };
            Ok(format!("({} {} {})", left_str, op_str, right_str))
        }

        Expr::UnaryOp { op, expr } => {
            let expr_str = ast_to_fasteval(expr)?;
            match op {
                UnaryOpKind::Neg => Ok(format!("(-{})", expr_str)),
                UnaryOpKind::Not => Ok(format!("(1 - {})", expr_str)), // Boolean not as 1-x
            }
        }

        Expr::Call { func, args } => {
            let arg_strs: Vec<String> = args
                .iter()
                .map(ast_to_fasteval)
                .collect::<Result<_, _>>()?;

            let func_name = match func {
                // These are handled as custom functions in the namespace
                BuiltinFunc::Noise => "noise",
                BuiltinFunc::Fbm => "fbm",
                BuiltinFunc::Turbulence => "turbulence",
                BuiltinFunc::Step => "step",
                BuiltinFunc::Smoothstep => "smoothstep",
                BuiltinFunc::Lerp => "lerp",
                BuiltinFunc::Clamp => "clamp",
                BuiltinFunc::Fract => "fract",
                BuiltinFunc::Sign => "sign",
                BuiltinFunc::Trunc => "trunc",

                // Fasteval built-ins
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
                BuiltinFunc::Ln => "log", // fasteval uses 'log' for natural log
                BuiltinFunc::Log2 => "log2",
                BuiltinFunc::Log10 => "log10",
                BuiltinFunc::Floor => "floor",
                BuiltinFunc::Ceil => "ceil",
                BuiltinFunc::Round => "round",
                BuiltinFunc::Abs => "abs",
                BuiltinFunc::Min => "min",
                BuiltinFunc::Max => "max",
            };

            Ok(format!("{}({})", func_name, arg_strs.join(", ")))
        }

        Expr::If {
            cond,
            then_expr,
            else_expr,
        } => {
            // Convert to fasteval ternary: if(cond, then, else)
            let cond_str = ast_to_fasteval(cond)?;
            let then_str = ast_to_fasteval(then_expr)?;
            let else_str = ast_to_fasteval(else_expr)?;
            // fasteval uses: cond ? then : else (but we use a workaround)
            // Actually, fasteval doesn't have ternary. We use: then*cond + else*(1-cond)
            // But this evaluates both branches. For proper short-circuit, we'd need
            // to handle this differently. For now, use the multiplication approach.
            Ok(format!(
                "(({}) * ({}) + ({}) * (1 - ({})))",
                then_str, cond_str, else_str, cond_str
            ))
        }

        Expr::Let { name, value, body } => {
            // Inline the value by substituting it in the body
            // This evaluates the value expression once for each use, which may be
            // inefficient for complex expressions but is semantically correct
            let value_str = ast_to_fasteval(value)?;
            let body = substitute_user_var(body, name, &Expr::UserVar(format!("__let_{}", name)));
            let body_str = ast_to_fasteval(&body)?;
            // Replace the placeholder with the actual value
            Ok(body_str.replace(&format!("__let_{}", name), &format!("({})", value_str)))
        }

        Expr::Match {
            expr,
            cases,
            default,
        } => {
            // Convert match to nested ternaries
            let expr_str = ast_to_fasteval(expr)?;
            let default_str = ast_to_fasteval(default)?;

            let mut result = default_str;
            for (pattern, case_expr) in cases.iter().rev() {
                let case_str = ast_to_fasteval(case_expr)?;
                let cond_str = match pattern {
                    MatchPattern::Number(n) => format!("({} == {})", expr_str, n),
                    MatchPattern::Range { low, high } => {
                        format!("(({} >= {}) && ({} < {}))", expr_str, low, expr_str, high)
                    }
                };
                result = format!(
                    "(({}) * ({}) + ({}) * (1 - ({})))",
                    case_str, cond_str, result, cond_str
                );
            }
            Ok(result)
        }
    }
}

/// Substitute a user variable in an expression with another expression
fn substitute_user_var(expr: &Expr, name: &str, replacement: &Expr) -> Expr {
    match expr {
        Expr::UserVar(n) if n == name => replacement.clone(),
        Expr::UserVar(_) | Expr::Number(_) | Expr::Var(_) => expr.clone(),
        Expr::BinOp { op, left, right } => Expr::BinOp {
            op: *op,
            left: Box::new(substitute_user_var(left, name, replacement)),
            right: Box::new(substitute_user_var(right, name, replacement)),
        },
        Expr::UnaryOp { op, expr: e } => Expr::UnaryOp {
            op: *op,
            expr: Box::new(substitute_user_var(e, name, replacement)),
        },
        Expr::Call { func, args } => Expr::Call {
            func: *func,
            args: args
                .iter()
                .map(|a| substitute_user_var(a, name, replacement))
                .collect(),
        },
        Expr::If {
            cond,
            then_expr,
            else_expr,
        } => Expr::If {
            cond: Box::new(substitute_user_var(cond, name, replacement)),
            then_expr: Box::new(substitute_user_var(then_expr, name, replacement)),
            else_expr: Box::new(substitute_user_var(else_expr, name, replacement)),
        },
        Expr::Let {
            name: n,
            value,
            body,
        } => {
            if n == name {
                // Shadowing: don't substitute in body
                Expr::Let {
                    name: n.clone(),
                    value: Box::new(substitute_user_var(value, name, replacement)),
                    body: body.clone(),
                }
            } else {
                Expr::Let {
                    name: n.clone(),
                    value: Box::new(substitute_user_var(value, name, replacement)),
                    body: Box::new(substitute_user_var(body, name, replacement)),
                }
            }
        }
        Expr::Match {
            expr: e,
            cases,
            default,
        } => Expr::Match {
            expr: Box::new(substitute_user_var(e, name, replacement)),
            cases: cases
                .iter()
                .map(|(pat, case_expr)| {
                    (pat.clone(), substitute_user_var(case_expr, name, replacement))
                })
                .collect(),
            default: Box::new(substitute_user_var(default, name, replacement)),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::function::parse_expr;

    #[test]
    fn test_compile_simple() {
        let ast = parse_expr("x + 1").unwrap();
        let func = CpuFunction::compile(&ast).unwrap();
        let ctx = EvalContext::default();

        assert!((func.eval(0.0, 0.0, 0.0, &ctx) - 1.0).abs() < 0.001);
        assert!((func.eval(0.5, 0.0, 0.0, &ctx) - 1.5).abs() < 0.001);
    }

    #[test]
    fn test_compile_trig() {
        let ast = parse_expr("sin(x * 3.14159)").unwrap();
        let func = CpuFunction::compile(&ast).unwrap();
        let ctx = EvalContext::default();

        assert!((func.eval(0.0, 0.0, 0.0, &ctx) - 0.0).abs() < 0.001);
        assert!((func.eval(0.5, 0.0, 0.0, &ctx) - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_compile_conditional() {
        let ast = parse_expr("if x > 0 then 10 else 5").unwrap();
        let func = CpuFunction::compile(&ast).unwrap();
        let ctx = EvalContext::default();

        assert!((func.eval(1.0, 0.0, 0.0, &ctx) - 10.0).abs() < 0.001);
        assert!((func.eval(-1.0, 0.0, 0.0, &ctx) - 5.0).abs() < 0.001);
    }

    #[test]
    fn test_compile_noise() {
        let ast = parse_expr("noise(x, y, z)").unwrap();
        let func = CpuFunction::compile(&ast).unwrap();
        let ctx = EvalContext::new(0.0, 0, 42);

        // Noise should return values in a reasonable range
        let v = func.eval(0.5, 0.5, 0.5, &ctx);
        assert!(v >= -1.0 && v <= 1.0, "Noise value {} out of range", v);

        // Same inputs should give same outputs (deterministic)
        let v2 = func.eval(0.5, 0.5, 0.5, &ctx);
        assert!((v - v2).abs() < 0.0001);
    }

    #[test]
    fn test_compile_material() {
        let ast = parse_expr("if y > 0 then STONE else GRASS").unwrap();
        let func = CpuFunction::compile(&ast).unwrap();
        let ctx = EvalContext::default();

        assert_eq!(func.eval_material(0.0, 1.0, 0.0, &ctx), 1); // STONE
        assert_eq!(func.eval_material(0.0, -1.0, 0.0, &ctx), 2); // GRASS
    }

    #[test]
    fn test_compile_complex() {
        let ast =
            parse_expr("if noise(x * 0.1, y * 0.1, z * 0.1) > 0 then STONE else AIR").unwrap();
        let func = CpuFunction::compile(&ast).unwrap();
        let ctx = EvalContext::new(0.0, 0, 123);

        // Should compile and evaluate without error
        let _ = func.eval(0.5, 0.5, 0.5, &ctx);
    }

    #[test]
    fn test_uses_time() {
        let no_time = parse_expr("x + y").unwrap();
        let func = CpuFunction::compile(&no_time).unwrap();
        assert!(!func.uses_time);

        let with_time = parse_expr("x + time").unwrap();
        let func = CpuFunction::compile(&with_time).unwrap();
        assert!(func.uses_time);
    }

    #[test]
    fn test_uses_noise() {
        let no_noise = parse_expr("x + y").unwrap();
        let func = CpuFunction::compile(&no_noise).unwrap();
        assert!(!func.uses_noise);

        let with_noise = parse_expr("noise(x, y, z)").unwrap();
        let func = CpuFunction::compile(&with_noise).unwrap();
        assert!(func.uses_noise);
    }
}
