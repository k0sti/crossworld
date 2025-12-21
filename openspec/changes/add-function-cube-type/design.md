# Design: Function Cube Type

## Architecture Overview

```
┌─────────────────────────────────────────────────────────────┐
│                     Expression String                        │
│        "if noise(x,y,z) > 0.5 then GRASS else STONE"        │
└─────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────┐
│                      Parser (nom)                            │
│                                                              │
│  Tokenize → Parse → Build AST                               │
│  (shared between CPU and GPU backends)                      │
└─────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────┐
│                      AST (Expr enum)                         │
│                                                              │
│   IfElse {                                                   │
│     cond: BinOp(Gt, Call("noise", [Var(x),Var(y),Var(z)]),  │
│                     Const(0.5)),                             │
│     then_: Material(GRASS),                                  │
│     else_: Material(STONE)                                   │
│   }                                                          │
└─────────────────────────────────────────────────────────────┘
                              │
              ┌───────────────┴───────────────┐
              ▼                               ▼
┌──────────────────────────┐    ┌──────────────────────────┐
│     CPU Backend          │    │     GPU Backend          │
│                          │    │                          │
│  ┌────────────────────┐  │    │  ┌────────────────────┐  │
│  │  AST → fasteval    │  │    │  │  AST → WGSL       │  │
│  │  expression        │  │    │  │  source code      │  │
│  └────────────────────┘  │    │  └────────────────────┘  │
│           │              │    │           │              │
│           ▼              │    │           ▼              │
│  ┌────────────────────┐  │    │  ┌────────────────────┐  │
│  │  fasteval compile  │  │    │  │  WGSL compile     │  │
│  │  (RPN bytecode)    │  │    │  │  (GPU pipeline)   │  │
│  └────────────────────┘  │    │  └────────────────────┘  │
│           │              │    │           │              │
│           ▼              │    │           ▼              │
│  ┌────────────────────┐  │    │  ┌────────────────────┐  │
│  │  Sequential eval   │  │    │  │  Parallel dispatch │  │
│  │  ~1M voxels/sec    │  │    │  │  ~100M voxels/sec  │  │
│  └────────────────────┘  │    │  └────────────────────┘  │
└──────────────────────────┘    └──────────────────────────┘
              │                               │
              └───────────────┬───────────────┘
                              ▼
┌─────────────────────────────────────────────────────────────┐
│                    CompiledFunction                          │
│                                                              │
│   ast: Expr                                                  │
│   cpu: CpuFunction (fasteval)                               │
│   gpu: Option<GpuFunction> (WGSL pipeline)                  │
│   uses_time: bool                                            │
│   uses_noise: bool                                           │
│   complexity: u32                                            │
└─────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────┐
│                     DynamicCube                              │
│                                                              │
│   eval(coord, ctx) → u8  // auto-selects backend            │
│   materialize(depth, ctx) → Cube<u8>  // bulk eval          │
└─────────────────────────────────────────────────────────────┘
```

## Component Details

### 1. Expression Language Grammar

Designed for GPU compatibility - every construct maps to WGSL:

```ebnf
expr       = let_expr | if_expr | match_expr | or_expr ;
let_expr   = "let" IDENT "=" expr ";" expr ;
if_expr    = "if" expr "then" expr "else" expr ;
match_expr = "match" expr "{" match_arms "}" ;
match_arms = (pattern "=>" expr ",")* pattern "=>" expr ;
pattern    = "_" | NUMBER | IDENT ;

or_expr    = and_expr ("or" and_expr)* ;
and_expr   = comp_expr ("and" comp_expr)* ;
comp_expr  = add_expr (("<" | "<=" | ">" | ">=" | "==" | "!=") add_expr)? ;
add_expr   = mul_expr (("+" | "-") mul_expr)* ;
mul_expr   = unary_expr (("*" | "/" | "%") unary_expr)* ;
unary_expr = "-" unary_expr | "not" unary_expr | call_expr ;
call_expr  = IDENT "(" (expr ("," expr)*)? ")" | primary ;
primary    = NUMBER | IDENT | "(" expr ")" ;

NUMBER     = [0-9]+ ("." [0-9]+)? ;
IDENT      = [a-zA-Z_][a-zA-Z0-9_]* ;
```

### 2. AST Definition

