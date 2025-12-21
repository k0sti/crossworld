# Tasks: Add Function Cube Type

## Phase 1: Core Expression System (CPU) ✅ COMPLETE

### 1.1 AST Definition
- [x] Create `crates/cube/src/function/mod.rs` with module structure
- [x] Define `Expr` enum with all expression variants
- [x] Define `VarId`, `BinOpKind`, `UnaryOpKind`, `BuiltinFunc` enums
- [x] Implement `Display` trait for AST pretty-printing
- [x] Add helper methods: `contains_var()`, `contains_func()`, `estimate_complexity()`
- [x] Add basic unit tests for AST construction

### 1.2 Parser Implementation
- [x] Add `nom` dependency to cube crate (already present)
- [x] Create `crates/cube/src/function/parser.rs`
- [x] Implement tokenizer for numbers, identifiers, operators
- [x] Parse arithmetic expressions with operator precedence
- [x] Parse comparison and logical operators
- [x] Parse function calls (sin, cos, noise, etc.)
- [x] Parse `if-then-else` expressions
- [x] Parse `let` bindings
- [x] Parse `match` expressions
- [x] Parse material constants (STONE, GRASS, etc.)
- [x] Comprehensive parser tests with edge cases
- [x] Error messages with source location

### 1.3 fasteval Integration
- [x] Add `fasteval` dependency to cube crate
- [x] Create `crates/cube/src/function/cpu/mod.rs`
- [x] Implement `AstToFasteval` converter:
  - [x] Convert `if/then/else` to ternary (via multiplication)
  - [x] Convert `match` to nested ternary
  - [x] Handle `let` bindings via substitution
- [x] Register custom functions (noise, fbm, turbulence)
- [x] Implement `CpuFunction` struct with `compile()` and `eval()`
- [x] Unit tests for CPU evaluation

### 1.4 CPU Noise Implementation
- [x] Create `crates/cube/src/function/cpu/noise.rs`
- [x] Implement Value noise 3D (primary implementation)
- [x] Implement Perlin noise 3D (available for future use)
- [x] Implement FBM (Fractal Brownian Motion)
- [x] Implement turbulence
- [x] Implement ridged noise (available for future use)
- [x] Tests with deterministic seeds

### 1.5 DynamicCube Type
- [x] Create `crates/cube/src/function/dynamic_cube.rs`
- [x] Define `DynamicCube` enum (Static, Function)
- [x] Implement `get_material()` for CPU evaluation
- [x] Implement `materialize()` for bulk CPU evaluation
- [x] Add caching for time-invariant functions
- [x] Integration tests

## Phase 2: GPU Backend (WGSL)

### 2.1 WGSL Code Generator
- [ ] Create `crates/cube/src/function/gpu/mod.rs`
- [ ] Create `crates/cube/src/function/gpu/wgsl.rs`
- [ ] Implement `WgslCodegen` struct:
  - [ ] Generate arithmetic expressions
  - [ ] Generate comparison/logical operators
  - [ ] Generate function calls (map to WGSL builtins)
  - [ ] Generate `if/else` → `select()` or WGSL `if`
  - [ ] Generate `match` → nested `select()` chain
  - [ ] Generate `let` → WGSL local variables
- [ ] Implement noise function library in WGSL:
  - [ ] Hash function
  - [ ] `noise3()` - 3D value noise
  - [ ] `fbm()` - Fractal Brownian Motion
  - [ ] `turbulence()` - Turbulence noise
- [ ] Generate complete shader with uniforms and main entry
- [ ] Unit tests comparing generated WGSL with expected output

### 2.2 GPU Pipeline Setup
- [ ] Add `wgpu` dependency (or use existing if available)
- [ ] Create `crates/cube/src/function/gpu/pipeline.rs`
- [ ] Define `Uniforms` struct (time, depth, seed, world_offset, size)
- [ ] Implement `GpuFunction` struct:
  - [ ] Create shader module from generated WGSL
  - [ ] Create bind group layout (uniforms + output buffer)
  - [ ] Create compute pipeline
- [ ] Implement `eval_batch()`:
  - [ ] Create uniform buffer
  - [ ] Create output storage buffer
  - [ ] Create staging buffer for readback
  - [ ] Dispatch compute shader
  - [ ] Read back results
- [ ] Handle GPU errors gracefully (fallback to CPU)

### 2.3 Octree Construction from GPU Output
- [ ] Implement `flat_to_octree()` - convert flat array to octree
- [ ] Implement recursive octree builder with `simplified()` calls
- [ ] Optimize for uniform regions (early termination)
- [ ] Future: Consider GPU-side octree construction

### 2.4 Backend Selection
- [ ] Create `crates/cube/src/function/compiled.rs`
- [ ] Implement `CompiledFunction` with both backends
- [ ] Implement heuristic for backend selection:
  - [ ] Voxel count threshold (> 4096 → GPU)
  - [ ] Uses time variable → GPU (continuous updates)
  - [ ] Expression complexity → GPU if complex
  - [ ] WebGPU availability check
