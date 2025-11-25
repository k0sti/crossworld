//! Shared shader compilation utilities for OpenGL renderers

use glow::*;

/// Compile a shader from source code
///
/// # Safety
/// Requires an active OpenGL context
pub unsafe fn compile_shader(
    gl: &Context,
    shader_type: u32,
    source: &str,
) -> Result<Shader, String> {
    unsafe {
        let shader = gl.create_shader(shader_type).map_err(|e| e.to_string())?;

        gl.shader_source(shader, source);
        gl.compile_shader(shader);

        if !gl.get_shader_compile_status(shader) {
            let log = gl.get_shader_info_log(shader);
            return Err(format!("Shader compilation error: {}", log));
        }

        Ok(shader)
    }
}

/// Create and link a shader program from vertex and fragment shader sources
///
/// # Safety
/// Requires an active OpenGL context
pub unsafe fn create_program(
    gl: &Context,
    vertex_src: &str,
    fragment_src: &str,
) -> Result<Program, String> {
    unsafe {
        let program = gl.create_program().map_err(|e| e.to_string())?;

        let vertex_shader = compile_shader(gl, VERTEX_SHADER, vertex_src)?;
        let fragment_shader = compile_shader(gl, FRAGMENT_SHADER, fragment_src)?;

        gl.attach_shader(program, vertex_shader);
        gl.attach_shader(program, fragment_shader);
        gl.link_program(program);

        if !gl.get_program_link_status(program) {
            let log = gl.get_program_info_log(program);
            return Err(format!("Program link error: {}", log));
        }

        gl.detach_shader(program, vertex_shader);
        gl.detach_shader(program, fragment_shader);
        gl.delete_shader(vertex_shader);
        gl.delete_shader(fragment_shader);

        Ok(program)
    }
}

/// Create a compute shader program from source
///
/// # Safety
/// Requires an active OpenGL context with compute shader support (GL 4.3+)
#[allow(dead_code)]
pub unsafe fn create_compute_program(gl: &Context, compute_src: &str) -> Result<Program, String> {
    unsafe {
        let program = gl.create_program().map_err(|e| e.to_string())?;

        let compute_shader = compile_shader(gl, COMPUTE_SHADER, compute_src)?;

        gl.attach_shader(program, compute_shader);
        gl.link_program(program);

        if !gl.get_program_link_status(program) {
            let log = gl.get_program_info_log(program);
            return Err(format!("Compute program link error: {}", log));
        }

        gl.detach_shader(program, compute_shader);
        gl.delete_shader(compute_shader);

        Ok(program)
    }
}