```rust
/// Abstract syntax tree for function expressions
/// Designed to be compilable to both fasteval and WGSL
#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    /// Numeric constant
    Const(f32),

    /// Material constant (GRASS, STONE, etc.)
    Material(u8),

    /// Variable reference (x, y, z, time, depth, etc.)
    Var(VarId),

    /// Binary operation
    BinOp {
        op: BinOpKind,
        lhs: Box<Expr>,
        rhs: Box<Expr>,
    },

    /// Unary operation
    UnaryOp {
        op: UnaryOpKind,
        expr: Box<Expr>,
    },

    /// Function call (sin, cos, noise, etc.)
    Call {
        func: BuiltinFunc,
        args: Vec<Expr>,
    },

    /// Conditional expression
    IfElse {
        cond: Box<Expr>,
        then_: Box<Expr>,
        else_: Box<Expr>,
    },

    /// Pattern matching (compiles to if-chain for GPU)
    Match {
        expr: Box<Expr>,
        arms: Vec<MatchArm>,
    },

    /// Let binding (compiles to local variable)
    Let {
        name: String,
        value: Box<Expr>,
        body: Box<Expr>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VarId {
    X, Y, Z,                    // Normalized position [-1, 1]
    WorldX, WorldY, WorldZ,     // World coordinates
    Time,                       // Elapsed time
    Depth,                      // Octree depth
    Seed,                       // Random seed
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinOpKind {
    Add, Sub, Mul, Div, Mod, Pow,
    Lt, Le, Gt, Ge, Eq, Ne,
    And, Or,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnaryOpKind {
    Neg, Not,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BuiltinFunc {
    // Trigonometric
    Sin, Cos, Tan, ASin, ACos, ATan, ATan2,
    // Rounding
    Floor, Ceil, Round, Abs, Sign,
    // Math
    Sqrt, Pow, Exp, Log, Log2,
    // Interpolation
    Min, Max, Clamp, Lerp, Smoothstep,
    // Noise
    Noise, Fbm, Turbulence,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MatchArm {
    pub pattern: Pattern,
    pub expr: Expr,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Pattern {
    Wildcard,
    Const(f32),
    Range { min: f32, max: f32 },
}
```

### 3. CPU Backend (fasteval)