- [ ] Implement `eval_cube()` with automatic selection
- [ ] Allow manual backend override

### 2.5 CPU/GPU Parity Tests
- [ ] Create `crates/cube/src/function/tests.rs`
- [ ] Test that CPU and GPU produce identical results for:
  - [ ] Simple arithmetic expressions
  - [ ] Conditional expressions
  - [ ] Noise-based expressions (use deterministic noise)
  - [ ] Complex nested expressions
- [ ] Benchmark CPU vs GPU at various depths

## Phase 3: Integration

### 3.1 Mesh Generation Integration
- [ ] Modify `visit_faces` to accept `DynamicCube`
- [ ] Pass `EvalContext` through traversal
- [ ] Evaluate function at each voxel position during mesh generation
- [ ] Test rendering simple function cubes

### 3.2 World Integration
- [ ] Add `DynamicCube` support to `WorldCube`
- [ ] Allow function-based terrain layers
- [ ] Example: noise-based grass/stone distribution
- [ ] Performance test with procedural world generation

### 3.3 Time-based Animation
- [ ] Implement frame-by-frame re-evaluation for `uses_time` functions
- [ ] Cache management for time-varying cubes
- [ ] LOD-aware evaluation (lower depth for distant cubes)

## Phase 4: WASM Bindings

### 4.1 WASM API
- [ ] Add `wasm-bindgen` exports for parser
- [ ] Add `wasm-bindgen` exports for compiler
- [ ] Add `wasm-bindgen` exports for evaluation
- [ ] Create TypeScript type definitions
- [ ] Handle GPU context from JavaScript (WebGPU device)

### 4.2 JavaScript Integration
- [ ] Create `FunctionCube` class in TypeScript
- [ ] Add expression validation in TypeScript
- [ ] Example usage in app
- [ ] Error handling and display

## Phase 5: Polish and Optimization

### 5.1 Error Handling
- [ ] Create `FunctionError` enum with all error types
- [ ] Add source location tracking to parser errors
- [ ] User-friendly error messages
- [ ] Shader compilation error handling

### 5.2 Performance Optimization
- [ ] Profile CPU evaluation
- [ ] Profile GPU dispatch and readback
- [ ] Optimize hot paths (stack operations, noise)
- [ ] Consider async GPU dispatch (non-blocking)
- [ ] Benchmark against direct fasteval (no AST intermediary)
- [ ] Document performance characteristics

### 5.3 Documentation
- [ ] Doc comments for all public types
- [ ] Expression language reference document
- [ ] Example expressions library
- [ ] GPU requirements and fallback behavior

## Verification Milestones

### M1: Parser Works ✅
- Can parse `sin(x) + cos(y)` to valid AST
- Error message for invalid syntax

### M2: CPU Backend Works ✅
- Can compile and evaluate with fasteval
- `sin(0)` returns `0.0`
- Noise functions return deterministic values

### M3: WGSL Generator Works
- Generates valid WGSL for simple expressions
- Generates noise functions
- Shader compiles without errors

### M4: GPU Backend Works
- Compute shader dispatches successfully
- Results match CPU output
- Time-based expressions animate

### M5: Integration Works
- Function cube renders in 3D view
- Material colors match expression logic
- Performance acceptable for real-time

### M6: WASM Works
- Full pipeline runs in browser
- TypeScript API is ergonomic
- GPU backend works in WebGPU-capable browsers

## Dependencies

```
Phase 1 (CPU) ──────┬──────> Phase 3 (Integration)
                    │
Phase 2 (GPU) ──────┘
                    │
Phase 4 (WASM) <────┘
                    │
Phase 5 (Polish) <──┘
```

- Phase 2 can run in parallel with Phase 1.4-1.5
- Phase 3 requires both Phase 1 and Phase 2
- Phase 4 requires Phase 3
- Phase 5 can start after Phase 3

## Estimated Complexity

| Phase | Tasks | Complexity | Notes |
|-------|-------|------------|-------|
| Phase 1 | 25 | High | Parser, fasteval integration ✅ |
| Phase 2 | 18 | High | WGSL codegen, GPU pipeline |
| Phase 3 | 7 | Medium | Integration with existing systems |
| Phase 4 | 6 | Low | WASM bindings |
| Phase 5 | 9 | Medium | Polish and optimization |

**Total**: ~65 tasks

## Risk Mitigation

| Risk | Mitigation |
|------|------------|
| fasteval API changes | Pin version, wrapper abstraction |
| WGSL codegen bugs | Extensive parity tests with CPU |
| WebGPU not available | CPU fallback always works |
| Shader compilation slow | Cache compiled pipelines |
| GPU memory limits | Tile large cubes, chunk processing |
