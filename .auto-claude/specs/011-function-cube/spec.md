# Specification: Add Function Cube Type

## Overview

Add a new `Cube::Function` variant that evaluates material values dynamically from user-defined expressions. Expressions compile to a shared AST that can target both **CPU (fasteval)** and **GPU (WGSL compute shaders)** backends, enabling massive parallelism for procedural content generation.

## Source

Migrated from: `openspec/changes/add-function-cube-type/`

## Current Status

**Completion: 56% (29/52 tasks complete)**

### Completed Work
- Phase 1: Core Expression System (CPU) - All subtasks complete
  - AST Definition with Expr enum and helper methods
  - Parser Implementation with nom
  - fasteval Integration for CPU evaluation
  - CPU Noise Implementation (Value, Perlin, FBM, turbulence)
  - DynamicCube Type with caching
- Phase 2: GPU Backend (WGSL) - Partial
  - WGSL Code Generator complete
  - CPU/GPU Parity Tests complete

### Pending Work
- Phase 2: GPU Pipeline Setup, Octree Construction, Backend Selection
- Phase 3: Integration (Mesh, World, Animation)
- Phase 4: WASM Bindings
- Phase 5: Polish and Optimization

## Problem Statement

The current cube system supports:
- `Cube::Solid(T)` - Static material values
- `Cube::Cubes/Quad/Layers` - Spatial subdivision
- `Cube<Quat>` via Fabric system - Quaternion-based procedural fields

**Gap**: No mechanism for user-defined procedural material generation with:
- Mathematical expressions (sin, cos, noise, conditionals)
- Runtime parameters (time, position, custom variables)
- GPU acceleration for real-time evaluation

## Solution: Dual Backend Architecture

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

## Expression Language

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

## Affected Files

### New Files (already created)
- `crates/cube/src/function/mod.rs` - Module structure
- `crates/cube/src/function/parser.rs` - nom-based expression parser
- `crates/cube/src/function/cpu/mod.rs` - CPU backend with fasteval
- `crates/cube/src/function/cpu/noise.rs` - Noise implementations
- `crates/cube/src/function/gpu/mod.rs` - GPU backend module
- `crates/cube/src/function/gpu/wgsl.rs` - WGSL code generator
- `crates/cube/src/function/dynamic_cube.rs` - DynamicCube type
- `crates/cube/src/function/tests.rs` - CPU/GPU parity tests

### Modified Files
- `crates/cube/src/lib.rs` - Add function module export

## Success Criteria

1. Parser correctly handles all expression language constructs
2. CPU backend evaluates expressions via fasteval
3. GPU backend generates valid WGSL shaders
4. CPU and GPU produce identical results for same inputs
5. Backend selection heuristic chooses optimal path
6. DynamicCube integrates with existing cube traversal
7. Time-based expressions support animation

## Development Environment

```bash
# Run function tests
cargo test -p cube function

# Check cube crate builds
cargo check -p cube

# Run all workspace tests
cargo test --workspace
```

## Key Algorithms

### Expression Parsing
```rust
// nom-based parser produces AST
let ast = parse_expression("if noise(x,y,z) > 0.5 then GRASS else STONE")?;
```

### CPU Evaluation
```rust
let cpu_func = CpuFunction::compile(ast)?;
let result = cpu_func.eval(&EvalContext { x, y, z, time, ... });
```

### WGSL Code Generation
```rust
let wgsl_code = WgslCodegen::generate(ast)?;
// Returns complete WGSL shader source
```
