## 1. Error Material Color System
- [x] 1.1 Define error material values 1-7 in fragment shader
- [x] 1.2 Create `getErrorMaterialColor()` function mapping materials to colors
- [x] 1.3 Assign distinct colors: hot pink (1), red-orange (2), orange (3), sky blue (4), purple (5), spring green (6), yellow (7)
- [x] 1.4 Ensure color distinctness across error categories

## 2. Animated Checkered Pattern
- [x] 2.1 Create `applyErrorAnimation()` function for error materials
- [x] 2.2 Implement 8x8 pixel grid pattern generation
- [x] 2.3 Add checkerboard light/dark cell alternation
- [x] 2.4 Implement brightness oscillation with sine wave (0.3 to 1.0 range)
- [x] 2.5 Add pattern flip every half-cycle (every second)
- [x] 2.6 Integrate animation with time uniform (u_time)

## 3. Material System Integration
- [x] 3.1 Update `getMaterialColor()` to handle error materials (1-7)
- [x] 3.2 Route error materials to `getErrorMaterialColor()`
- [x] 3.3 Apply animation via `applyErrorAnimation()` for error materials
- [x] 3.4 Skip lighting calculations for error materials
- [x] 3.5 Preserve existing material handling (0, 8-127, 128-255)

## 4. Error Propagation via Material Values
- [x] 4.1 Update `readU8()` to use `inout uint error_material` parameter
- [x] 4.2 Set error_material to 2u on bounds exceeded
- [x] 4.3 Update `parseBcfNode()` to propagate error_material
- [x] 4.4 Return error material value directly on type validation failure (material 3u)
- [x] 4.5 Check and propagate error_material through call stack

## 5. Stack and Iteration Error Detection
- [x] 5.1 Add stack overflow detection in `raycastBcfOctree()`
- [x] 5.2 Set result.value to 4 (material 4) on stack overflow
- [x] 5.3 Add iteration timeout detection
- [x] 5.4 Set result.value to 4 on iteration timeout
- [x] 5.5 Ensure immediate termination on resource errors

## 6. Time Uniform Integration
- [x] 6.1 Add u_time uniform declaration in fragment shader
- [x] 6.2 Retrieve time uniform location in Rust (gl_tracer.rs)
- [x] 6.3 Store time_location in GlTracerGl struct
- [x] 6.4 Pass time parameter to render_to_gl()
- [x] 6.5 Set u_time uniform value each frame via gl.uniform_1_f32()

## 7. Material Selector UI
- [x] 7.1 Add single_voxel_material field to DualRendererApp
- [x] 7.2 Initialize with default red R2G3B2 color (224)
- [x] 7.3 Create material selector combo box with 13 presets
- [x] 7.4 Add presets: 0 (Empty), 1-7 (Error materials), 10/50/100 (Palette), 224/252 (R2G3B2)
- [x] 7.5 Add numeric slider for fine control (0-255 range)
- [x] 7.6 Implement scene reload on material change
- [x] 7.7 Reinitialize GL and GPU renderers with new Cube::Solid(material_value)
- [x] 7.8 Show selector only when TestModel::SingleRedVoxel is selected

## 8. Responsive UI Layout
- [x] 8.1 Change ui.horizontal() to ui.horizontal_wrapped() for control panel
- [x] 8.2 Test wrapping behavior with narrow window
- [x] 8.3 Verify all controls remain accessible when wrapped
- [x] 8.4 Ensure no controls are clipped or hidden

## 9. Backward Compatibility
- [x] 9.1 Keep u_show_errors uniform declaration in shader
- [x] 9.2 Keep show_errors_location field in GlTracerGl struct
- [x] 9.3 Leave show_errors uniform unused but available for future extension
- [x] 9.4 Document that errors are always visible (no toggle)

## 10. Testing and Validation
- [x] 10.1 Build and run renderer with new error system
- [x] 10.2 Test error materials 1-7 display with animation
- [x] 10.3 Verify checkered pattern animates correctly
- [x] 10.4 Test material selector with SingleRedVoxel model
- [x] 10.5 Verify UI wrapping behavior
- [x] 10.6 Confirm no compilation errors or warnings (only pre-existing warnings)

## 11. Documentation
- [x] 11.1 Create OpenSpec proposal.md documenting design decision
- [x] 11.2 Create comprehensive spec.md with all requirements
- [x] 11.3 Document error material color mapping
- [x] 11.4 Document animation behavior and timing
- [x] 11.5 Document material system integration
- [x] 11.6 Add implementation note to proposal about material-based approach

## 12. Git Integration
- [x] 12.1 Commit implementation code changes
- [x] 12.2 Create descriptive commit message with error material details
- [x] 12.3 Commit OpenSpec documentation
- [x] 12.4 Verify working tree clean

## Status Notes

**Completed Implementation (2025-11-26):**
- Material-based error system implemented instead of error_code enum
- Animated checkered pattern with 2-second cycle
- 7 distinct error material colors with semantic meaning
- Material selector UI for testing on depth 0 model
- Responsive UI layout with horizontal wrapping
- All requirements from spec.md implemented and tested

**Key Design Decisions:**
- Used material values 1-7 as error indicators (not separate error_code field)
- Integrated seamlessly with existing material system (0, 8-127, 128-255)
- Error materials always animated (no toggle required)
- Lighting skipped for error materials for clear visibility

**Commits:**
- `c2fe230` - Implementation: animated error material system
- `4ed02d8` - Documentation: OpenSpec proposal and spec
