# Change: Add Error Coloring for GL Renderer Fragment Shader

**Implementation Note:** The actual implementation uses a material-based approach (material values 1-7 as error indicators) instead of a separate error_code enum system. See `specs/gl-error-coloring/spec.md` for the complete as-implemented specification with animated checkered patterns.

## Why

The GL renderer (WebGL 2.0 fragment shader raytracer) currently fails silently when encountering errors during BCF octree traversal. Investigation shows:

**Current Failure Mode:**
- Depth 0 (single voxel): Works ✓
- Depth 1 (octa-with-leaves): Works ✓
- Depth 2+ (nested octrees with pointers): **Silently fails** - renders background color
- No visual indication of what error occurred or where in the traversal it failed
- Debugging requires recompiling shader with manual printf statements (not available in WebGL)

**Why This Matters:**
- GL renderer is a validation raytracer used for correctness testing alongside CPU and GPU raytracers
- Silent failures make debugging BCF traversal logic extremely difficult
- Cannot distinguish between: empty space, traversal errors, bounds violations, invalid pointers, stack overflow, iteration timeout, or corrupt BCF data
- Users cannot diagnose whether issue is in BCF serialization, shader traversal, or texture upload

**Current Pain Points:**
1. **Recursive traversal fails** (depth 2+ doesn't work) but renders as empty - no error indication
2. **Bounds checking** returns 0 on out-of-bounds reads - indistinguishable from valid empty voxel
3. **Pointer following** may read invalid offsets - silent failure, no feedback
4. **Stack overflow** may occur (MAX_STACK=16) - no visual indication
5. **Iteration timeout** may trigger (MAX_ITERATIONS=512) - returns miss, looks like empty space
6. **Invalid BCF type IDs** (types 3-7 undefined) - return 0, silent failure

**Root Cause:**
Fragment shaders cannot use printf or debuggers. The only output is the pixel color. Currently, all error conditions result in the same output: background color (0.4, 0.5, 0.6), making errors invisible.

**Example Scenario:**
```glsl
// Current code at line 199-201
if (offset >= u_octree_data_size) {
    return 0u;  // Out of bounds! But how would we know?
}
```

This returns 0 (empty voxel) when reading beyond buffer bounds. The calling code treats this as "no hit" and continues traversal. Result: silent failure.

## What Changes

### Add Error Material System with Animated Visualization

Implement a material-based error reporting system using distinct animated colors to visually indicate different failure modes in the fragment shader.

**Design Decision: Material Values as Errors**

Instead of a separate error_code field, we use material values 1-7 as error indicators. This integrates seamlessly with the existing material system and provides a unified rendering path.

**Error Material Mapping:**
1. **Material 1**: Generic error (hot pink) - Catch-all for undefined errors
2. **Material 2**: Bounds/pointer errors (red-orange) - BCF buffer access violations
3. **Material 3**: Type validation errors (orange) - Invalid BCF type IDs
4. **Material 4**: Stack/iteration errors (sky blue) - Resource limit exhaustion
5. **Material 5**: Octant errors (purple) - Invalid octree navigation
6. **Material 6**: Data truncation errors (spring green) - Incomplete multi-byte reads
7. **Material 7**: Unknown errors (yellow) - Unexpected failure modes

**Implementation Approach:**
- Use material values 1-7 as error indicators (not separate error_code enum)
- Propagate errors via `inout uint error_material` parameter
- Add animated 8x8 checkered pattern with brightness oscillation
- Error materials always display animated pattern (no toggle required)
- Skip lighting calculations for error materials
- Add material selector UI for testing error materials on depth 0 model

### Phase 1: Add Error Material Colors and Animation

**File:** `crates/renderer/src/shaders/octree_raycast.frag`

Add error material color mapping and animated pattern:

```glsl
// Map error material values to distinct colors
vec3 getErrorMaterialColor(int material_value) {
    if (material_value == 1) return vec3(1.0, 0.0, 0.3);      // Hot pink - Generic error
    if (material_value == 2) return vec3(1.0, 0.2, 0.0);      // Red-orange - Bounds/pointer
    if (material_value == 3) return vec3(1.0, 0.6, 0.0);      // Orange - Type validation
    if (material_value == 4) return vec3(0.0, 0.8, 1.0);      // Sky blue - Stack/iteration
    if (material_value == 5) return vec3(0.6, 0.0, 1.0);      // Purple - Octant errors
    if (material_value == 6) return vec3(0.0, 1.0, 0.5);      // Spring green - Truncation
    if (material_value == 7) return vec3(1.0, 1.0, 0.0);      // Yellow - Unknown
    return vec3(1.0, 0.0, 1.0); // Fallback: bright magenta
}

// Apply animated checkered pattern
vec3 applyErrorAnimation(vec3 base_color, vec2 pixel_coord, float time) {
    // 8x8 pixel grid
    ivec2 grid_pos = ivec2(pixel_coord) / 8;

    // Brightness oscillation (0.0 to 1.0)
    float anim = sin(time * 3.14159) * 0.5 + 0.5;

    // Checkerboard pattern
    bool is_light_cell = ((grid_pos.x + grid_pos.y) % 2) == 0;

    // Flip pattern every half cycle
    bool flip_pattern = fract(time * 0.5) > 0.5;
    if (flip_pattern) is_light_cell = !is_light_cell;

    // Apply brightness (dark: 0.3-0.7, light: 0.7-1.0)
    float brightness = is_light_cell ? mix(0.7, 1.0, anim) : mix(0.3, 0.7, anim);

    return base_color * brightness;
}
```

### Phase 2: Update HitInfo Structure

Modify `HitInfo` to track error state:

```glsl
struct HitInfo {
    bool hit;
    float t;
    vec3 point;
    vec3 normal;
    int value;
    uint error_code;  // NEW: error code if traversal failed
};
```

### Phase 3: Add Error Checking to BCF Functions

**Update `readU8()` to track bounds violations:**

```glsl
uint readU8(uint offset, out uint error_code) {
    error_code = ERROR_NONE;
    if (offset >= u_octree_data_size) {
        error_code = ERROR_BOUNDS_EXCEEDED;
        return 0u;
    }
    return texelFetch(u_octree_data, ivec2(int(offset), 0), 0).r;
}
```

**Update `parseBcfNode()` to propagate errors:**

```glsl
uint parseBcfNode(uint offset, uint octant, out uint child_offset, out uint error_code) {
    error_code = ERROR_NONE;
    child_offset = 0u;

    uint type_byte = readU8(offset, error_code);
    if (error_code != ERROR_NONE) return 0u;

    uint msb, type_id, size_val;
    decodeTypeByte(type_byte, msb, type_id, size_val);

    // ... existing node parsing ...

    // Check for invalid type IDs (types 3-7 undefined)
    if (type_id >= 3u && type_id <= 7u) {
        error_code = ERROR_INVALID_TYPE_ID;
        return 0u;
    }

    // ... rest of function
}
```

### Phase 4: Update Raycast Functions

**Track errors in traversal loops:**

```glsl
HitInfo raycastBcfOctree(vec3 pos, vec3 dir) {
    HitInfo result;
    result.hit = false;
    result.error_code = ERROR_NONE;
    // ... initialization ...

    while (stack_ptr >= 0 && iter < MAX_ITERATIONS) {
        iter++;

        // ... existing traversal ...

        uint error_code;
        uint value = parseBcfNode(node_offset, octant_idx, child_offset, error_code);

        if (error_code != ERROR_NONE) {
            result.error_code = error_code;
            return result;  // Return with error color
        }

        // Check stack overflow
        if (stack_ptr + 1 >= MAX_STACK) {
            result.error_code = ERROR_STACK_OVERFLOW;
            return result;
        }

        // ... rest of loop
    }

    // Iteration timeout
    if (iter >= MAX_ITERATIONS) {
        result.error_code = ERROR_ITERATION_TIMEOUT;
    }

    return result;
}
```

### Phase 5: Add Error Visualization Uniform

**Add uniform to toggle error display:**

```glsl
uniform bool u_show_errors; // If true, show error colors; if false, background color
```

**Update main() to use error colors:**

```glsl
void main() {
    // ... camera setup ...

    HitInfo octreeHit = raycastBcfOctree(ray.origin, ray.direction);

    vec3 color = vec3(0.4, 0.5, 0.6); // Background

    // Check for errors first
    if (u_show_errors && octreeHit.error_code != ERROR_NONE) {
        color = errorToColor(octreeHit.error_code);
    } else if (octreeHit.hit) {
        // Normal rendering
        vec3 materialColor = getMaterialColor(octreeHit.value);
        // ... lighting ...
    }

    FragColor = vec4(color, 1.0);
}
```

### Phase 6: Add Rust-Side Error Toggle

**File:** `crates/renderer/src/gl_tracer.rs`

Add field to `GlTracerGl`:

```rust
pub struct GlTracerGl {
    // ... existing fields ...
    show_errors_location: Option<UniformLocation>,
}
```

Add method to toggle error display:

```rust
impl GlCubeTracer {
    pub fn set_show_errors(&mut self, show_errors: bool) {
        self.show_errors = show_errors;
    }
}
```

Update render functions to set uniform:

```rust
if let Some(loc) = &self.show_errors_location {
    gl.uniform_1_i32(Some(loc), if show_errors { 1 } else { 0 });
}
```

### Phase 7: Add UI Toggle

**File:** `crates/renderer/src/egui_app.rs`

Add checkbox to control panel:

```rust
ui.checkbox(&mut self.show_gl_errors, "Show GL Errors");
```

Pass to renderer:

```rust
self.gl_renderer.set_show_errors(self.show_gl_errors);
```

## Success Criteria

1. **Error Visibility**: Each error type displays a distinct color:
   - Bounds exceeded: Bright red
   - Invalid pointer: Dark red
   - Invalid type ID: Orange
   - Stack overflow: Blue
   - Iteration timeout: Cyan
   - Invalid octant: Magenta

2. **Debugging Workflow**: Developer can:
   - Enable error display via UI checkbox
   - Identify which error is occurring from the color
   - Localize the bug to specific BCF operations
   - Distinguish error from valid empty space

3. **Toggle-able**: Error display can be disabled to show normal rendering behavior

4. **Performance**: Error checking adds negligible overhead (<1% frame time)

5. **Current Bug**: With error coloring enabled, depth 2+ models will show **which specific error** causes the failure (likely ERROR_INVALID_POINTER or ERROR_STACK_OVERFLOW)

## Non-Goals

- **Full debugger**: This is not a step-through debugger, just error visualization
- **Error recovery**: Errors are reported, not recovered from
- **Performance profiling**: No timing or performance metrics
- **CPU parity checking**: No automatic comparison with CPU raytracer output

## Dependencies

- Requires working depth 0/1 rendering (already implemented)
- No new external dependencies
- Uses existing GLSL and OpenGL ES 3.0 features

## Testing Strategy

1. **Unit Tests**: Create test scenes that trigger each error type
2. **Visual Validation**: Verify each error shows correct color
3. **Regression**: Ensure depth 0/1 models still render correctly
4. **Debug Session**: Use error colors to identify and fix depth 2+ bug

## Future Extensions

- Error overlay text (requires additional rendering pass)
- Error statistics (count of each error type)
- Error heatmap (blend error color with distance field)
- Configurable error colors (via uniforms)