```rust
use fasteval::{Evaler, Compiler, Slab};

/// CPU function using fasteval for evaluation
pub struct CpuFunction {
    /// Original source for error messages
    source: String,
    /// Compiled fasteval expression
    compiled: fasteval::Instruction,
    /// fasteval memory slab
    slab: Slab,
    /// Parsed AST (shared with GPU backend)
    ast: Expr,
}

impl CpuFunction {
    /// Compile expression string to CPU function
    pub fn compile(source: &str) -> Result<Self, CompileError> {
        // First parse to our AST
        let ast = Parser::parse(source)?;

        // Convert AST to fasteval expression string
        // (handle if/then/else -> ternary, match -> nested ternary)
        let fasteval_source = AstToFasteval::convert(&ast)?;

        // Compile with fasteval
        let parser = fasteval::Parser::new();
        let mut slab = Slab::new();

        // Register custom functions
        let compiled = parser
            .parse(&fasteval_source, &mut slab.ps)?
            .from(&slab.ps)
            .compile(&slab.ps, &mut slab.cs, custom_functions);

        Ok(Self { source: source.to_string(), compiled, slab, ast })
    }

    /// Evaluate at single position
    pub fn eval(&mut self, ctx: &EvalContext) -> u8 {
        // Set up namespace with current context values
        let ns = |name: &str, _args: Vec<f64>| -> Option<f64> {
            match name {
                "x" => Some(ctx.position.x as f64),
                "y" => Some(ctx.position.y as f64),
                "z" => Some(ctx.position.z as f64),
                "wx" => Some(ctx.world_position.x as f64),
                "wy" => Some(ctx.world_position.y as f64),
                "wz" => Some(ctx.world_position.z as f64),
                "time" => Some(ctx.time as f64),
                "depth" => Some(ctx.depth as f64),
                "seed" => Some(ctx.seed as f64),
                "noise" => {
                    // Called as noise(x, y, z)
                    // args available in closure
                    None  // handled by custom function
                }
                _ => None,
            }
        };

        let result = self.compiled.eval(&self.slab, &mut ns).unwrap_or(0.0);
        result.clamp(0.0, 255.0) as u8
    }
}

/// Custom functions for fasteval (noise, fbm, etc.)
fn custom_functions(name: &str, args: Vec<f64>) -> Option<f64> {
    match name {
        "noise" if args.len() == 3 => {
            Some(noise::perlin3(args[0] as f32, args[1] as f32, args[2] as f32) as f64)
        }
        "fbm" if args.len() == 4 => {
            Some(noise::fbm(args[0] as f32, args[1] as f32, args[2] as f32, args[3] as u8) as f64)
        }
        "turbulence" if args.len() == 4 => {
            Some(noise::turbulence(args[0] as f32, args[1] as f32, args[2] as f32, args[3] as u8) as f64)
        }
        _ => None,
    }
}

/// Convert our AST to fasteval-compatible expression string
struct AstToFasteval;

impl AstToFasteval {
    fn convert(expr: &Expr) -> Result<String, CompileError> {
        match expr {
            Expr::Const(v) => Ok(format!("{}", v)),
            Expr::Material(id) => Ok(format!("{}", id)),
            Expr::Var(var) => Ok(Self::var_name(var).to_string()),

            Expr::BinOp { op, lhs, rhs } => {
                let l = Self::convert(lhs)?;
                let r = Self::convert(rhs)?;
                Ok(format!("({} {} {})", l, Self::binop_str(op), r))
            }

            Expr::UnaryOp { op, expr } => {
                let e = Self::convert(expr)?;
                match op {
                    UnaryOpKind::Neg => Ok(format!("(-{})", e)),
                    UnaryOpKind::Not => Ok(format!("(1 - {})", e)),  // fasteval has no 'not'
                }
            }

            Expr::Call { func, args } => {
                let args_str: Vec<String> = args.iter()
                    .map(Self::convert)
                    .collect::<Result<_, _>>()?;
                Ok(format!("{}({})", Self::func_name(func), args_str.join(", ")))
            }

            // if/then/else -> ternary using fasteval's custom syntax
            // fasteval supports: if(cond, then, else) as a function
            Expr::IfElse { cond, then_, else_ } => {
                let c = Self::convert(cond)?;
                let t = Self::convert(then_)?;
                let e = Self::convert(else_)?;
                // Use fasteval's conditional: if(cond > 0.5, then, else)
                Ok(format!("if({} > 0.5, {}, {})", c, t, e))
            }

            // match -> nested ternary
            Expr::Match { expr, arms } => {
                Self::convert_match(expr, arms)
            }

            // let -> inline substitution (fasteval doesn't have let)
            // For simplicity, we expand lets during AST conversion
            Expr::Let { name, value, body } => {
                // This requires substitution in body - complex
                // Alternative: use fasteval's variable system
                Err(CompileError::LetNotSupported)
            }
        }
    }

    fn convert_match(expr: &Expr, arms: &[MatchArm]) -> Result<String, CompileError> {
        // Convert match to nested if/else
        // match e { 0 => a, 1 => b, _ => c }
        // becomes: if(e == 0, a, if(e == 1, b, c))

        let e = Self::convert(expr)?;
        let mut result = String::new();

        for (i, arm) in arms.iter().enumerate() {
            let arm_expr = Self::convert(&arm.expr)?;

            match &arm.pattern {
                Pattern::Wildcard => {
                    // Last arm, no condition needed
                    result.push_str(&arm_expr);
                }
                Pattern::Const(val) => {
                    result.push_str(&format!("if({} == {}, {}, ", e, val, arm_expr));
                }
                Pattern::Range { min, max } => {
                    result.push_str(&format!(
                        "if({} >= {} && {} <= {}, {}, ",
                        e, min, e, max, arm_expr
                    ));
                }
            }
        }

        // Close all the if( parentheses
        let non_wildcard = arms.iter().filter(|a| !matches!(a.pattern, Pattern::Wildcard)).count();
        result.push_str(&")".repeat(non_wildcard));

        Ok(result)
    }

    fn var_name(var: &VarId) -> &'static str {
        match var {
            VarId::X => "x",
            VarId::Y => "y",
            VarId::Z => "z",
            VarId::WorldX => "wx",
            VarId::WorldY => "wy",
            VarId::WorldZ => "wz",
            VarId::Time => "time",
            VarId::Depth => "depth",
            VarId::Seed => "seed",
        }
    }

    fn binop_str(op: &BinOpKind) -> &'static str {
        match op {
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
        }
    }

    fn func_name(func: &BuiltinFunc) -> &'static str {
        match func {
            BuiltinFunc::Sin => "sin",
            BuiltinFunc::Cos => "cos",
            BuiltinFunc::Tan => "tan",
            BuiltinFunc::ASin => "asin",
            BuiltinFunc::ACos => "acos",
            BuiltinFunc::ATan => "atan",
            BuiltinFunc::ATan2 => "atan2",
            BuiltinFunc::Floor => "floor",
            BuiltinFunc::Ceil => "ceil",
            BuiltinFunc::Round => "round",
            BuiltinFunc::Abs => "abs",
            BuiltinFunc::Sign => "sign",
            BuiltinFunc::Sqrt => "sqrt",
            BuiltinFunc::Pow => "pow",
            BuiltinFunc::Exp => "exp",
            BuiltinFunc::Log => "log",
            BuiltinFunc::Log2 => "log2",
            BuiltinFunc::Min => "min",
            BuiltinFunc::Max => "max",
            BuiltinFunc::Clamp => "clamp",
            BuiltinFunc::Lerp => "lerp",
            BuiltinFunc::Smoothstep => "smoothstep",
            BuiltinFunc::Noise => "noise",
            BuiltinFunc::Fbm => "fbm",
            BuiltinFunc::Turbulence => "turbulence",
        }
    }
}
```

