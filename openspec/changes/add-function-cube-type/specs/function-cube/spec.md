# Function Cube Capability

Provides dynamic material evaluation through user-defined mathematical expressions compiled to bytecode.

## ADDED Requirements

### Requirement: Expression Parser

The system SHALL parse mathematical expressions from string format into an Abstract Syntax Tree (AST).

#### Scenario: Parse arithmetic expression
- **Given** an expression string `"x * 0.5 + y * 0.3"`
- **When** the parser processes the string
- **Then** it produces an AST with:
  - A top-level `Add` node
  - Left child: `Mul(Var(X), Const(0.5))`
  - Right child: `Mul(Var(Y), Const(0.3))`

#### Scenario: Parse conditional expression
- **Given** an expression string `"if y > 0.5 then GRASS else STONE"`
- **When** the parser processes the string
- **Then** it produces an AST with:
  - An `IfElse` node
  - Condition: `Gt(Var(Y), Const(0.5))`
  - Then branch: `Material(GRASS_ID)`
  - Else branch: `Material(STONE_ID)`

#### Scenario: Parse nested function calls
- **Given** an expression string `"sin(x * 3.14) * 0.5 + 0.5"`
- **When** the parser processes the string
- **Then** it produces an AST with a `Sin` function call containing `Mul(Var(X), Const(3.14))`

#### Scenario: Report parse error with location
- **Given** an invalid expression string `"sin(x +)"`
- **When** the parser processes the string
- **Then** it returns an error indicating the unexpected `)` token and its position

### Requirement: Bytecode Compiler

The system SHALL compile AST expressions into bytecode instructions for efficient evaluation.

#### Scenario: Compile simple expression
- **Given** an AST for `"x + y"`
- **When** the compiler processes the AST
- **Then** it produces bytecode: `[PushVar(X), PushVar(Y), Add, Return]`

#### Scenario: Constant folding optimization
- **Given** an AST for `"2 + 3 + x"`
- **When** the compiler processes the AST with optimization
- **Then** it produces bytecode with `PushConst(5)` instead of `[PushConst(2), PushConst(3), Add]`

#### Scenario: Track time usage
- **Given** an AST containing the `time` variable
- **When** the compiler processes the AST
- **Then** the resulting `CompiledFunction.uses_time` is `true`

#### Scenario: Track noise usage
- **Given** an AST containing a `noise(x, y, z)` call
- **When** the compiler processes the AST
- **Then** the resulting `CompiledFunction.uses_noise` is `true`

### Requirement: Bytecode VM

The system SHALL execute compiled bytecode with an evaluation context providing input values.

#### Scenario: Evaluate arithmetic expression
- **Given** compiled bytecode for `"x * 2"`
- **And** an EvalContext with `position.x = 0.5`
- **When** the VM evaluates the bytecode
- **Then** it returns `1.0` (clamped to material range as `1`)

#### Scenario: Evaluate conditional expression
- **Given** compiled bytecode for `"if y > 0 then 20 else 16"`
- **And** an EvalContext with `position.y = 0.5`
- **When** the VM evaluates the bytecode
- **Then** it returns `20` (material ID)

#### Scenario: Evaluate noise function
- **Given** compiled bytecode for `"noise(x, y, z)"`
- **And** an EvalContext with `position = (0.1, 0.2, 0.3)` and a NoiseSource
- **When** the VM evaluates the bytecode
- **Then** it returns a deterministic value in range `[0, 1]` based on the position and noise implementation

### Requirement: Built-in Math Functions

The system SHALL provide standard mathematical functions accessible in expressions.

#### Scenario: Trigonometric functions
- **Given** an expression `"sin(0)"`
- **When** evaluated
- **Then** it returns `0.0`

#### Scenario: Rounding functions
- **Given** an expression `"floor(1.7)"`
- **When** evaluated
- **Then** it returns `1.0`

#### Scenario: Interpolation functions
- **Given** an expression `"lerp(0, 10, 0.5)"`
- **When** evaluated
- **Then** it returns `5.0`

#### Scenario: Clamp function
- **Given** an expression `"clamp(x, 0, 1)"` with `x = 1.5`
- **When** evaluated
- **Then** it returns `1.0`

### Requirement: Noise Functions

The system SHALL provide noise generation functions for procedural content.

#### Scenario: Basic 3D noise
- **Given** an expression `"noise(0.5, 0.5, 0.5)"`
- **When** evaluated multiple times with the same inputs
- **Then** it returns the same deterministic value each time

#### Scenario: FBM noise
- **Given** an expression `"fbm(x, y, z, 4)"`
- **When** evaluated at various positions
- **Then** it returns smooth, multi-octave noise values

#### Scenario: Turbulence noise
- **Given** an expression `"turbulence(x, y, z, 4)"`
- **When** evaluated
- **Then** it returns absolute-value folded noise suitable for texture effects

### Requirement: DynamicCube Type

The system SHALL provide a wrapper type that can hold either static cubes or function-based cubes.

#### Scenario: Create static DynamicCube
- **Given** an existing `Cube<u8>`
- **When** wrapped in `DynamicCube::Static`
- **Then** `get_material()` returns the same values as the original cube

#### Scenario: Create function DynamicCube
- **Given** a `CompiledFunction` for `"if y > 0 then GRASS else STONE"`
- **When** wrapped in `DynamicCube::Function`
- **Then** `get_material()` evaluates the function at each position

#### Scenario: Materialize function to static cube
- **Given** a `DynamicCube::Function`
- **When** `materialize(depth=4)` is called
- **Then** it returns a `Cube<u8>` with materials evaluated at every position at that depth

### Requirement: Expression Language Variables

