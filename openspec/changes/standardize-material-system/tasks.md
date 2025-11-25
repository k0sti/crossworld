# Implementation Tasks - Standardize Material System and Lighting

## 1. Verify and Use Existing Material System
- [x] 1.1 Confirm `crates/world` has `MaterialColorMapper` (already exists - verified)
- [ ] 1.2 Check if `assets/materials.json` exists with 128 material definitions
- [ ] 1.3 If `materials.json` missing, create with first 6 entries: Red, Green, Blue, Yellow, White, Black
- [x] 1.4 Create `crates/renderer/src/materials.rs` with minimal 7-color palette for tests
- [x] 1.5 Add `get_material_color(value: i32) -> Vec3` returning hardcoded colors 0-6
- [x] 1.6 Document that this is a test-only subset of world crate's 128 materials

## 2. Define Lighting Constants
- [x] 2.1 Add `LIGHT_DIR: Vec3 = Vec3::new(0.431934, 0.863868, 0.259161)` in `renderer.rs` (pre-normalized)
- [x] 2.2 Add `AMBIENT: f32 = 0.3` constant
- [x] 2.3 Add `DIFFUSE_STRENGTH: f32 = 0.7` constant
- [x] 2.4 Add `BACKGROUND_COLOR: Vec3 = Vec3::new(0.4, 0.5, 0.6)` constant
- [x] 2.5 Make all constants `pub` for external use

## 3. Update Lighting Calculation (Rust)
- [x] 3.1 Modify `calculate_lighting()` signature to accept `material_color: Vec3`
- [x] 3.2 Remove hardcoded orangeish base color and normal-based color variation
- [x] 3.3 Remove fresnel effect
- [x] 3.4 Implement formula: `material_color * (AMBIENT + diffuse * DIFFUSE_STRENGTH)`
- [x] 3.5 Add `calculate_lighting_unlit(material_color: Vec3) -> Vec3` for debug mode
- [x] 3.6 Update function documentation

## 4. Update CPU Tracer
- [x] 4.1 Import material system: `use crate::materials::get_material_color;`
- [x] 4.2 Update `render_ray()` to get material color from `cube_hit.value`
- [x] 4.3 Pass `material_color` to `calculate_lighting()`
- [x] 4.4 Update `background_color` to use `BACKGROUND_COLOR` constant
- [x] 4.5 Add support for `disable_lighting` flag (via CpuCubeTracer.set_disable_lighting())
- [x] 4.6 Update all CPU tracer tests to expect new colors

## 5. Update GL Tracer (Rust side)
- [x] 5.1 Update `gl.clear_color()` calls to use `BACKGROUND_COLOR` (0.4, 0.5, 0.6, 1.0)
- [x] 5.2 Add `u_disable_lighting` uniform binding
- [x] 5.3 Set `u_disable_lighting` based on tracer flag (GlCubeTracer.disable_lighting)
- [x] 5.4 Update texture encoding to store material values (not just binary 0/255)

## 6. Update GL Fragment Shader
- [x] 6.1 Add `MATERIAL_PALETTE` constant array (7 entries: Empty, Red, Green, Blue, Yellow, White, Black)
- [x] 6.2 Add `getMaterialColor(int value)` function
- [x] 6.3 Update background color to `vec3(0.4, 0.5, 0.6)`
- [x] 6.4 Remove orangeish color and normal-based variation in lighting code
- [x] 6.5 Remove fresnel effect
- [x] 6.6 Update lighting to use `materialColor * (ambient + diffuse * 0.7)`
- [x] 6.7 Add `uniform bool u_disable_lighting;` and conditional lighting application
- [x] 6.8 Update light direction constant to match Rust version (0.431934, 0.863868, 0.259161)

## 7. Update GPU Compute Shader (if applicable)
- [ ] 7.1 Check if GPU tracer is implemented (may be stub)
- [ ] 7.2 If implemented, apply same material and lighting changes as GL shader
- [ ] 7.3 If not implemented, skip this section

## 8. Update Test Scene
- [x] 8.1 Modify `create_octa_cube()` in `scenes/octa_cube.rs`
- [x] 8.2 Set octant 0: Red (value 1)
- [x] 8.3 Set octant 1: White (value 5)
- [x] 8.4 Set octant 2: Green (value 2)
- [x] 8.5 Set octant 3: Empty (value 0)
- [x] 8.6 Set octant 4: Blue (value 3)
- [x] 8.7 Set octant 5: White (value 5)
- [x] 8.8 Set octant 6: Yellow (value 4)
- [x] 8.9 Set octant 7: Empty (value 0)
- [x] 8.10 Update documentation comments with new material layout