### 4. GPU Backend (WGSL)

```rust
/// GPU function using WGSL compute shaders
pub struct GpuFunction {
    /// Generated WGSL source code
    wgsl_source: String,
    /// Compiled compute pipeline
    pipeline: wgpu::ComputePipeline,
    /// Bind group layout
    bind_group_layout: wgpu::BindGroupLayout,
    /// Whether this function uses time (needs re-dispatch each frame)
    uses_time: bool,
}

impl GpuFunction {
    /// Compile AST to GPU function
    pub fn compile(
        ast: &Expr,
        device: &wgpu::Device,
    ) -> Result<Self, CompileError> {
        // Generate WGSL source
        let wgsl_source = WgslCodegen::generate(ast)?;

        // Create shader module
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("function_cube_shader"),
            source: wgpu::ShaderSource::Wgsl(wgsl_source.clone().into()),
        });

        // Create bind group layout
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("function_cube_bind_group_layout"),
            entries: &[
                // Uniforms (time, depth, seed, world_offset)
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                // Output materials buffer
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: false },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });

        // Create pipeline
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("function_cube_pipeline_layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("function_cube_pipeline"),
            layout: Some(&pipeline_layout),
            module: &shader,
            entry_point: Some("main"),
            compilation_options: Default::default(),
            cache: None,
        });

        let uses_time = ast.contains_var(VarId::Time);

        Ok(Self { wgsl_source, pipeline, bind_group_layout, uses_time })
    }

    /// Evaluate cube at given depth, returning flat array of materials
    pub fn eval_batch(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        size: u32,
        uniforms: &Uniforms,
    ) -> Vec<u8> {
        let total_voxels = (size * size * size) as usize;

        // Create uniform buffer
        let uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("uniforms"),
            contents: bytemuck::bytes_of(uniforms),
            usage: wgpu::BufferUsages::UNIFORM,
        });

        // Create output buffer
        let output_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("output"),
            size: (total_voxels * 4) as u64,  // u32 per voxel
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });

        // Create staging buffer for readback
        let staging_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("staging"),
            size: (total_voxels * 4) as u64,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // Create bind group
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("function_cube_bind_group"),
            layout: &self.bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: uniform_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: output_buffer.as_entire_binding(),
                },
            ],
        });

        // Dispatch compute shader
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("function_cube_encoder"),
        });

        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("function_cube_pass"),
                timestamp_writes: None,
            });
            pass.set_pipeline(&self.pipeline);
            pass.set_bind_group(0, &bind_group, &[]);
            // Dispatch with 8x8x8 workgroups
            let workgroups = (size + 7) / 8;
            pass.dispatch_workgroups(workgroups, workgroups, workgroups);
        }

        encoder.copy_buffer_to_buffer(&output_buffer, 0, &staging_buffer, 0, (total_voxels * 4) as u64);

        queue.submit(Some(encoder.finish()));

        // Read back results
        let slice = staging_buffer.slice(..);
        slice.map_async(wgpu::MapMode::Read, |_| {});
        device.poll(wgpu::Maintain::Wait);

        let data = slice.get_mapped_range();
        let materials: Vec<u8> = bytemuck::cast_slice::<u8, u32>(&data)
            .iter()
            .map(|&v| v as u8)
            .collect();

        drop(data);
        staging_buffer.unmap();

        materials
    }
}

/// Uniforms passed to GPU shader
#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Uniforms {
    pub time: f32,
    pub depth: u32,
    pub seed: u32,
    pub _padding: u32,
    pub world_offset: [f32; 3],
    pub size: u32,
}
```

