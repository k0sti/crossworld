//! Function module - Expression-based procedural voxel generation
//!
//! This module provides a dual-backend system for evaluating mathematical expressions
//! to generate voxel materials dynamically. Expressions compile to a shared AST that
//! targets both CPU (fasteval) and GPU (WGSL compute shaders) backends.
//!
//! # Architecture
//!
//! ```text
//!                     Expression String
//!                     "if noise(x,y,z) > 0.5 then GRASS else STONE"
//!                            │
//!                            ▼
//!                     ┌─────────────┐
//!                     │   Parser    │  (nom-based)
//!                     │   → AST     │
//!                     └─────────────┘
//!                            │
//!               ┌────────────┴────────────┐
//!               ▼                         ▼
//!     ┌─────────────────┐       ┌─────────────────┐
//!     │  CPU Backend    │       │  GPU Backend    │
//!     │  (fasteval)     │       │  (WGSL)         │
//!     └─────────────────┘       └─────────────────┘
//! ```
//!
//! # Expression Language
//!
//! ## Variables
//! - `x`, `y`, `z` - Position coordinates in [-1, 1] range
//! - `wx`, `wy`, `wz` - World position coordinates
//! - `time` - Animation time in seconds
//! - `depth` - Current octree depth
//! - `seed` - Random seed for noise functions
//!
//! ## Operators
//! - Arithmetic: `+`, `-`, `*`, `/`, `%`, `^` (power)
//! - Comparison: `<`, `<=`, `>`, `>=`, `==`, `!=`
//! - Logical: `and`, `or`, `not`
//!
//! ## Functions
//! - Math: `sin`, `cos`, `tan`, `asin`, `acos`, `atan`, `atan2`, `sqrt`, `abs`,
//!   `floor`, `ceil`, `round`, `min`, `max`, `clamp`, `lerp`, `smoothstep`
//! - Noise: `noise(x,y,z)`, `fbm(x,y,z,octaves)`, `turbulence(x,y,z,octaves)`
//!
//! ## Control Flow
//! - Conditionals: `if cond then a else b`
//! - Match expressions: `match expr { pat => val, _ => default }`
//! - Let bindings: `let name = expr; body`
//!
//! ## Constants
//! - Material names: `AIR`, `STONE`, `GRASS`, `DIRT`, etc.
//! - Math constants: `PI`, `E`, `TAU`
//!
//! # Example
//!
//! ```ignore
//! use cube::function::{parse_expr, CpuFunction, EvalContext};
//!
//! let expr = parse_expr("if noise(x * 0.1, y * 0.1, z * 0.1) > 0.5 then STONE else AIR")?;
//! let cpu_fn = CpuFunction::compile(&expr)?;
//!
//! let ctx = EvalContext::new(0.0, 3, 42);
//! let material = cpu_fn.eval(0.5, 0.0, 0.5, &ctx);
//! ```

mod ast;
mod cpu;
mod dynamic_cube;
mod gpu;
mod parser;

#[cfg(test)]
mod tests;

pub use ast::{BinOpKind, BuiltinFunc, Expr, MatchPattern, UnaryOpKind, VarId};
pub use cpu::{CpuFunction, EvalContext};
pub use dynamic_cube::{CachedCube, DynamicCube};
pub use gpu::{GpuCompileError, GpuFunction, WgslCodegen};
pub use parser::{parse_expr, ParseError};

/// Compile an expression string to a CPU-evaluable function
pub fn compile(source: &str) -> Result<CpuFunction, CompileError> {
    let ast = parse_expr(source)?;
    let cpu_fn = CpuFunction::compile(&ast)?;
    Ok(cpu_fn)
}

/// Errors that can occur during compilation
#[derive(Debug, thiserror::Error)]
pub enum CompileError {
    #[error("Parse error: {0}")]
    Parse(#[from] ParseError),

    #[error("CPU compile error: {0}")]
    CpuCompile(#[from] cpu::CpuCompileError),

    #[error("GPU compile error: {0}")]
    GpuCompile(#[from] gpu::GpuCompileError),
}

/// Compile an expression string to a GPU-evaluable function
pub fn compile_gpu(source: &str) -> Result<GpuFunction, CompileError> {
    let ast = parse_expr(source)?;
    let gpu_fn = GpuFunction::compile(&ast)?;
    Ok(gpu_fn)
}
