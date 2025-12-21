# Proposal: Add Function Cube Type

## Summary

Add a new `Cube::Function` variant that evaluates material values dynamically from user-defined expressions. Expressions compile to a shared AST that can target both **CPU (fasteval)** and **GPU (WGSL compute shaders)** backends, enabling massive parallelism for procedural content generation.

## Motivation

The current cube system supports:
- `Cube::Solid(T)` - Static material values
- `Cube::Cubes/Quad/Layers` - Spatial subdivision
- `Cube<Quat>` via Fabric system - Quaternion-based procedural fields

**Gap**: No mechanism for user-defined procedural material generation with:
- Mathematical expressions (sin, cos, noise, conditionals)
- Runtime parameters (time, position, custom variables)
- GPU acceleration for real-time evaluation

**Use cases**:
- Animated terrain (flowing water, pulsing crystals)
- Procedural textures/patterns (stripes, gradients, noise-based)
- Dynamic material transitions (day/night cycle effects)
- User-generated content with custom formulas
- Real-time terrain generation on GPU

## Design Approach: Dual Backend Architecture

```
                    Expression String
                    "if noise(x,y,z) > 0.5 then GRASS else STONE"
                           │
                           ▼
                    ┌─────────────┐
                    │   Parser    │  (nom-based, shared)
                    │   → AST     │
                    └─────────────┘
                           │
              ┌────────────┴────────────┐
              ▼                         ▼
    ┌─────────────────┐       ┌─────────────────┐
    │  CPU Backend    │       │  GPU Backend    │
    │                 │       │                 │
    │  fasteval       │       │  WGSL codegen   │
    │  expression     │       │  + compute      │
    │  compiler       │       │  pipeline       │
    └─────────────────┘       └─────────────────┘
              │                         │
              ▼                         ▼
        Sequential              Parallel eval
        evaluation              (8x8x8 workgroups)
        ~1M voxels/sec          ~100M+ voxels/sec
```

### Why Dual Backends?

| Scenario | Best Backend | Reason |
|----------|--------------|--------|
| Small edits (< 1000 voxels) | CPU | GPU dispatch overhead not worth it |
| Initial world generation | GPU | Millions of voxels, massive parallelism |
| Time-animated expressions | GPU | Continuous re-evaluation needed |
| Complex noise (FBM, turbulence) | GPU | Compute-intensive |
| Simple gradients | CPU | Fast enough, simpler |
| WebGL 1.0 fallback | CPU | No compute shaders |

### Expression Language

Designed for GPU compatibility - all constructs map to WGSL:

```
// Simple gradient
x * 0.5 + y * 0.3

// Wave pattern
sin(x * 3.14) * 0.5 + 0.5

// Conditional (compiles to WGSL if/else)
if noise(x, y, z) > 0.5 then GRASS else STONE

// Match expression (compiles to if-chain in WGSL)
match floor(y * 4) {
  0 => BEDROCK,
  1 => STONE,
  2 => DIRT,
  _ => GRASS
}

// Let bindings (compiles to WGSL local variables)
let height = noise(x * 0.1, z * 0.1) * 10;
if y < height then
  if y < height - 2 then STONE else GRASS
else AIR
```

### Inputs Available to Functions

| Input | Type | WGSL Equivalent |
|-------|------|-----------------|
| `x`, `y`, `z` | f32 | `pos.x`, `pos.y`, `pos.z` |
| `wx`, `wy`, `wz` | f32 | `world_pos.x/y/z` |
| `time` | f32 | `uniforms.time` |
| `depth` | u32 | `uniforms.depth` |
| `noise(x,y,z)` | f32 | `noise3(vec3(x,y,z))` |
| `seed` | u32 | `uniforms.seed` |

### Supported Operations

All operations have direct WGSL equivalents:

**Arithmetic**: `+`, `-`, `*`, `/`, `%`, `pow(x,y)`
**Comparison**: `<`, `<=`, `>`, `>=`, `==`, `!=`
**Logic**: `and`, `or`, `not`
**Math functions**: `sin`, `cos`, `tan`, `asin`, `acos`, `atan`, `atan2`, `sqrt`, `abs`, `floor`, `ceil`, `round`, `min`, `max`, `clamp`, `lerp` (mix), `smoothstep`
**Noise**: `noise(x,y,z)`, `fbm(x,y,z,octaves)`, `turbulence(x,y,z,octaves)`
**Control flow**: `if cond then a else b`, `match expr { pat => val, ... }`
**Variables**: `let name = expr; body`
**Constants**: Material names (`STONE`, `GRASS`, etc.), `PI`, `E`

