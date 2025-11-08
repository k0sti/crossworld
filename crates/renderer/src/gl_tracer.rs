use crate::renderer::Renderer;
use glow::*;

// Vertex shader - simple fullscreen quad
const VERTEX_SHADER_SOURCE: &str = r#"#version 300 es
precision highp float;

const vec2 positions[3] = vec2[3](
    vec2(-1.0, -1.0),
    vec2(3.0, -1.0),
    vec2(-1.0, 3.0)
);

void main() {
    gl_Position = vec4(positions[gl_VertexID], 0.0, 1.0);
}
"#;

// Fragment shader - raytracing a cube with directional lighting
const FRAGMENT_SHADER_SOURCE: &str = r#"#version 300 es
precision highp float;

out vec4 FragColor;

uniform vec2 u_resolution;
uniform float u_time;

// Ray structure
struct Ray {
    vec3 origin;
    vec3 direction;
};

// Hit information
struct HitInfo {
    bool hit;
    float t;
    vec3 point;
    vec3 normal;
};

// Ray-box intersection
HitInfo intersectBox(Ray ray, vec3 boxMin, vec3 boxMax) {
    HitInfo hitInfo;
    hitInfo.hit = false;
    hitInfo.t = 1e10;

    vec3 invDir = 1.0 / ray.direction;
    vec3 tMin = (boxMin - ray.origin) * invDir;
    vec3 tMax = (boxMax - ray.origin) * invDir;

    vec3 t1 = min(tMin, tMax);
    vec3 t2 = max(tMin, tMax);

    float tNear = max(max(t1.x, t1.y), t1.z);
    float tFar = min(min(t2.x, t2.y), t2.z);

    if (tNear > tFar || tFar < 0.0) {
        return hitInfo;
    }

    hitInfo.hit = true;
    hitInfo.t = tNear > 0.0 ? tNear : tFar;
    hitInfo.point = ray.origin + ray.direction * hitInfo.t;

    // Calculate normal
    vec3 center = (boxMin + boxMax) * 0.5;
    vec3 localPoint = hitInfo.point - center;
    vec3 size = (boxMax - boxMin) * 0.5;
    vec3 d = abs(localPoint / size);

    float maxComponent = max(max(d.x, d.y), d.z);
    if (abs(maxComponent - d.x) < 0.0001) {
        hitInfo.normal = vec3(sign(localPoint.x), 0.0, 0.0);
    } else if (abs(maxComponent - d.y) < 0.0001) {
        hitInfo.normal = vec3(0.0, sign(localPoint.y), 0.0);
    } else {
        hitInfo.normal = vec3(0.0, 0.0, sign(localPoint.z));
    }

    return hitInfo;
}

void main() {
    // Normalized pixel coordinates
    vec2 uv = (gl_FragCoord.xy - 0.5 * u_resolution) / u_resolution.y;

    // Camera setup
    vec3 cameraPos = vec3(3.0 * cos(u_time * 0.3), 2.0, 3.0 * sin(u_time * 0.3));
    vec3 target = vec3(0.0, 0.0, 0.0);
    vec3 up = vec3(0.0, 1.0, 0.0);

    vec3 forward = normalize(target - cameraPos);
    vec3 right = normalize(cross(forward, up));
    vec3 camUp = cross(right, forward);

    // Create ray
    Ray ray;
    ray.origin = cameraPos;
    ray.direction = normalize(forward + uv.x * right + uv.y * camUp);

    // Cube bounds
    vec3 boxMin = vec3(-1.0, -1.0, -1.0);
    vec3 boxMax = vec3(1.0, 1.0, 1.0);

    // Intersect with cube
    HitInfo hit = intersectBox(ray, boxMin, boxMax);

    // Background color
    vec3 color = vec3(0.2, 0.3, 0.4);

    if (hit.hit) {
        // Directional light
        vec3 lightDir = normalize(vec3(0.5, 1.0, 0.3));

        // Diffuse lighting
        float diffuse = max(dot(hit.normal, lightDir), 0.0);

        // Ambient
        float ambient = 0.2;

        // Base cube color with some variation based on normal
        vec3 baseColor = vec3(0.8, 0.4, 0.2);
        baseColor = mix(baseColor, vec3(1.0, 0.6, 0.3), abs(hit.normal.x));
        baseColor = mix(baseColor, vec3(0.6, 0.8, 0.4), abs(hit.normal.y));
        baseColor = mix(baseColor, vec3(0.4, 0.5, 0.9), abs(hit.normal.z));

        // Combine lighting
        color = baseColor * (ambient + diffuse * 0.8);

        // Add some edge highlighting
        float fresnel = pow(1.0 - abs(dot(-ray.direction, hit.normal)), 3.0);
        color += vec3(0.1) * fresnel;
    }

    // Gamma correction
    color = pow(color, vec3(1.0 / 2.2));

    FragColor = vec4(color, 1.0);
}
"#;

pub struct GlCubeTracer {
    program: Program,
    vao: VertexArray,
    resolution_location: Option<UniformLocation>,
    time_location: Option<UniformLocation>,
}

impl GlCubeTracer {
    pub unsafe fn new(gl: &Context) -> Result<Self, String> {
        unsafe {
            // Create shader program
            let program = create_program(gl, VERTEX_SHADER_SOURCE, FRAGMENT_SHADER_SOURCE)?;

            // Create VAO (required for OpenGL core profile)
            let vao = gl
                .create_vertex_array()
                .map_err(|e| format!("Failed to create VAO: {}", e))?;

            // Get uniform locations
            let resolution_location = gl.get_uniform_location(program, "u_resolution");
            let time_location = gl.get_uniform_location(program, "u_time");

            Ok(Self {
                program,
                vao,
                resolution_location,
                time_location,
            })
        }
    }

    pub unsafe fn render_to_gl(&self, gl: &Context, width: i32, height: i32, time: f32) {
        unsafe {
            gl.clear_color(0.0, 0.0, 0.0, 1.0);
            gl.clear(COLOR_BUFFER_BIT);

            gl.use_program(Some(self.program));
            gl.bind_vertex_array(Some(self.vao));

            // Set uniforms
            if let Some(loc) = &self.resolution_location {
                gl.uniform_2_f32(Some(loc), width as f32, height as f32);
            }
            if let Some(loc) = &self.time_location {
                gl.uniform_1_f32(Some(loc), time);
            }

            // Draw fullscreen triangle
            gl.draw_arrays(TRIANGLES, 0, 3);
        }
    }

    pub unsafe fn destroy(&self, gl: &Context) {
        unsafe {
            gl.delete_program(self.program);
            gl.delete_vertex_array(self.vao);
        }
    }
}

impl Renderer for GlCubeTracer {
    fn render(&mut self, _width: u32, _height: u32, _time: f32) {
        // Note: GL rendering is handled by render_to_gl in the app loop
        // This is here to satisfy the trait, but actual rendering needs GL context
    }

    fn name(&self) -> &str {
        "GlCubeTracer"
    }
}

unsafe fn create_program(
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

unsafe fn compile_shader(gl: &Context, shader_type: u32, source: &str) -> Result<Shader, String> {
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
