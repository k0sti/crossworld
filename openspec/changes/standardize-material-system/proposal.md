# Change: Standardize Material System and Lighting Across All Tracers

## Why

Currently, each tracer (CPU, GL, GPU) has inconsistent material handling and lighting calculations:
- **No material system**: Voxel values (1, 2, 3...) don't map to colors - lighting uses hardcoded orangeish tones with normal-based color variation
- **Inconsistent lighting**: CPU and GL tracers have similar but slightly different lighting code (ambient 0.5, diffuse with fresnel)
- **No color verification**: Tests check rendering happens but don't verify correct colors are produced
- **Missing test scene materials**: Octa cube uses only value 0 (empty) and 1 (solid), limiting testing

This makes it impossible to verify visual correctness and prevents implementing features like colored voxels, material properties, or debug visualization modes.

## What Changes

### 1. Use Existing Material System from World Crate
- **Source**: `crates/world/src/world_cube/mod.rs::MaterialColorMapper`
- **128 materials**: Indices 0-127 from `materials.json` (loaded via `setMaterialColors()`)
- **Dawnbringer 32 palette**: Indices 32-63 hardcoded (fallback)
- **6 primary test materials**: Define indices 1-6 in `materials.json` as Red, Green, Blue, Yellow, White, Black
- **Renderer integration**: Import material lookup from world crate or duplicate minimal palette for renderer-only tests
- **Value 0**: Reserved for empty/transparent (already implemented)

### 2. Standardized Lighting Model
- **Directional light**: Fixed direction `vec3(0.5, 1.0, 0.3).normalize()`
- **Ambient term**: 0.3 (darker than current 0.5 for better contrast)
- **Diffuse term**: Lambert cosine law `max(dot(normal, lightDir), 0.0)`
- **Final formula**: `materialColor * (ambient + diffuse * 0.7)` (no fresnel effect)
- **Background color**: Bluish gray `vec3(0.4, 0.5, 0.6)` (instead of 0.2, 0.3, 0.4)

### 3. Updated Test Scene
- **Octa cube materials**:
  - Octants 0, 2, 4, 6: Red (1), Green (2), Blue (3), Yellow (4)
  - Octants 1, 5: White (5)
  - Octants 3, 7: Empty (0)
- **3 empty + 5 colored**: Clear visual variety for testing

### 4. Optional Lighting Toggle
- **Debug mode flag**: `--no-lighting` or `RenderRequest.disable_lighting`
- **Pure voxel colors**: When enabled, output `materialColor` directly (no ambient/diffuse)
- **Use case**: Verify exact color values in tests

### 5. Color Verification Tests
- **Per-tracer tests**: Each tracer (CPU, GL, GPU) renders fixed camera view and verifies pixel colors
- **Specific octant checks**: Test that octant 0 renders as red, octant 2 as green, etc.
- **Color tolerance**: ±5 RGB units to account for lighting/gamma variations
- **Background check**: Verify bluish gray background in empty regions

## Impact

### Affected Specs
- **NEW**: `voxel-materials` - Material system and R2G3B2 palette specification
- **NEW**: `renderer-lighting` - Standardized lighting model across all tracers

### Affected Code
- `crates/world/src/world_cube/mod.rs`:
  - Already has `MaterialColorMapper` with 128 materials (no changes needed)
  - Material colors loaded from `materials.json` via `setMaterialColors()`
- `assets/materials.json` (or create if missing):
  - Define first 6 materials as Red, Green, Blue, Yellow, White, Black for testing
- `crates/renderer/src/materials.rs` (NEW):
  - Create minimal 6-color palette for renderer-only tests (when world crate not available)
  - `get_material_color(value: i32) -> Vec3` helper
- `crates/renderer/src/renderer.rs`:
  - Replace `calculate_lighting()` to accept material color and use simplified formula
  - Add lighting constants: `LIGHT_DIR`, `AMBIENT`, `BACKGROUND_COLOR`
- `crates/renderer/src/cpu_tracer.rs`:
  - Use voxel value to lookup material color (from renderer materials or fallback)
  - Apply standardized lighting
  - Support optional lighting disable
- `crates/renderer/src/gl_tracer.rs`:
  - Update background clear color to `BACKGROUND_COLOR`
- `crates/renderer/src/shaders/octree_raycast.frag`:
  - Add minimal palette (6 colors) as GLSL constant
  - Replace lighting code to use material colors
  - Update background color
  - Add `u_disable_lighting` uniform
- `crates/renderer/src/gpu_tracer.rs`:
  - Same material palette as GL shader (if implemented)
- `crates/renderer/src/scenes/octa_cube.rs`:
  - Update octant values to use materials 1-6 (Red, Green, Blue, Yellow, White, White)
- `crates/renderer/tests/*`:
  - Add color verification tests for each tracer
  - Test specific octant colors, background color, lighting toggle

### Testing Scope
- **Color correctness tests**: Render fixed scene, sample pixels, verify colors match expected values
- **Per-tracer parity**: All tracers produce same colors (within tolerance)
- **Lighting toggle test**: Verify `--no-lighting` mode outputs pure palette colors
- **Visual regression**: Existing tests continue to pass (but colors will change)

### Public API Changes
- **RenderRequest**: Add optional `disable_lighting: bool` field
- **Material constant**: Exported `MATERIAL_PALETTE` for external use
- **Lighting constant**: Exported `LIGHT_DIR`, `AMBIENT`, `BACKGROUND_COLOR`

### Breaking Changes
- **Visual output changes**: All rendered images will have different colors (orangeish → palette colors)
- **Octa cube structure**: Test scene now has 5 different colors instead of uniform appearance
- **Existing visual tests**: Any tests comparing exact pixel values will need updates

### Success Criteria
- All 3 tracers (CPU, GL, GPU) render octa cube with correct material colors
- Color verification tests pass for each tracer with <5 RGB error
- Lighting can be disabled via flag for debug/testing
- Background color is consistent bluish gray across tracers
- Documentation explains R2G3B2 palette system and lighting model