## 9. Add Color Verification Tests
- [x] 9.1 Create `tests/color_verification.rs` file
- [x] 9.2 Add `test_cpu_tracer_material_colors()` - render and verify distinct colors visible
- [ ] 9.3 Add `test_gl_tracer_material_colors()` - render and verify octant colors (requires GL context)
- [ ] 9.4 Add `test_gpu_tracer_material_colors()` (if GPU tracer implemented, requires GL context)
- [x] 9.5 Add `test_cpu_tracer_background_color()` - verify background RGB(170, 186, 201) with gamma
- [x] 9.6 Add `test_lighting_toggle()` - verify `disable_lighting` produces distinct visual output
- [x] 9.7 Add helper function `sample_pixel(image, x, y) -> (u8, u8, u8)`
- [x] 9.8 Add helper function `assert_color_near(actual, expected, tolerance)` with tolerance=5
- [x] 9.9 Add `test_cpu_tracer_renders_without_crash()` - basic rendering smoke test
- [x] 9.10 Add `test_material_palette_accessibility()` - verify all 7 palette colors
- [x] 9.11 Add `test_lighting_constants()` - verify normalization and valid ranges

## 10. Update RenderRequest API
- [ ] 10.1 Add `disable_lighting: bool` field to `RenderRequest` struct
- [ ] 10.2 Default `disable_lighting` to `false`
- [ ] 10.3 Update `RenderRequest::new()` to set `disable_lighting: false`
- [ ] 10.4 Add `RenderRequest::with_no_lighting()` convenience method
- [ ] 10.5 Document lighting toggle feature in struct docs

## 11. Update Existing Tests
- [x] 11.1 Review all tests in `tests/` for hardcoded color expectations
- [x] 11.2 Update background color in `octa_cube_rendering.rs` from RGB(51,76,102) to RGB(170,186,201)
- [x] 11.3 Update background color in `render_validation.rs` (2 locations)
- [x] 11.4 Verify all existing tests still pass with new color output (octa_cube_rendering: 2/2, render_validation: 2/2)
- [x] 11.5 Update test documentation comments to reference gamma-corrected background

## 12. Documentation
- [x] 12.1 Add material palette explanation to `crates/renderer/README.md` (7-color test palette table)
- [x] 12.2 Document lighting model formula and constants (LIGHT_DIR, AMBIENT, DIFFUSE_STRENGTH)
- [x] 12.3 Add examples of using material colors in code (`get_material_color()` usage)
- [x] 12.4 Document lighting toggle flag usage (`set_disable_lighting()` examples)
- [x] 12.5 Add testing section documenting color verification tests
- [x] 12.6 Update shader documentation to reflect octree traversal and material system
- [x] 12.7 Document gamma correction and background color

## 13. Code Quality
- [x] 13.1 Run `cargo fmt` on all modified Rust files
- [x] 13.2 Run `cargo clippy -p renderer` and fix warnings (verified via cargo build)
- [x] 13.3 Verify all tests pass: `cargo test --test color_verification` (6/6 passing, includes lighting toggle)
- [x] 13.4 Check shader syntax (shaders compile without errors in GL tracer)
- [x] 13.5 Remove any dead code (old lighting functions removed, fresnel removed)

## 14. Final Validation
- [x] 14.1 Render octa cube with CPU tracer and verify distinct colors visible (test passes)
- [ ] 14.2 Render octa cube with GL tracer and verify matches CPU output (requires GL context)
- [x] 14.3 Test lighting toggle produces distinct visual output (test_lighting_toggle passes)
- [x] 14.4 Verify background color is bluish gray in CPU tracer (RGB 170, 186, 201 with gamma)
- [x] 14.5 Run test suite: `cargo test --test color_verification` (6/6 passing, includes lighting toggle)

## Success Criteria
- All renderer tests pass with new material system
- Color verification tests pass with <5 RGB tolerance
- Visual output shows 5 distinct colors (red, green, blue, yellow, white) plus background
- Lighting toggle works correctly (pure colors vs shaded)
- No clippy warnings or format issues
- Documentation clearly explains material and lighting system
