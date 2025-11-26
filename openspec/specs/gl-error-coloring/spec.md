# gl-error-coloring Specification

## Purpose
TBD - created by archiving change add-gl-renderer-error-coloring. Update Purpose after archive.
## Requirements
### Requirement: Error Material Constants
The fragment shader SHALL define material values 1-7 as error indicators with distinct visual encoding.

#### Scenario: Error material enumeration
- **WHEN** fragment shader is compiled
- **THEN** material value 1 = Generic error (hot pink)
- **AND** material value 2 = Bounds/pointer errors (red-orange)
- **AND** material value 3 = Type validation errors (orange)
- **AND** material value 4 = Stack/iteration errors (sky blue)
- **AND** material value 5 = Octant errors (purple)
- **AND** material value 6 = Data truncation errors (spring green)
- **AND** material value 7 = Unknown errors (yellow)

#### Scenario: Error material to color mapping
- **WHEN** getErrorMaterialColor() is called with material value 1
- **THEN** it returns vec3(1.0, 0.0, 0.3) (hot pink)
- **AND** material 2 returns vec3(1.0, 0.2, 0.0) (red-orange)
- **AND** material 3 returns vec3(1.0, 0.6, 0.0) (orange)
- **AND** material 4 returns vec3(0.0, 0.8, 1.0) (sky blue)
- **AND** material 5 returns vec3(0.6, 0.0, 1.0) (purple)
- **AND** material 6 returns vec3(0.0, 1.0, 0.5) (spring green)
- **AND** material 7 returns vec3(1.0, 1.0, 0.0) (yellow)

### Requirement: Animated Checkered Pattern
Error materials SHALL display an animated 8x8 checkered pattern with brightness oscillation.

#### Scenario: Checkered pattern generation
- **WHEN** applyErrorAnimation() is called with base color and pixel coordinates
- **THEN** it divides screen into 8x8 pixel grid cells
- **AND** alternates light/dark cells in checkerboard pattern
- **AND** applies sine wave brightness oscillation (0.3 to 1.0 range)
- **AND** flips light/dark pattern every half-cycle (every second)

#### Scenario: Animation timing
- **WHEN** time uniform advances
- **THEN** brightness oscillates with sin(time * π) formula
- **AND** pattern flips when fract(time * 0.5) > 0.5
- **AND** full cycle completes every 2 seconds
- **AND** animation is smooth and continuous

### Requirement: HitInfo Material Value Tracking
The HitInfo struct SHALL use the value field to store material values including error materials.

#### Scenario: HitInfo structure definition
- **WHEN** HitInfo is defined
- **THEN** it has field: bool hit
- **AND** has field: float t
- **AND** has field: vec3 point
- **AND** has field: vec3 normal
- **AND** has field: int value (stores material value, including error materials 1-7)

#### Scenario: HitInfo initialization for errors
- **WHEN** error occurs during traversal
- **THEN** HitInfo.value is set to error material value (1-7)
- **AND** HitInfo.hit may be true or false depending on error type
- **AND** error material is rendered with animated pattern

### Requirement: Bounds Checking with Error Materials
BCF buffer reads SHALL detect and report out-of-bounds access via error material values.

#### Scenario: readU8 bounds validation
- **WHEN** readU8(offset, error_material) is called with offset >= u_octree_data_size
- **THEN** error_material is set to 2u (material 2 = bounds/pointer errors)
- **AND** function returns 0u
- **AND** caller propagates error material to result

#### Scenario: readU8 successful read
- **WHEN** readU8(offset, error_material) is called with valid offset
- **THEN** error_material remains unchanged (0 if no prior error)
- **AND** function returns byte value from texture at offset
- **AND** value is in range 0-255

### Requirement: Type Validation with Error Materials
BCF type byte parsing SHALL detect and report invalid type IDs via error materials.

#### Scenario: Valid type IDs
- **WHEN** parseBcfNode encounters type_id 0 (extended leaf)
- **THEN** error_material remains unchanged
- **AND** WHEN type_id is 1 (octa-with-leaves), error_material remains unchanged
- **AND** WHEN type_id is 2 (octa-with-pointers), error_material remains unchanged

#### Scenario: Invalid type IDs
- **WHEN** parseBcfNode encounters type_id in range 3-7
- **THEN** error_material is set to 3u (material 3 = type validation)
- **AND** function returns error material value as result
- **AND** child_offset is set to 0u

#### Scenario: Inline leaf type
- **WHEN** parseBcfNode encounters type byte with MSB=0 (inline leaf)
- **THEN** error_material remains unchanged
- **AND** function returns value extracted from lower 7 bits

### Requirement: Pointer Validation with Error Materials
Pointer reading SHALL validate offsets point within BCF data bounds.

