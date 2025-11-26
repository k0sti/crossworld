# renderer-lighting Specification

## Purpose
TBD - created by archiving change standardize-material-system. Update Purpose after archive.
## Requirements
### Requirement: Standardized Lighting Model
All renderers (CPU, GL, GPU) SHALL use the same lighting model with consistent parameters and calculations.

**Lighting Components**:
- **Light Direction**: `vec3(0.5, 1.0, 0.3).normalize()` (fixed directional light from upper-right-front)
- **Ambient Term**: 0.3 (30% ambient illumination)
- **Diffuse Term**: Lambert `max(dot(normal, lightDir), 0.0) * 0.7`
- **Final Formula**: `materialColor * (ambient + diffuse)`

**Not Included**:
- No specular highlights
- No fresnel edge effects
- No shadows
- No global illumination

#### Scenario: Calculate lighting for top face
- **GIVEN** a voxel surface with normal `vec3(0, 1, 0)` (top face) and material color RGB(255, 0, 0)
- **WHEN** applying the lighting formula
- **THEN** diffuse SHALL be `max(dot([0,1,0], normalize([0.5,1.0,0.3])), 0) * 0.7` ≈ 0.602
- **AND** final color SHALL be `[255,0,0] * (0.3 + 0.602)` = [230, 0, 0] approximately

#### Scenario: Calculate lighting for bottom face
- **GIVEN** a voxel surface with normal `vec3(0, -1, 0)` (bottom face) and material color RGB(0, 255, 0)
- **WHEN** applying the lighting formula
- **THEN** diffuse SHALL be `max(dot([0,-1,0], lightDir), 0) * 0.7` = 0.0 (facing away from light)
- **AND** final color SHALL be `[0,255,0] * 0.3` = [0, 77, 0] (ambient only)

#### Scenario: All tracers produce consistent lighting
- **GIVEN** the same scene with identical voxel, camera, and material
- **WHEN** rendered by CPU, GL, and GPU tracers
- **THEN** pixel RGB values SHALL differ by at most 5 units (accounting for float precision and gamma)

### Requirement: Lighting Constants
The system SHALL define lighting constants that are shared across all renderers.

**Rust Constants**:
```rust
pub const LIGHT_DIR: Vec3 = Vec3::new(0.5, 1.0, 0.3).normalize();
pub const AMBIENT: f32 = 0.3;
pub const DIFFUSE_STRENGTH: f32 = 0.7;
```

**GLSL Constants**:
```glsl
const vec3 LIGHT_DIR = normalize(vec3(0.5, 1.0, 0.3));
const float AMBIENT = 0.3;
const float DIFFUSE_STRENGTH = 0.7;
```

#### Scenario: Lighting constants are consistent
- **GIVEN** lighting constants defined in both Rust and GLSL
- **WHEN** comparing their values
- **THEN** `LIGHT_DIR`, `AMBIENT`, and `DIFFUSE_STRENGTH` SHALL be identical across languages

### Requirement: Background Color
All renderers SHALL use a consistent bluish-gray background color for empty space.

**Background Color**: RGB(0.4, 0.5, 0.6) or `vec3(102, 128, 153)` in 8-bit

**Rust**:
```rust
pub const BACKGROUND_COLOR: Vec3 = Vec3::new(0.4, 0.5, 0.6);
```

**GLSL**:
```glsl
const vec3 BACKGROUND_COLOR = vec3(0.4, 0.5, 0.6);
gl_FragColor = vec4(BACKGROUND_COLOR, 1.0); // for misses
```

#### Scenario: Background renders as bluish gray
- **GIVEN** a ray that misses all voxels
- **WHEN** the renderer calculates the pixel color
- **THEN** it SHALL return RGB(102, 128, 153) ± 5

#### Scenario: Background color is consistent across tracers
- **GIVEN** the same empty region rendered by all tracers
- **WHEN** sampling background pixels
- **THEN** all tracers SHALL produce the same background color RGB(102, 128, 153) ± 5

### Requirement: Lighting Toggle for Debugging
Renderers SHALL support an optional flag to disable lighting and output pure material colors.

**API**:
- `RenderRequest.disable_lighting: bool` (Rust)
- `uniform bool u_disable_lighting;` (GLSL)

**Behavior**:
- When `disable_lighting = true`: Output `materialColor` directly (no ambient/diffuse)
- When `disable_lighting = false` (default): Apply standard lighting model

#### Scenario: Disable lighting outputs pure colors
- **GIVEN** a red voxel (RGB 255, 0, 0) and `disable_lighting = true`
- **WHEN** rendering the voxel
- **THEN** the output SHALL be exactly RGB(255, 0, 0) (no shading)

#### Scenario: Enable lighting applies shading
- **GIVEN** the same red voxel and `disable_lighting = false`
- **WHEN** rendering with a top face (normal [0,1,0])
- **THEN** the output SHALL be approximately RGB(230, 0, 0) (with lighting)

#### Scenario: Lighting toggle is accessible via API
- **GIVEN** a render request
- **WHEN** setting `request.disable_lighting = true`
- **THEN** all tracers SHALL output unshaded colors

### Requirement: Color Verification Tests
The renderer test suite SHALL include automated tests that verify correct material colors and lighting.

**Required Tests**:
1. Test each primary material color (Red, Green, Blue, Yellow, White, Black)
2. Test background color in empty regions
3. Test lighting produces expected shading on different face orientations
4. Test lighting toggle produces pure vs shaded colors
5. Test all tracers produce consistent colors (within tolerance)

**Color Tolerance**: ±5 RGB units (0-255 scale) to account for floating-point precision and gamma correction.

#### Scenario: Verify red material renders correctly
- **GIVEN** an octa cube with octant 0 set to Red (value 1)
- **WHEN** rendering with a fixed camera and sampling the octant 0 region
- **THEN** the sampled pixel SHALL have R component ≥ 200 and G, B components ≤ 50

#### Scenario: Verify tracers are consistent
- **GIVEN** the same scene rendered by CPU and GL tracers at the same resolution
- **WHEN** sampling the same pixel coordinates
- **THEN** RGB values SHALL differ by at most 5 units per channel

#### Scenario: Verify lighting toggle works
- **GIVEN** a scene rendered twice: once with `disable_lighting=false`, once with `true`
- **WHEN** comparing pixel brightness
- **THEN** the lit version SHALL have pixels with varying brightness (shading)
- **AND** the unlit version SHALL have uniform colors matching exact palette values

### Requirement: Remove Legacy Lighting Effects
The renderer SHALL NOT include fresnel effects, normal-based color variation, or other non-standard lighting.

**Removed Features**:
- Fresnel edge highlighting (`pow(1.0 - dot(-rayDir, normal), 3.0)`)
- Normal-based color interpolation (`mix(baseColor, colorX, abs(normal.x))`)
- Hardcoded orangeish base colors

#### Scenario: No fresnel effect on edges
- **GIVEN** a voxel viewed at a grazing angle (high fresnel in old system)
- **WHEN** rendering with the new lighting model
- **THEN** the pixel SHALL NOT have edge brightening (fresnel term = 0)

#### Scenario: No normal-based color variation
- **GIVEN** a red voxel (value 1) viewed from different angles
- **WHEN** rendering faces with different normals
- **THEN** the base material color SHALL remain pure red RGB(255,0,0) before lighting
- **AND** color variation SHALL only come from diffuse lighting, not normal-based lerping