## CPU Backend: fasteval

Use [fasteval](https://docs.rs/fasteval/latest/fasteval/) for CPU evaluation:

```rust
use fasteval::{Compiler, Evaler, Instruction};

pub struct CpuFunction {
    compiled: fasteval::Instruction,
    slab: fasteval::Slab,
    uses_time: bool,
    uses_noise: bool,
}

impl CpuFunction {
    pub fn compile(source: &str) -> Result<Self, CompileError> {
        let parser = fasteval::Parser::new();
        let mut slab = fasteval::Slab::new();
        let mut ns = Namespace::new();

        // Register custom functions
        ns.insert("noise", |args| noise3(args[0], args[1], args[2]));
        ns.insert("fbm", |args| fbm(args[0], args[1], args[2], args[3] as u8));

        let compiled = parser.parse(source, &mut ns)?.compile(&ns, &mut slab);

        Ok(Self { compiled, slab, uses_time: source.contains("time"), uses_noise: source.contains("noise") })
    }

    pub fn eval(&self, ctx: &EvalContext) -> u8 {
        let mut ns = self.create_namespace(ctx);
        let result = self.compiled.eval(&ns, &mut self.slab.clone()).unwrap_or(0.0);
        result.clamp(0.0, 255.0) as u8
    }
}
```

**fasteval provides:**
- Compiled expressions (10x faster than interpreted)
- Custom functions (noise, fbm, etc.)
- All standard math functions
- Ternary operator for conditionals

**We add:**
- Wrapper for `if/then/else` syntax → ternary conversion
- `match` → nested ternary conversion
- `let` → inline substitution or fasteval's variable system

## GPU Backend: WGSL Compute Shaders

Generate WGSL code from the same AST:

```rust
pub struct GpuFunction {
    shader_source: String,
    pipeline: wgpu::ComputePipeline,
    uses_time: bool,
}

impl GpuFunction {
    pub fn compile(ast: &Expr, device: &wgpu::Device) -> Result<Self, CompileError> {
        let wgsl = WgslCodegen::generate(ast)?;
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("function_cube_shader"),
            source: wgpu::ShaderSource::Wgsl(wgsl.into()),
        });
        // ... create pipeline
    }

    pub fn eval_batch(&self, positions: &[Vec3], time: f32) -> Vec<u8> {
        // Dispatch compute shader, read back results
    }
}
```

**Example WGSL output:**

```wgsl
// Noise implementation (included in all shaders)
fn hash(p: vec3<f32>) -> f32 {
    let h = dot(p, vec3(127.1, 311.7, 74.7));
    return fract(sin(h) * 43758.5453);
}

fn noise3(p: vec3<f32>) -> f32 {
    let i = floor(p);
    let f = fract(p);
    let u = f * f * (3.0 - 2.0 * f);
    // ... standard 3D noise implementation
}

fn fbm(p: vec3<f32>, octaves: u32) -> f32 {
    var value = 0.0;
    var amplitude = 0.5;
    var pos = p;
    for (var i = 0u; i < octaves; i++) {
        value += amplitude * noise3(pos);
        pos *= 2.0;
        amplitude *= 0.5;
    }
    return value;
}

// User expression compiled to WGSL
struct Uniforms {
    time: f32,
    depth: u32,
    seed: u32,
    world_offset: vec3<f32>,
}

@group(0) @binding(0) var<uniform> uniforms: Uniforms;
@group(0) @binding(1) var<storage, read_write> materials: array<u32>;

@compute @workgroup_size(8, 8, 8)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let size = 128u;  // cube resolution
    let pos = vec3<f32>(global_id) / f32(size) * 2.0 - 1.0;  // [-1, 1]
    let x = pos.x;
    let y = pos.y;
    let z = pos.z;
    let time = uniforms.time;

    // === COMPILED EXPRESSION ===
    // Source: "let height = noise(x * 0.1, z * 0.1, 0) * 10; if y < height then STONE else AIR"
    let height = noise3(vec3(x * 0.1, z * 0.1, 0.0)) * 10.0;
    var result: u32;
    if (y < height) {
        result = 20u;  // STONE
    } else {
        result = 0u;   // AIR
    }
    // === END EXPRESSION ===

    let idx = global_id.z * size * size + global_id.y * size + global_id.x;
    materials[idx] = result;
}
```

## Backend Selection Strategy

```rust
pub struct CompiledFunction {
    ast: Expr,
    cpu: CpuFunction,
    gpu: Option<GpuFunction>,  // None if WebGL 1.0 / no compute
    uses_time: bool,
    uses_noise: bool,
    complexity: u32,  // Estimated evaluation cost
}

impl CompiledFunction {
    pub fn eval_cube(&self, depth: u32, ctx: &EvalContext) -> Cube<u8> {
        let voxel_count = 8usize.pow(depth);

        // Heuristic: use GPU for large evaluations
        let use_gpu = self.gpu.is_some()
            && (voxel_count > 4096 || self.uses_time || self.complexity > 10);

        if use_gpu {
            self.eval_gpu(depth, ctx)
        } else {
            self.eval_cpu(depth, ctx)
        }
    }
}
```

## Integration with DynamicCube

```rust
pub enum DynamicCube {
    /// Static cube with precomputed materials
    Static(Cube<u8>),

    /// Function-based cube
    Function {
        function: Rc<CompiledFunction>,
        cache: RefCell<Option<CachedCube>>,
    },
}

impl DynamicCube {
    /// Materialize to static cube (uses GPU if available)
    pub fn materialize(&self, depth: u32, ctx: &EvalContext) -> Cube<u8> {
        match self {
            Self::Static(cube) => cube.clone(),
            Self::Function { function, cache } => {
                // Check cache for time-invariant functions
                if !function.uses_time {
                    if let Some(cached) = cache.borrow().as_ref() {
                        if cached.depth >= depth {
                            return cached.cube.clone();
                        }
                    }
                }

                let result = function.eval_cube(depth, ctx);

                // Cache if time-invariant
                if !function.uses_time {
                    *cache.borrow_mut() = Some(CachedCube { cube: result.clone(), depth });
                }

                result
            }
        }
    }
}
```

## Scope

### Phase 1: CPU Backend (MVP)
- Expression parser (shared AST)
- fasteval integration with custom noise functions
- DynamicCube type
- WASM bindings
- Basic mesh generation integration

### Phase 2: GPU Backend
- WGSL code generator from AST
- Compute shader pipeline setup
- GPU/CPU backend selection
- Batch evaluation API

### Phase 3: Optimization
- Expression complexity analysis
- Caching for time-invariant functions
- Incremental updates for time-varying
- LOD-aware evaluation

### Out of Scope (Future Work)
- JIT compilation to native code
- User-defined functions (beyond let bindings)
- Persistence format for compiled functions
- Visual expression editor

## Risks & Mitigations

| Risk | Mitigation |
|------|------------|
| fasteval limitations | We control the AST; can add preprocessor for unsupported syntax |
| WGSL codegen bugs | Extensive test suite comparing CPU vs GPU output |
| WebGL 1.0 fallback needed | CPU backend always available |
| Shader compilation latency | Cache compiled pipelines, compile async |
| GPU memory limits | Tile large cubes, process in chunks |

## Success Criteria

1. Parse and compile expression in < 1ms (CPU), < 50ms (GPU pipeline)
2. CPU: Evaluate 1M voxels/sec on single thread
3. GPU: Evaluate 100M+ voxels/sec
4. < 50KB WASM size increase (CPU only)
5. Works in browser (WASM + WebGPU) and native (renderer)
6. Graceful fallback when WebGPU unavailable

## References

- [fasteval](https://docs.rs/fasteval/latest/fasteval/) - Fast Rust expression evaluator
- [WGSL Specification](https://www.w3.org/TR/WGSL/) - WebGPU Shading Language
- [WebGPU Compute Shaders](https://webgpufundamentals.org/webgpu/lessons/webgpu-compute-shaders.html)
- [Simplex Noise in GLSL](https://gist.github.com/patriciogonzalezvivo/670c22f3966e662d2f83) - Noise implementations
- Existing Fabric system in `crates/cube/src/fabric/`