#### Scenario: Valid pointer within bounds
- **WHEN** readPointer reads pointer that points to offset < u_octree_data_size
- **THEN** error_material remains unchanged
- **AND** function returns pointer value

#### Scenario: Invalid pointer beyond bounds
- **WHEN** readPointer reads pointer that points to offset >= u_octree_data_size
- **THEN** subsequent readU8 sets error_material to 2u (bounds/pointer error)
- **AND** error propagates to caller
- **AND** traversal terminates with error material

#### Scenario: Truncated multi-byte pointer
- **WHEN** reading 2-byte pointer starting at offset u_octree_data_size - 1
- **THEN** second byte read sets error_material to 2u
- **AND** function returns 0u

### Requirement: Stack Overflow Detection
Traversal SHALL detect and report stack overflow via error material.

#### Scenario: Normal stack usage
- **WHEN** traversal pushes to stack with stack_ptr + 1 < MAX_STACK
- **THEN** error_material remains unchanged
- **AND** child node is pushed to stack
- **AND** traversal continues

#### Scenario: Stack overflow
- **WHEN** traversal attempts to push when stack_ptr + 1 >= MAX_STACK
- **THEN** result.hit is set to true
- **AND** result.value is set to 4 (material 4 = stack/iteration errors)
- **AND** traversal terminates immediately
- **AND** function returns HitInfo with error material

### Requirement: Iteration Timeout Detection
Traversal SHALL detect and report when iteration limit is exceeded via error material.

#### Scenario: Normal iteration count
- **WHEN** traversal completes with iter < MAX_ITERATIONS
- **THEN** error_material remains unchanged
- **AND** result is based on whether voxel was hit

#### Scenario: Iteration timeout
- **WHEN** traversal loop reaches iter >= MAX_ITERATIONS
- **THEN** result.hit is set to true
- **AND** result.value is set to 4 (material 4 = stack/iteration errors)
- **AND** loop terminates
- **AND** HitInfo is returned with error material

### Requirement: Error Propagation Through Call Stack
Errors SHALL propagate from low-level functions up to main() via material values.

#### Scenario: readU8 error propagation
- **WHEN** readU8 sets error_material to 2u (bounds exceeded)
- **AND** caller is parseBcfNode
- **THEN** parseBcfNode checks error_material
- **AND** parseBcfNode returns error material value if error_material != 0
- **AND** error propagates to raycastBcfOctree
- **AND** raycastBcfOctree sets HitInfo.value to error material
- **AND** main() receives HitInfo with error material value

#### Scenario: No error propagation
- **WHEN** all functions complete without error
- **THEN** error_material remains 0 throughout call stack
- **AND** HitInfo.value contains normal material value (0, 8-127, 128-255)
- **AND** main() renders normal color with lighting

### Requirement: Error Display Always On
Error materials SHALL always be rendered with animated pattern (no toggle required).

#### Scenario: Error material rendering
- **WHEN** HitInfo.value is in range 1-7
- **THEN** getMaterialColor() calls getErrorMaterialColor(value)
- **AND** applies applyErrorAnimation() with time uniform
- **AND** FragColor is set to animated error color
- **AND** lighting is skipped for error materials

#### Scenario: Normal material rendering
- **WHEN** HitInfo.value is 0, 8-127, or 128-255
- **THEN** getMaterialColor() uses palette or R2G3B2 encoding
- **AND** lighting is applied normally
- **AND** error animation is not applied

### Requirement: Material Selector UI Integration
Application UI SHALL provide material selector for depth 0 model testing.

#### Scenario: Material selector for Single Red Voxel
- **WHEN** current_model == TestModel::SingleRedVoxel
- **THEN** UI displays "Material:" label
- **AND** shows combo box with 13 preset materials
- **AND** shows slider for value 0-255
- **AND** changing material reloads scene with new Cube::Solid(material_value)

#### Scenario: Material preset selection
- **WHEN** user selects preset from combo box
- **THEN** single_voxel_material field is updated
- **AND** scene is reloaded with new material value
- **AND** GL and GPU renderers are reinitialized

#### Scenario: Available presets
- **WHEN** material selector is displayed
- **THEN** presets include: 0 (Empty), 1-7 (Error materials), 10/50/100 (Palette), 224/252 (R2G3B2)
- **AND** each preset has descriptive label
- **AND** slider allows fine control 0-255

### Requirement: Responsive UI Layout
UI controls SHALL wrap to next line when horizontal space is insufficient.

#### Scenario: Horizontal wrapped layout
- **WHEN** top panel second row is rendered
- **THEN** it uses ui.horizontal_wrapped() layout
- **AND** controls wrap to next line if window is narrow
- **AND** includes: Manual Camera checkbox, Disable Lighting, Show GL Errors, Model selector, Material selector