### 5. WGSL Code Generator

```rust
/// Generates WGSL compute shader source from AST
pub struct WgslCodegen {
    /// Generated expression code
    expr_code: String,
    /// Local variable declarations
    locals: Vec<String>,
    /// Counter for generating unique variable names
    var_counter: u32,
}

impl WgslCodegen {
    /// Generate complete WGSL shader source
    pub fn generate(ast: &Expr) -> Result<String, CompileError> {
        let mut codegen = Self {
            expr_code: String::new(),
            locals: Vec::new(),
            var_counter: 0,
        };

        let result_expr = codegen.gen_expr(ast)?;

        Ok(codegen.build_shader(&result_expr))
    }

    fn build_shader(&self, result_expr: &str) -> String {
        format!(r#"
// === NOISE FUNCTIONS ===
fn hash(p: vec3<f32>) -> f32 {{
    let h = dot(p, vec3(127.1, 311.7, 74.7));
    return fract(sin(h) * 43758.5453);
}}

fn noise3(p: vec3<f32>) -> f32 {{
    let i = floor(p);
    let f = fract(p);
    let u = f * f * (3.0 - 2.0 * f);

    return mix(
        mix(
            mix(hash(i + vec3(0.0, 0.0, 0.0)), hash(i + vec3(1.0, 0.0, 0.0)), u.x),
            mix(hash(i + vec3(0.0, 1.0, 0.0)), hash(i + vec3(1.0, 1.0, 0.0)), u.x),
            u.y
        ),
        mix(
            mix(hash(i + vec3(0.0, 0.0, 1.0)), hash(i + vec3(1.0, 0.0, 1.0)), u.x),
            mix(hash(i + vec3(0.0, 1.0, 1.0)), hash(i + vec3(1.0, 1.0, 1.0)), u.x),
            u.y
        ),
        u.z
    );
}}

fn fbm(p: vec3<f32>, octaves: u32) -> f32 {{
    var value = 0.0;
    var amplitude = 0.5;
    var pos = p;
    for (var i = 0u; i < octaves; i++) {{
        value += amplitude * noise3(pos);
        pos *= 2.0;
        amplitude *= 0.5;
    }}
    return value;
}}

fn turbulence(p: vec3<f32>, octaves: u32) -> f32 {{
    var value = 0.0;
    var amplitude = 0.5;
    var pos = p;
    for (var i = 0u; i < octaves; i++) {{
        value += amplitude * abs(noise3(pos) * 2.0 - 1.0);
        pos *= 2.0;
        amplitude *= 0.5;
    }}
    return value;
}}

// === UNIFORMS ===
struct Uniforms {{
    time: f32,
    depth: u32,
    seed: u32,
    _padding: u32,
    world_offset: vec3<f32>,
    size: u32,
}}

@group(0) @binding(0) var<uniform> uniforms: Uniforms;
@group(0) @binding(1) var<storage, read_write> materials: array<u32>;

// === MAIN ===
@compute @workgroup_size(8, 8, 8)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {{
    let size = uniforms.size;
    if (global_id.x >= size || global_id.y >= size || global_id.z >= size) {{
        return;
    }}

    // Normalized position [-1, 1]
    let pos = vec3<f32>(global_id) / f32(size) * 2.0 - 1.0;
    let x = pos.x;
    let y = pos.y;
    let z = pos.z;

    // World position
    let wx = uniforms.world_offset.x + pos.x * f32(size);
    let wy = uniforms.world_offset.y + pos.y * f32(size);
    let wz = uniforms.world_offset.z + pos.z * f32(size);

    let time = uniforms.time;
    let depth = uniforms.depth;
    let seed = uniforms.seed;

    // Local variable declarations
{locals}

    // === COMPILED EXPRESSION ===
    let result = u32({result_expr});
    // === END EXPRESSION ===

    let idx = global_id.z * size * size + global_id.y * size + global_id.x;
    materials[idx] = result;
}}
"#,
            locals = self.locals.join("\n"),
            result_expr = result_expr
        )
    }

    fn gen_expr(&mut self, expr: &Expr) -> Result<String, CompileError> {
        match expr {
            Expr::Const(v) => Ok(format!("{:.6}", v)),
            Expr::Material(id) => Ok(format!("{}", id)),
            Expr::Var(var) => Ok(self.var_name(var).to_string()),

            Expr::BinOp { op, lhs, rhs } => {
                let l = self.gen_expr(lhs)?;
                let r = self.gen_expr(rhs)?;
                Ok(format!("({} {} {})", l, self.binop_wgsl(op), r))
            }

            Expr::UnaryOp { op, expr } => {
                let e = self.gen_expr(expr)?;
                match op {
                    UnaryOpKind::Neg => Ok(format!("(-{})", e)),
                    UnaryOpKind::Not => Ok(format!("(1.0 - {})", e)),
                }
            }

            Expr::Call { func, args } => {
                let args_code: Vec<String> = args.iter()
                    .map(|a| self.gen_expr(a))
                    .collect::<Result<_, _>>()?;
                self.gen_call(func, &args_code)
            }

            Expr::IfElse { cond, then_, else_ } => {
                let c = self.gen_expr(cond)?;
                let t = self.gen_expr(then_)?;
                let e = self.gen_expr(else_)?;
                Ok(format!("select({}, {}, {} > 0.5)", e, t, c))
            }

            Expr::Match { expr, arms } => {
                self.gen_match(expr, arms)
            }

            Expr::Let { name, value, body } => {
                let val = self.gen_expr(value)?;
                let var_name = format!("_let_{}", self.var_counter);
                self.var_counter += 1;
                self.locals.push(format!("    let {} = {};  // {}", var_name, val, name));

                // Replace references to 'name' in body with var_name
                let body_with_subst = self.substitute_var(body, name, &var_name);
                self.gen_expr(&body_with_subst)
            }
        }
    }

    fn gen_call(&self, func: &BuiltinFunc, args: &[String]) -> Result<String, CompileError> {
        match func {
            // Direct WGSL equivalents
            BuiltinFunc::Sin => Ok(format!("sin({})", args[0])),
            BuiltinFunc::Cos => Ok(format!("cos({})", args[0])),
            BuiltinFunc::Tan => Ok(format!("tan({})", args[0])),
            BuiltinFunc::ASin => Ok(format!("asin({})", args[0])),
            BuiltinFunc::ACos => Ok(format!("acos({})", args[0])),
            BuiltinFunc::ATan => Ok(format!("atan({})", args[0])),
            BuiltinFunc::ATan2 => Ok(format!("atan2({}, {})", args[0], args[1])),
            BuiltinFunc::Floor => Ok(format!("floor({})", args[0])),
            BuiltinFunc::Ceil => Ok(format!("ceil({})", args[0])),
            BuiltinFunc::Round => Ok(format!("round({})", args[0])),
            BuiltinFunc::Abs => Ok(format!("abs({})", args[0])),
            BuiltinFunc::Sign => Ok(format!("sign({})", args[0])),
            BuiltinFunc::Sqrt => Ok(format!("sqrt({})", args[0])),
            BuiltinFunc::Pow => Ok(format!("pow({}, {})", args[0], args[1])),
            BuiltinFunc::Exp => Ok(format!("exp({})", args[0])),
            BuiltinFunc::Log => Ok(format!("log({})", args[0])),
            BuiltinFunc::Log2 => Ok(format!("log2({})", args[0])),
            BuiltinFunc::Min => Ok(format!("min({}, {})", args[0], args[1])),
            BuiltinFunc::Max => Ok(format!("max({}, {})", args[0], args[1])),
            BuiltinFunc::Clamp => Ok(format!("clamp({}, {}, {})", args[0], args[1], args[2])),
            BuiltinFunc::Lerp => Ok(format!("mix({}, {}, {})", args[0], args[1], args[2])),
            BuiltinFunc::Smoothstep => Ok(format!("smoothstep({}, {}, {})", args[0], args[1], args[2])),

            // Custom noise functions (defined at top of shader)
            BuiltinFunc::Noise => Ok(format!("noise3(vec3({}, {}, {}))", args[0], args[1], args[2])),
            BuiltinFunc::Fbm => Ok(format!("fbm(vec3({}, {}, {}), u32({}))", args[0], args[1], args[2], args[3])),
            BuiltinFunc::Turbulence => Ok(format!("turbulence(vec3({}, {}, {}), u32({}))", args[0], args[1], args[2], args[3])),
        }
    }

    fn gen_match(&mut self, expr: &Expr, arms: &[MatchArm]) -> Result<String, CompileError> {
        // Convert match to nested select() calls
        let e = self.gen_expr(expr)?;

        let mut result = String::new();
        let mut depth = 0;

        for arm in arms {
            let arm_expr = self.gen_expr(&arm.expr)?;

            match &arm.pattern {
                Pattern::Wildcard => {
                    result.push_str(&arm_expr);
                }
                Pattern::Const(val) => {
                    result.push_str(&format!("select("));
                    depth += 1;
                    // Will be filled by next iteration or wildcard
                    result.push_str(&format!("{}, abs({} - {:.6}) < 0.001)", arm_expr, e, val));
                }
                Pattern::Range { min, max } => {
                    result.push_str(&format!(
                        "select(, {}, {} >= {:.6} && {} <= {:.6})",
                        arm_expr, e, min, e, max
                    ));
                    depth += 1;
                }
            }
        }

        // Close select() calls
        for _ in 0..depth {
            result.push_str(", ");
        }

        Ok(result)
    }

    fn var_name(&self, var: &VarId) -> &'static str {
        match var {
            VarId::X => "x",
            VarId::Y => "y",
            VarId::Z => "z",
            VarId::WorldX => "wx",
            VarId::WorldY => "wy",
            VarId::WorldZ => "wz",
            VarId::Time => "time",
            VarId::Depth => "f32(depth)",
            VarId::Seed => "f32(seed)",
        }
    }

    fn binop_wgsl(&self, op: &BinOpKind) -> &'static str {
        match op {
            BinOpKind::Add => "+",
            BinOpKind::Sub => "-",
            BinOpKind::Mul => "*",
            BinOpKind::Div => "/",
            BinOpKind::Mod => "%",
            BinOpKind::Pow => "**",  // Note: WGSL doesn't have **, need pow()
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

    fn substitute_var(&self, expr: &Expr, name: &str, replacement: &str) -> Expr {
        // Clone and substitute variable references
        // This is a simplified version - real impl would be more thorough
        expr.clone()  // TODO: implement proper substitution
    }
}
```