The system SHALL provide predefined input variables accessible in expressions.

#### Scenario: Position variables
- **Given** an EvalContext with `position = (0.25, 0.5, 0.75)`
- **When** expressions `"x"`, `"y"`, `"z"` are evaluated
- **Then** they return `0.25`, `0.5`, `0.75` respectively

#### Scenario: World position variables
- **Given** an EvalContext with `world_position = (100, 50, 200)`
- **When** expressions `"wx"`, `"wy"`, `"wz"` are evaluated
- **Then** they return `100`, `50`, `200` respectively

#### Scenario: Time variable
- **Given** an EvalContext with `time = 2.5`
- **When** expression `"time"` is evaluated
- **Then** it returns `2.5`

#### Scenario: Depth variable
- **Given** an EvalContext with `depth = 4`
- **When** expression `"depth"` is evaluated
- **Then** it returns `4.0`

### Requirement: Material Constants

The system SHALL provide named constants for common material IDs in expressions.

#### Scenario: Reference material by name
- **Given** an expression `"STONE"`
- **When** evaluated
- **Then** it returns the material ID `20` (or whatever STONE is defined as)

#### Scenario: Material names in conditionals
- **Given** an expression `"if y > 0.5 then GRASS else DIRT"`
- **When** evaluated with `y = 0.75`
- **Then** it returns the GRASS material ID

### Requirement: Let Bindings

The system SHALL support local variable definitions within expressions.

#### Scenario: Simple let binding
- **Given** an expression `"let a = x * 2; a + a"`
- **When** evaluated with `x = 0.25`
- **Then** it returns `1.0` (0.25 * 2 + 0.25 * 2)

#### Scenario: Nested let bindings
- **Given** an expression `"let a = x; let b = a * 2; b + 1"`
- **When** evaluated with `x = 0.5`
- **Then** it returns `2.0`

### Requirement: Match Expressions

The system SHALL support pattern matching for multi-way conditionals.

#### Scenario: Match with constants
- **Given** an expression:
  ```
  match floor(y * 4) {
    0 => BEDROCK,
    1 => STONE,
    2 => DIRT,
    _ => GRASS
  }
  ```
- **When** evaluated with `y = 0.6`
- **Then** it returns DIRT (floor(0.6 * 4) = 2)

#### Scenario: Match with wildcard
- **Given** a match expression with a `_` wildcard pattern
- **When** no other patterns match
- **Then** the wildcard arm is executed

### Requirement: WASM Bindings

The system SHALL expose function cube functionality to JavaScript/TypeScript via wasm-bindgen.

#### Scenario: Parse expression from JavaScript
- **Given** a JavaScript call to `parseExpression("x + y")`
- **When** the WASM function executes
- **Then** it returns a handle to the parsed AST or an error message

#### Scenario: Compile and evaluate from JavaScript
- **Given** a JavaScript call to compile and evaluate an expression
- **When** provided with position and time values
- **Then** it returns the computed material ID as a number

### Requirement: GPU Backend

The system SHALL provide a GPU compute shader backend for parallel evaluation of expressions via WGSL.

#### Scenario: Generate WGSL from expression
- **Given** an AST for `"sin(x) + cos(y)"`
- **When** the WGSL code generator processes the AST
- **Then** it produces valid WGSL source code with equivalent logic

#### Scenario: Dispatch compute shader
- **Given** a compiled GPU function and a cube size of 64x64x64
- **When** the GPU backend evaluates the expression
- **Then** it dispatches a compute shader with appropriate workgroups (8x8x8)
- **And** writes material IDs to a storage buffer

#### Scenario: GPU results match CPU
- **Given** the same expression evaluated on both CPU and GPU backends
- **When** compared at the same positions
- **Then** the material IDs are identical

#### Scenario: GPU noise functions
- **Given** an expression using `noise(x, y, z)`
- **When** evaluated on GPU
- **Then** it uses the WGSL noise implementation
- **And** produces smooth, deterministic values

### Requirement: Backend Selection

The system SHALL automatically select between CPU and GPU backends based on workload characteristics.

#### Scenario: Small workload uses CPU
- **Given** an expression to evaluate at depth 2 (64 voxels)
- **When** the system selects a backend
- **Then** it chooses the CPU backend (GPU dispatch overhead not worthwhile)

#### Scenario: Large workload uses GPU
- **Given** an expression to evaluate at depth 6 (262144 voxels)
- **And** WebGPU is available
- **When** the system selects a backend
- **Then** it chooses the GPU backend

#### Scenario: Time-varying expressions prefer GPU
- **Given** an expression containing the `time` variable
- **And** WebGPU is available
- **When** the system selects a backend
- **Then** it chooses the GPU backend (continuous re-evaluation expected)

#### Scenario: Fallback to CPU when GPU unavailable
- **Given** an expression to evaluate
- **And** WebGPU is NOT available
- **When** the system selects a backend
- **Then** it uses the CPU backend without error

### Requirement: WGSL Noise Functions

The system SHALL provide GPU-compatible noise functions in WGSL matching CPU behavior.

#### Scenario: 3D noise consistency
- **Given** the same coordinates evaluated on CPU `noise(0.5, 0.5, 0.5)` and GPU
- **When** compared
- **Then** the values are within floating-point tolerance (< 0.001)

#### Scenario: FBM on GPU
- **Given** an expression `"fbm(x, y, z, 4)"`
- **When** evaluated on GPU
- **Then** it produces multi-octave noise with smooth gradients

#### Scenario: Turbulence on GPU
- **Given** an expression `"turbulence(x, y, z, 4)"`
- **When** evaluated on GPU
- **Then** it produces absolute-value folded noise suitable for texture effects
