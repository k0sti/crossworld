# Voxel Materials Specification (Delta)

## ADDED Requirements

### Requirement: Use Existing World Material System
The renderer SHALL use the material system defined in `crates/world/src/world_cube/mod.rs::MaterialColorMapper`.

**Material Source**:
- **World crate**: 128 materials (indices 0-127) loaded from `assets/materials.json`
- **Dawnbringer 32**: Indices 32-63 use hardcoded Dawnbringer palette (fallback)
- **Test palette**: Renderer crate provides minimal 6-color palette for standalone tests

**Special Value**:
- Value 0: Reserved for empty/transparent voxels (not rendered)

#### Scenario: World crate provides material colors
- **GIVEN** the world crate has loaded materials from `materials.json`
- **WHEN** mesh generation requests material color for index 32
- **THEN** it SHALL return the color from Dawnbringer palette or materials.json

#### Scenario: Empty voxel is transparent
- **GIVEN** a voxel with value 0
- **WHEN** the raytracer evaluates the voxel
- **THEN** it SHALL skip the voxel (treat as empty) and continue traversal

#### Scenario: Renderer uses test palette when world unavailable
- **GIVEN** a renderer-only test without world crate dependency
- **WHEN** requesting material color for index 1 (Red)
- **THEN** the renderer SHALL return RGB(1.0, 0.0, 0.0) from its test palette

### Requirement: Primary Material Constants
The system SHALL define at least 6 primary materials with fixed palette indices for testing and common use.

**Primary Materials**:
1. Red: Index 1, RGB(255, 0, 0)
2. Green: Index 2, RGB(0, 255, 0)
3. Blue: Index 3, RGB(0, 0, 255)
4. Yellow: Index 4, RGB(255, 255, 0)
5. White: Index 5, RGB(255, 255, 255)
6. Black: Index 6, RGB(0, 0, 0)

#### Scenario: Render red voxel
- **GIVEN** a voxel with value 1 (Red material)
- **WHEN** the renderer calculates the base color
- **THEN** it SHALL use RGB(255, 0, 0) before applying lighting

#### Scenario: Render multi-colored scene
- **GIVEN** a scene with voxels valued 1, 2, 3, 4, 5 (Red, Green, Blue, Yellow, White)
- **WHEN** the scene is rendered
- **THEN** each voxel SHALL display its respective palette color

### Requirement: Material Palette Constant
The renderer SHALL provide a `MATERIAL_PALETTE` constant accessible to both Rust and GLSL code.

**Rust**:
```rust
pub const MATERIAL_PALETTE: [Vec3; 256] = [...];
pub fn get_material_color(value: i32) -> Vec3;
```

**GLSL**:
```glsl
const vec3 MATERIAL_PALETTE[256] = vec3[256](...);
vec3 getMaterialColor(int value);
```

#### Scenario: Query material color in Rust
- **GIVEN** a voxel value of 2 (Green)
- **WHEN** calling `get_material_color(2)`
- **THEN** it SHALL return `Vec3::new(0.0, 1.0, 0.0)` (normalized RGB)

#### Scenario: Query material color in GLSL shader
- **GIVEN** a voxel value of 4 (Yellow) in a fragment shader
- **WHEN** calling `getMaterialColor(4)`
- **THEN** it SHALL return `vec3(1.0, 1.0, 0.0)`

### Requirement: Material-Based Rendering
All tracers (CPU, GL, GPU) SHALL use voxel values to look up material colors from the palette instead of hardcoded colors.

#### Scenario: CPU tracer uses material palette
- **GIVEN** a CPU tracer rendering a voxel with value 3 (Blue)
- **WHEN** the raycast hits the voxel
- **THEN** the tracer SHALL retrieve RGB(0, 0, 255) from `MATERIAL_PALETTE[3]`

#### Scenario: GL tracer uses material palette
- **GIVEN** a GL fragment shader rendering a voxel with value 5 (White)
- **WHEN** the shader calculates the pixel color
- **THEN** it SHALL retrieve `vec3(1.0)` from `MATERIAL_PALETTE[5]`

#### Scenario: All tracers produce consistent colors
- **GIVEN** the same octree scene rendered by CPU, GL, and GPU tracers
- **WHEN** comparing pixel colors at the same screen position
- **THEN** all tracers SHALL produce identical material base colors (before lighting differences)