### 6. Backend Selection

```rust
/// Compiled function with both CPU and GPU backends
pub struct CompiledFunction {
    /// Original source
    source: String,
    /// Parsed AST
    ast: Expr,
    /// CPU backend (always available)
    cpu: CpuFunction,
    /// GPU backend (None if WebGPU unavailable)
    gpu: Option<GpuFunction>,
    /// Does expression use time variable?
    uses_time: bool,
    /// Does expression use noise functions?
    uses_noise: bool,
    /// Estimated expression complexity (for backend selection)
    complexity: u32,
}

impl CompiledFunction {
    /// Compile expression with automatic backend selection
    pub fn compile(
        source: &str,
        device: Option<&wgpu::Device>,
    ) -> Result<Self, CompileError> {
        // Parse to AST
        let ast = Parser::parse(source)?;

        // Analyze expression
        let uses_time = ast.contains_var(VarId::Time);
        let uses_noise = ast.contains_func(BuiltinFunc::Noise)
            || ast.contains_func(BuiltinFunc::Fbm)
            || ast.contains_func(BuiltinFunc::Turbulence);
        let complexity = ast.estimate_complexity();

        // Compile CPU backend
        let cpu = CpuFunction::compile(source)?;

        // Compile GPU backend if device available
        let gpu = device.map(|d| GpuFunction::compile(&ast, d)).transpose()?;

        Ok(Self {
            source: source.to_string(),
            ast,
            cpu,
            gpu,
            uses_time,
            uses_noise,
            complexity,
        })
    }

    /// Evaluate cube, auto-selecting best backend
    pub fn eval_cube(
        &mut self,
        depth: u32,
        ctx: &EvalContext,
        device: Option<&wgpu::Device>,
        queue: Option<&wgpu::Queue>,
    ) -> Cube<u8> {
        let voxel_count = 8usize.pow(depth);

        // Heuristic for backend selection
        let use_gpu = self.gpu.is_some()
            && device.is_some()
            && queue.is_some()
            && (voxel_count > 4096 || self.uses_time || self.complexity > 10);

        if use_gpu {
            self.eval_gpu(depth, ctx, device.unwrap(), queue.unwrap())
        } else {
            self.eval_cpu(depth, ctx)
        }
    }

    fn eval_cpu(&mut self, depth: u32, ctx: &EvalContext) -> Cube<u8> {
        // Recursive evaluation building octree
        self.eval_cpu_recursive(depth, CubeCoord::root(), ctx)
    }

    fn eval_cpu_recursive(
        &mut self,
        remaining_depth: u32,
        coord: CubeCoord,
        ctx: &EvalContext,
    ) -> Cube<u8> {
        if remaining_depth == 0 {
            let local_ctx = EvalContext {
                position: coord.to_normalized_position(),
                ..*ctx
            };
            return Cube::Solid(self.cpu.eval(&local_ctx));
        }

        let children: [Rc<Cube<u8>>; 8] = std::array::from_fn(|i| {
            let child_coord = coord.child(i);
            Rc::new(self.eval_cpu_recursive(remaining_depth - 1, child_coord, ctx))
        });

        Cube::Cubes(Box::new(children)).simplified()
    }

    fn eval_gpu(
        &self,
        depth: u32,
        ctx: &EvalContext,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
    ) -> Cube<u8> {
        let gpu = self.gpu.as_ref().unwrap();
        let size = 2u32.pow(depth);

        let uniforms = Uniforms {
            time: ctx.time,
            depth,
            seed: ctx.seed,
            _padding: 0,
            world_offset: ctx.world_position.into(),
            size,
        };

        // Get flat array of materials from GPU
        let materials = gpu.eval_batch(device, queue, size, &uniforms);

        // Convert to octree (can optimize later with octree-aware dispatch)
        self.flat_to_octree(&materials, size, depth)
    }

    fn flat_to_octree(&self, materials: &[u8], size: u32, depth: u32) -> Cube<u8> {
        // Build octree from flat array
        // This is a bottleneck - future optimization: build octree on GPU
        self.flat_to_octree_recursive(materials, size, 0, 0, 0, depth)
    }

    fn flat_to_octree_recursive(
        &self,
        materials: &[u8],
        size: u32,
        x: u32, y: u32, z: u32,
        remaining_depth: u32,
    ) -> Cube<u8> {
        if remaining_depth == 0 {
            let idx = (z * size * size + y * size + x) as usize;
            return Cube::Solid(materials[idx]);
        }

        let half = size / 2u32.pow(remaining_depth - 1);
        let children: [Rc<Cube<u8>>; 8] = std::array::from_fn(|i| {
            let ox = (i & 1) as u32 * half;
            let oy = ((i >> 1) & 1) as u32 * half;
            let oz = ((i >> 2) & 1) as u32 * half;
            Rc::new(self.flat_to_octree_recursive(
                materials, size,
                x + ox, y + oy, z + oz,
                remaining_depth - 1
            ))
        });

        Cube::Cubes(Box::new(children)).simplified()
    }
}
```