#### Scenario: Window resize behavior
- **WHEN** user resizes window to be narrower
- **THEN** UI elements automatically wrap to next line
- **AND** layout remains functional and readable
- **AND** no controls are clipped or hidden

### Requirement: Error Color Distinctness
Error material colors SHALL be visually distinct from each other and from normal rendering.

#### Scenario: Color family separation
- **WHEN** different error materials are displayed
- **THEN** hot pink (1) is distinct from red-orange (2)
- **AND** orange (3) is distinct from yellow (7)
- **AND** sky blue (4) is distinct from purple (5)
- **AND** spring green (6) has unique hue
- **AND** all error colors are saturated and bright

#### Scenario: Primary color separation
- **WHEN** errors of different categories are displayed
- **THEN** RED family (1, 2) uses red channel dominance
- **AND** BLUE family (4, 5) uses blue channel dominance
- **AND** YELLOW/GREEN family (6, 7) uses red+green or green dominance
- **AND** no error color appears in normal material palette

### Requirement: Performance Impact
Error checking and animation SHALL add minimal performance overhead.

#### Scenario: Overhead measurement
- **WHEN** rendering with error materials animated
- **AND** rendering same scene with normal materials
- **THEN** frame time increase is < 5%
- **AND** animation calculations are GPU-optimized
- **AND** error material checks are branch-predictable

#### Scenario: Fast path optimization
- **WHEN** no errors occur (common case)
- **THEN** error_material checks compile efficiently
- **AND** performance is comparable to non-error-checking version
- **AND** modern GPU branch prediction handles checks efficiently

### Requirement: Error-Based Debugging Workflow
Error visualization SHALL enable systematic debugging of traversal issues.

#### Scenario: Identify depth 2+ bug
- **WHEN** developer loads ExtendedOctaCube (depth 2) model
- **AND** observes error material color displayed
- **THEN** specific error color indicates problem type (e.g., sky blue = stack/iteration)
- **AND** developer knows to investigate stack management or iteration limits
- **AND** debugging time is reduced significantly

#### Scenario: Validate BCF serialization
- **WHEN** developer sees orange error (material 3)
- **THEN** developer knows BCF serialization produced invalid type bytes
- **AND** can add test to verify BCF output
- **AND** can compare against BCF spec

#### Scenario: Diagnose bounds issues
- **WHEN** developer sees red-orange error (material 2)
- **THEN** developer knows pointer arithmetic or offset calculation is wrong
- **AND** can add bounds checking assertions
- **AND** can log BCF data size and access patterns

### Requirement: Material System Integration
Error materials SHALL integrate seamlessly with existing material system.

#### Scenario: Material value ranges
- **WHEN** shader processes material values
- **THEN** value 0 = empty space (background color)
- **AND** values 1-7 = error materials (animated)
- **AND** values 8-127 = palette materials (lookup in texture)
- **AND** values 128-255 = R2G3B2 encoded colors

#### Scenario: getMaterialColor dispatch
- **WHEN** getMaterialColor() is called
- **THEN** it checks if value is in range 1-7
- **AND** routes to getErrorMaterialColor() if true
- **AND** applies applyErrorAnimation() for error materials
- **AND** routes to palette or R2G3B2 otherwise

### Requirement: Time Uniform Propagation
Animation SHALL receive time value from CPU via uniform.

#### Scenario: Time uniform setup
- **WHEN** GL tracer initializes
- **THEN** u_time uniform location is retrieved
- **AND** stored in gl_state.time_location
- **AND** available for render calls

#### Scenario: Time uniform update
- **WHEN** render_to_gl() is called
- **THEN** current time is passed as parameter
- **AND** gl.uniform_1_f32(time_location, time) is called
- **AND** shader receives time for animation calculations

### Requirement: Lighting Skip for Error Materials
Error materials SHALL not receive lighting calculations.

#### Scenario: Error material lighting check
- **WHEN** main() shader function renders pixel
- **AND** materialColor is computed from getMaterialColor()
- **THEN** it checks if value is in range 1-7
- **AND** sets is_error_material = true if so
- **AND** skips lighting calculations (no diffuse, no Blinn-Phong)

#### Scenario: Normal material lighting
- **WHEN** material value is not in error range
- **THEN** is_error_material = false
- **AND** lighting is applied normally
- **AND** diffuse and specular terms are computed

### Requirement: Shader Uniform Compatibility
Existing u_show_errors uniform SHALL remain for backward compatibility.

#### Scenario: Unused uniform presence
- **WHEN** shader is compiled
- **THEN** u_show_errors uniform is declared
- **AND** uniform location is stored in Rust code
- **AND** uniform is not used in shader logic

#### Scenario: Future extension readiness
- **WHEN** future feature needs error toggle
- **THEN** u_show_errors uniform is available
- **AND** can be integrated into rendering logic
- **AND** no shader recompilation needed

