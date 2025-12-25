//! GPU backend for function expression evaluation
//!
//! Uses WebGPU (wgpu) compute shaders to evaluate expressions in parallel.
//! WGSL code is generated from the AST and executed on the GPU for massive parallelism.

mod wgsl;

use thiserror::Error;

use super::ast::Expr;

pub use wgsl::WgslCodegen;

/// Errors that can occur during GPU compilation
#[derive(Debug, Error)]
pub enum GpuCompileError {
    #[error("WGSL code generation error: {0}")]
    CodegenError(String),

    #[error("Unsupported expression: {0}")]
    UnsupportedExpr(String),
}

/// Compiled GPU function for expression evaluation
#[derive(Debug)]
pub struct GpuFunction {
    /// The generated WGSL shader source code
    shader_source: String,
    /// Whether the expression uses time
    pub uses_time: bool,
    /// Whether the expression uses noise functions
    pub uses_noise: bool,
    /// Estimated complexity
    pub complexity: u32,
}

impl GpuFunction {
    /// Compile an AST expression to GPU-evaluable WGSL shader
    pub fn compile(ast: &Expr) -> Result<Self, GpuCompileError> {
        // Collect info about the expression
        let uses_time = ast.uses_time();
        let uses_noise = ast.uses_noise();
        let complexity = ast.estimate_complexity();

        // Generate WGSL shader source
        let shader_source = WgslCodegen::generate(ast)?;

        Ok(Self {
            shader_source,
            uses_time,
            uses_noise,
            complexity,
        })
    }

    /// Get the generated WGSL shader source (for debugging or manual pipeline creation)
    pub fn shader_source(&self) -> &str {
        &self.shader_source
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::function::parse_expr;

    #[test]
    fn test_compile_simple() {
        let ast = parse_expr("x + 1").unwrap();
        let func = GpuFunction::compile(&ast).unwrap();
        assert!(func.shader_source().contains("pos.x"));
    }

    #[test]
    fn test_compile_noise() {
        let ast = parse_expr("noise(x, y, z)").unwrap();
        let func = GpuFunction::compile(&ast).unwrap();
        assert!(func.uses_noise);
        assert!(func.shader_source().contains("noise3"));
    }

    #[test]
    fn test_compile_time() {
        let ast = parse_expr("x + time").unwrap();
        let func = GpuFunction::compile(&ast).unwrap();
        assert!(func.uses_time);
        assert!(func.shader_source().contains("uniforms.time"));
    }
}