## File Organization

```
crates/cube/src/
├── function/
│   ├── mod.rs           # Module exports
│   ├── ast.rs           # AST definitions (shared)
│   ├── parser.rs        # nom-based parser
│   ├── cpu/
│   │   ├── mod.rs       # CPU backend
│   │   ├── fasteval.rs  # fasteval wrapper
│   │   └── noise.rs     # CPU noise implementation
│   ├── gpu/
│   │   ├── mod.rs       # GPU backend
│   │   ├── wgsl.rs      # WGSL code generator
│   │   ├── pipeline.rs  # Compute pipeline setup
│   │   └── noise.wgsl   # WGSL noise functions
│   ├── compiled.rs      # CompiledFunction (unified)
│   └── tests.rs         # Cross-backend comparison tests
├── dynamic_cube.rs      # DynamicCube wrapper type
└── ...
```

## Testing Strategy

1. **Parser tests**: Various expression syntaxes
2. **AST tests**: Roundtrip (parse → AST → string → parse)
3. **CPU tests**: Known inputs → expected outputs
4. **GPU tests**: Compare GPU output with CPU output (must match)
5. **Performance tests**: Benchmark CPU vs GPU at various depths
6. **Visual tests**: Render function cubes, compare screenshots

```rust
#[test]
fn test_cpu_gpu_parity() {
    let expr = "if noise(x, y, z) > 0.5 then 20 else 16";
    let func = CompiledFunction::compile(expr, Some(&device)).unwrap();

    let ctx = EvalContext::default();

    let cpu_result = func.eval_cpu(4, &ctx);
    let gpu_result = func.eval_gpu(4, &ctx, &device, &queue);

    // Compare all voxels
    assert_cubes_equal(&cpu_result, &gpu_result);
}
```
