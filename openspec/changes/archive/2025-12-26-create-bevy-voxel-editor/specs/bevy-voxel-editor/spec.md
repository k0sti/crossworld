# Capability: Bevy Voxel Editor

## ADDED Requirements

### Requirement: Build Configuration
The editor SHALL provide optimized build configuration for Linux development using mold linker and cranelift codegen for fast iteration cycles.

#### Scenario: Linux build uses mold linker
- **WHEN** developer builds on Linux with mold installed
- **THEN** `.cargo/config.toml` configures mold as the linker
- **AND** incremental linking is 5-10x faster than GNU ld

#### Scenario: Linux debug build uses cranelift
- **WHEN** developer builds in debug mode with Rust nightly
- **THEN** cranelift codegen backend is used automatically
- **AND** debug build times are 2-3x faster than LLVM

#### Scenario: Nix flake provides dependencies
- **WHEN** Nix user runs `nix develop`
- **THEN** all Bevy system dependencies are available (libudev, alsa, X11, Wayland, Vulkan)
- **AND** build succeeds without manual dependency installation

### Requirement: Application Scaffold
The editor SHALL be a standalone Bevy 0.17.3 application executable named `planet` that initializes the game engine, sets up default plugins, and provides a 3D viewport with camera controls.

#### Scenario: Launch editor application
- **WHEN** user runs `cargo run --bin planet` or `just planet`
- **THEN** a window opens with title "Crossworld Voxel Editor"
- **AND** the window contains a 3D viewport showing a default empty grid
- **AND** the camera is positioned at (10, 10, 10) looking at origin

#### Scenario: Editor runs at stable framerate
- **WHEN** editor is displaying a scene with 10,000 visible voxels
- **THEN** the application maintains at least 60 FPS on a mid-range GPU
- **AND** frame time is displayed in status bar

### Requirement: Camera Modes
The editor SHALL support two camera modes: LookAt mode where camera always faces the center of the scene, and Free mode where camera can move independently.

#### Scenario: LookAt camera mode
- **WHEN** camera mode is LookAt
- **AND** user right-click drags to rotate camera
- **THEN** camera orbits around the center point of the scene
- **AND** camera always faces toward the center
- **AND** distance from center is maintained

#### Scenario: Free camera mode
- **WHEN** camera mode is Free
- **AND** user right-click drags to rotate camera
- **THEN** camera rotates in place without orbiting
- **AND** camera direction changes freely
- **AND** camera position remains fixed during rotation

#### Scenario: Toggle camera mode with C key
- **WHEN** user presses C key
- **THEN** camera mode toggles between LookAt and Free
- **AND** UI displays current camera mode
- **AND** camera behavior changes immediately

#### Scenario: Calculate scene center for LookAt mode
- **WHEN** camera mode is LookAt
- **THEN** scene center is calculated from bounding box of all voxels
- **AND** center updates when voxels are added or removed
- **AND** F key frames camera to center with appropriate distance

### Requirement: Camera Controls
The editor SHALL provide camera controls allowing users to rotate, pan, and zoom using mouse interactions, with behavior dependent on camera mode.

#### Scenario: Rotate camera with right-click hold and drag
- **WHEN** user presses and holds right mouse button
- **AND** drags mouse 100 pixels right
- **THEN** camera rotates according to current camera mode
- **AND** rotation continues as long as button is held
- **AND** rotation speed is proportional to drag distance

#### Scenario: Pan camera with middle-click drag
- **WHEN** user middle-clicks and drags mouse
- **THEN** camera and target move together in the viewport plane
- **AND** camera orientation remains unchanged

#### Scenario: Zoom camera with scroll wheel
- **WHEN** user scrolls mouse wheel up
- **THEN** camera moves closer to target point
- **AND** zoom respects minimum distance limit (1.0 unit)

### Requirement: Voxel Scene Management
The editor SHALL maintain a voxel scene using the `WorldCube` or `Cube` data structure from existing crates and synchronize it with Bevy's rendering system.

#### Scenario: Initialize empty voxel scene
- **WHEN** editor starts with no file loaded
- **THEN** a default `WorldCube` is created with depth 5
- **AND** the scene contains procedurally generated terrain (from world crate)
- **AND** the scene is rendered as a mesh in the viewport

#### Scenario: Mesh updates when voxel data changes
- **WHEN** user places a voxel in the scene
- **THEN** the system marks mesh as dirty
- **AND** mesh is regenerated on next frame via `generate_frame()`
- **AND** updated mesh is visible in viewport within 1 frame

### Requirement: Edit Ray and Raycast Face
The editor SHALL cast an edit ray from the camera through the mouse cursor position into the voxel world and determine the raycast face (hit voxel and face normal).

#### Scenario: Cast edit ray on mouse movement
- **WHEN** user moves mouse over viewport
- **THEN** a ray is cast from camera through cursor position
- **AND** raycast result includes hit position, face normal, and distance
- **AND** result is stored for cursor positioning

#### Scenario: Raycast face determination
- **WHEN** ray intersects a voxel
- **THEN** the hit voxel position is recorded
- **AND** the face normal is determined (one of: +X, -X, +Y, -Y, +Z, -Z)
- **AND** face normal points towards the camera

#### Scenario: Raycast returns empty when no intersection
- **WHEN** user points cursor at empty space with no voxels
- **THEN** raycast returns None
- **AND** cursor is hidden

### Requirement: Cube Cursor
The editor SHALL maintain a cube cursor that defines the position and size of the edit operation, positioned based on the raycast face and current focus mode.

#### Scenario: Cursor positioned in Near mode
- **WHEN** editor is in Near focus mode
- **AND** raycast hits a voxel face
- **THEN** cursor is positioned at the hit voxel position (near side of face)
- **AND** cursor wireframe is displayed at that position

#### Scenario: Cursor positioned in Far mode
- **WHEN** editor is in Far focus mode
- **AND** raycast hits a voxel face
- **THEN** cursor is positioned at hit position + face normal (far side of face)
- **AND** cursor wireframe is displayed at that position

#### Scenario: Cursor size adjustment
- **WHEN** cursor size is set to 2
- **THEN** cursor represents a 2x2x2 voxel volume
- **AND** wireframe gizmo displays 2x2x2 cube
- **AND** editing operations affect all voxels within cursor bounds

#### Scenario: Cursor visibility
- **WHEN** raycast has no intersection
- **THEN** cursor is hidden
- **WHEN** raycast hits a voxel
- **THEN** cursor is visible at calculated position

### Requirement: Focus Mode
The editor SHALL support two focus modes (Near and Far) that control whether the cursor is placed on the near side or far side of the raycast face.

#### Scenario: Near focus mode behavior
- **WHEN** focus mode is Near
- **AND** user clicks on a voxel face pointing right (+X normal)
- **THEN** cursor is at the hit voxel position
- **AND** place operation adds voxel adjacent to the right
- **AND** remove operation removes the hit voxel

#### Scenario: Far focus mode behavior
- **WHEN** focus mode is Far
- **AND** user clicks on a voxel face pointing right (+X normal)
- **THEN** cursor is at hit position + (1, 0, 0)
- **AND** place operation adds voxel one position to the right
- **AND** remove operation removes voxel at cursor position

#### Scenario: Toggle focus mode with Tab
- **WHEN** user presses Tab key
- **THEN** focus mode toggles between Near and Far
- **AND** cursor position updates immediately to reflect new mode
- **AND** status bar displays current focus mode

#### Scenario: Focus mode visual feedback
- **WHEN** focus mode is Near
- **THEN** cursor gizmo is drawn in green color
- **WHEN** focus mode is Far
- **THEN** cursor gizmo is drawn in blue color

### Requirement: Paint Modes
The editor SHALL support two paint modes: Single Material mode for painting with a selected material, and Brush mode for painting with loaded voxel models.

#### Scenario: Single Material paint mode
- **WHEN** paint mode is set to Single
- **AND** material 5 is selected
- **AND** user paints a voxel
- **THEN** voxel is created with material ID 5
- **AND** cursor shows single-voxel or sized cube preview

#### Scenario: Brush paint mode
- **WHEN** paint mode is set to Brush
- **AND** a .vox brush is selected
- **AND** user paints at cursor position
- **THEN** entire brush model is stamped at cursor position
- **AND** all voxels from brush are copied with original materials
- **AND** cursor shows brush preview mesh

#### Scenario: Toggle paint mode with M key
- **WHEN** user presses M key
- **THEN** paint mode toggles between Single and Brush
- **AND** UI updates to show active mode
- **AND** cursor visualization changes accordingly

### Requirement: Voxel Brush System
The editor SHALL support loading voxel brushes from .vox files in the assets folder and allow users to stamp these brushes into the scene.

#### Scenario: Load brushes from assets folder
- **WHEN** editor starts
- **THEN** all .vox files in `assets/avatars/` and `assets/models/` are scanned
- **AND** each file is loaded as a VoxelBrush
- **AND** brush list is populated in brush selector UI

#### Scenario: Select brush from UI
- **WHEN** user clicks on a brush in brush selector
- **THEN** selected brush becomes active
- **AND** cursor preview shows brush mesh
- **AND** paint mode automatically switches to Brush

#### Scenario: Brush scale depth parameter
- **WHEN** brush has base size of 16x16x16 voxels
- **AND** scale_depth is set to 1
- **THEN** brush is scaled to 32x32x32 (16 * 2^1)
- **AND** each voxel in brush occupies 2x2x2 space in scene

#### Scenario: Adjust brush scale with bracket keys
- **WHEN** paint mode is Brush
- **AND** user presses ] key
- **THEN** scale_depth increases by 1 (maximum 4)
- **AND** brush preview updates to show new scale

#### Scenario: Copy brush from editor content
- **WHEN** user positions cursor over existing voxel content
- **AND** user presses Ctrl+C
- **THEN** voxels within cursor volume are copied to clipboard
- **AND** copied content becomes a temporary brush
- **AND** paint mode switches to Brush with copied content

### Requirement: Voxel Placement
The editor SHALL allow users to place voxels at the cube cursor position using the currently selected material or brush, with support for multi-voxel placement based on cursor size or brush geometry.

#### Scenario: Place single voxel with left-click
- **WHEN** cursor size is 1
- **AND** user left-clicks with cursor positioned via raycast
- **THEN** a new voxel is placed at cursor position
- **AND** the new voxel uses the currently selected material ID
- **AND** the mesh updates to show the new voxel

#### Scenario: Place multiple voxels with larger cursor
- **WHEN** cursor size is 2 (2x2x2 volume)
- **AND** user left-clicks
- **THEN** 8 voxels are placed filling the 2x2x2 cursor volume
- **AND** all voxels use the currently selected material ID
- **AND** the mesh updates to show all new voxels

#### Scenario: Placement respects focus mode
- **WHEN** focus mode is Near
- **AND** user clicks on a voxel face
- **THEN** voxels are placed at the near side of the face
- **WHEN** focus mode is Far
- **AND** user clicks on the same face
- **THEN** voxels are placed at the far side of the face

#### Scenario: Place brush at cursor position
- **WHEN** paint mode is Brush
- **AND** user left-clicks at cursor position
- **THEN** all voxels from brush model are placed at cursor position
- **AND** each voxel retains its original material from brush
- **AND** brush is transformed according to scale_depth

#### Scenario: Continuous paint mode on click and hold
- **WHEN** user left-clicks and holds mouse button down
- **AND** moves cursor to a new voxel coordinate
- **THEN** paint operation is performed at new coordinate
- **AND** painting continues as long as button is held and coordinates change
- **AND** no duplicate painting occurs at same coordinate

#### Scenario: Continuous paint deduplication
- **WHEN** user is in continuous paint mode
- **AND** cursor position changes from (5, 10, 5) to (6, 10, 5)
- **THEN** paint occurs at (6, 10, 5)
- **WHEN** cursor remains at (6, 10, 5) for multiple frames
- **THEN** no additional paint operations occur (deduplication)

#### Scenario: Place voxel respects depth setting
- **WHEN** user has depth set to 5 (micro depth)
- **AND** user places a voxel
- **THEN** `set_voxel_at_depth(x, y, z, 5, material_id)` is called
- **AND** voxel appears at the correct depth level

### Requirement: Voxel Removal
The editor SHALL allow users to remove voxels at the cube cursor position, with support for multi-voxel removal based on cursor size.

#### Scenario: Remove single voxel with shift-left-click
- **WHEN** cursor size is 1
- **AND** user holds Shift and left-clicks
- **THEN** voxel at cursor position is removed from the scene
- **AND** `remove_voxel_at_depth(x, y, z, depth)` is called
- **AND** the mesh updates to reflect removal

#### Scenario: Remove multiple voxels with larger cursor
- **WHEN** cursor size is 3 (3x3x3 volume)
- **AND** user holds Shift and left-clicks
- **THEN** all voxels within the 3x3x3 cursor volume are removed
- **AND** the mesh updates to reflect all removals

#### Scenario: Removal respects focus mode
- **WHEN** focus mode is Near
- **AND** user shift-clicks on a voxel
- **THEN** voxels at the near side are removed
- **WHEN** focus mode is Far
- **AND** user shift-clicks
- **THEN** voxels at the far side are removed

#### Scenario: Remove voxel with Delete key
- **WHEN** cursor is positioned via raycast
- **AND** user presses Delete key
- **THEN** voxels within cursor volume are removed

### Requirement: Material Palette Panel
The editor SHALL provide a material palette panel displaying all available materials with their colors and names, allowing users to select which material to use in Single paint mode.

#### Scenario: Display all materials in palette
- **WHEN** editor loads materials from `assets/materials.json`
- **THEN** palette panel shows a scrollable grid of all material swatches
- **AND** each swatch displays the material color as a colored square
- **AND** hovering over a swatch shows material name tooltip
- **AND** material ID is displayed on each swatch

#### Scenario: Select material from palette
- **WHEN** paint mode is Single
- **AND** user clicks on a material swatch
- **THEN** the selected material becomes the active placement material
- **AND** the swatch is highlighted with a border
- **AND** newly placed voxels use this material ID

#### Scenario: Material palette disabled in Brush mode
- **WHEN** paint mode is Brush
- **THEN** material palette is disabled (grayed out)
- **AND** clicking materials has no effect
- **AND** brush materials are used instead

#### Scenario: Keyboard shortcuts for first 9 materials
- **WHEN** paint mode is Single
- **AND** user presses keys 1-9
- **THEN** the corresponding material (index 0-8) is selected
- **AND** paint mode switches to Single if in Brush mode
- **AND** UI reflects the selection

### Requirement: Brush Selector Panel
The editor SHALL provide a brush selector panel displaying available voxel brushes loaded from .vox files, with scale depth controls for each brush.

#### Scenario: Display brush thumbnails
- **WHEN** brush selector panel is visible
- **THEN** all loaded brushes are displayed as thumbnail previews
- **AND** brush name is shown below each thumbnail
- **AND** current brush is highlighted

#### Scenario: Select brush from selector
- **WHEN** user clicks on a brush thumbnail
- **THEN** that brush becomes the active brush
- **AND** paint mode switches to Brush
- **AND** cursor preview shows brush mesh

#### Scenario: Adjust brush scale depth
- **WHEN** brush selector is visible
- **AND** user adjusts scale depth slider for a brush (0-4)
- **THEN** brush scale is updated to 2^scale_depth
- **AND** brush preview updates to show new size
- **AND** status bar shows scale multiplier (e.g., "Scale: 2x" for depth 1)

#### Scenario: Brush selector shows file locations
- **WHEN** user hovers over brush thumbnail
- **THEN** tooltip shows full file path (e.g., "assets/avatars/player.vox")
- **AND** shows base dimensions (e.g., "16x16x30 voxels")
- **AND** shows current scaled dimensions if scale_depth > 0

### Requirement: File Menu Operations
The editor SHALL provide file menu operations for opening, saving, and creating new voxel scenes.

#### Scenario: Create new scene
- **WHEN** user selects File → New
- **THEN** a confirmation dialog appears if unsaved changes exist
- **AND** upon confirmation, current scene is cleared
- **AND** a new default WorldCube is initialized

#### Scenario: Open CSM file
- **WHEN** user selects File → Open
- **THEN** a file dialog appears filtered to .csm files
- **AND** upon selecting a file, CSM is parsed and loaded into scene
- **AND** viewport updates to show loaded voxels

#### Scenario: Save CSM file
- **WHEN** user selects File → Save or presses Ctrl+S
- **THEN** if no current filename, save-as dialog appears
- **AND** current scene is exported via `export_to_csm()`
- **AND** CSM text is written to selected file path

#### Scenario: Open .vox file
- **WHEN** user selects File → Import → MagicaVoxel (.vox)
- **THEN** file dialog appears filtered to .vox files
- **AND** upon selection, .vox is parsed via `dot_vox` crate
- **AND** voxel data is converted to Cube and loaded into scene

### Requirement: Undo/Redo System
The editor SHALL implement an undo/redo system using the command pattern to allow users to revert and reapply editing operations.

#### Scenario: Undo voxel placement
- **WHEN** user places a voxel
- **AND** user presses Ctrl+Z
- **THEN** the voxel placement is reversed
- **AND** the scene returns to its previous state
- **AND** mesh updates to reflect undo

#### Scenario: Redo voxel placement
- **WHEN** user has undone an operation
- **AND** user presses Ctrl+Y
- **THEN** the undone operation is reapplied
- **AND** scene updates to reflect redo

#### Scenario: Undo history has depth limit
- **WHEN** user performs 1000 editing operations
- **THEN** undo history retains only the last 500 operations
- **AND** oldest operations are discarded automatically

### Requirement: Scene Inspector Panel
The editor SHALL provide an inspector panel showing scene metadata, statistics, and hierarchy information.

#### Scenario: Display scene statistics
- **WHEN** inspector panel is visible
- **THEN** it displays total voxel count
- **AND** it displays mesh triangle count
- **AND** it displays current depth level
- **AND** statistics update in real-time as scene changes

#### Scenario: Display cursor information
- **WHEN** inspector panel is visible
- **THEN** it shows cursor position (x, y, z)
- **AND** it shows cursor size (if Single mode) or brush info (if Brush mode)
- **AND** it shows focus mode (Near/Far)
- **AND** it shows paint mode (Single/Brush)
- **AND** it shows camera mode (LookAt/Free)
- **AND** values update as settings change

#### Scenario: Display camera information
- **WHEN** inspector panel is visible
- **THEN** it shows camera position (x, y, z)
- **AND** it shows camera target position
- **AND** values update as camera moves

### Requirement: Status Bar
The editor SHALL provide a status bar at the bottom of the window displaying relevant information about the current editor state, including cursor position, focus mode, and cursor size.

#### Scenario: Display cursor coordinates
- **WHEN** cursor is positioned via raycast
- **THEN** status bar shows cursor coordinates (x, y, z, depth)
- **AND** coordinates update in real-time as cursor moves

#### Scenario: Display cursor size or brush info
- **WHEN** paint mode is Single and cursor size is changed
- **THEN** status bar shows current cursor size (e.g., "Cursor: 2x2x2")
- **WHEN** paint mode is Brush
- **THEN** status bar shows brush name and scale (e.g., "Brush: player.vox (Scale: 2x)")
- **AND** updates immediately when brush or scale changes

#### Scenario: Display focus mode
- **WHEN** focus mode is toggled
- **THEN** status bar shows current mode: "Near" or "Far"
- **AND** includes keyboard shortcut hint "(Tab)"

#### Scenario: Display paint mode
- **WHEN** paint mode changes
- **THEN** status bar shows current mode: "Single" or "Brush"
- **AND** includes keyboard shortcut hint "(M)"

#### Scenario: Display camera mode
- **WHEN** camera mode changes
- **THEN** status bar shows current mode: "LookAt" or "Free"
- **AND** includes keyboard shortcut hint "(C)"

#### Scenario: Display raycast face
- **WHEN** raycast hits a voxel
- **THEN** status bar shows face normal (e.g., "Face: +X")
- **AND** updates as user moves mouse

#### Scenario: Display FPS counter
- **WHEN** status bar is visible
- **THEN** it shows current frames per second
- **AND** FPS updates every second

#### Scenario: Display current tool
- **WHEN** user switches between Place/Remove/Paint tools
- **THEN** status bar shows the active tool name
- **AND** shows keyboard shortcut hint (e.g., "Place (P)")

### Requirement: Toolbar
The editor SHALL provide a toolbar with buttons for common operations and tool selection.

#### Scenario: Tool selection buttons
- **WHEN** toolbar is visible
- **THEN** it shows buttons for Place, Remove, Paint, Select tools
- **AND** clicking a button activates that tool
- **AND** active button is visually highlighted

#### Scenario: View options
- **WHEN** user clicks View menu in toolbar
- **THEN** dropdown shows options for Grid, Wireframe, Lighting
- **AND** toggling options updates the viewport rendering

### Requirement: Keyboard Shortcuts
The editor SHALL support keyboard shortcuts for common operations to improve workflow efficiency.

#### Scenario: File operations shortcuts
- **WHEN** user presses Ctrl+N
- **THEN** New Scene operation is triggered
- **WHEN** user presses Ctrl+O
- **THEN** Open File operation is triggered
- **WHEN** user presses Ctrl+S
- **THEN** Save File operation is triggered

#### Scenario: Editing shortcuts
- **WHEN** user presses Ctrl+Z
- **THEN** Undo operation is triggered
- **WHEN** user presses Ctrl+Y or Ctrl+Shift+Z
- **THEN** Redo operation is triggered
- **WHEN** user presses Delete
- **THEN** Remove highlighted voxel operation is triggered

#### Scenario: View shortcuts
- **WHEN** user presses F
- **THEN** camera frames all voxels in scene (fit to view)
- **WHEN** user presses G
- **THEN** grid visibility is toggled

#### Scenario: Tool shortcuts
- **WHEN** user presses P
- **THEN** Place tool is activated
- **WHEN** user presses E
- **THEN** Remove tool is activated

#### Scenario: Cursor control shortcuts
- **WHEN** user presses Tab
- **THEN** focus mode toggles between Near and Far
- **WHEN** paint mode is Single and user presses [ key
- **THEN** cursor size decreases by 1 (minimum 1)
- **WHEN** paint mode is Single and user presses ] key
- **THEN** cursor size increases by 1 (maximum 16)
- **WHEN** paint mode is Brush and user presses [ key
- **THEN** brush scale_depth decreases by 1 (minimum 0)
- **WHEN** paint mode is Brush and user presses ] key
- **THEN** brush scale_depth increases by 1 (maximum 4)
- **WHEN** user scrolls mouse wheel while holding Shift
- **THEN** cursor size or brush scale adjusts incrementally

#### Scenario: Mode toggle shortcuts
- **WHEN** user presses M key
- **THEN** paint mode toggles between Single and Brush
- **WHEN** user presses C key
- **THEN** camera mode toggles between LookAt and Free

#### Scenario: Copy brush shortcut
- **WHEN** user presses Ctrl+C with cursor positioned
- **THEN** voxels within cursor volume are copied
- **AND** copied content becomes temporary brush
- **AND** paint mode switches to Brush

### Requirement: Error Handling and User Feedback
The editor SHALL handle errors gracefully and provide clear feedback to users when operations fail.

#### Scenario: Invalid file format error
- **WHEN** user attempts to open a corrupted CSM file
- **THEN** editor displays error dialog with message "Invalid CSM format"
- **AND** editor remains in previous state (doesn't crash)

#### Scenario: File save error
- **WHEN** user attempts to save to a read-only directory
- **THEN** editor displays error dialog with message "Cannot write to file: [reason]"
- **AND** user can choose a different save location

#### Scenario: Material load error
- **WHEN** editor starts but `assets/materials.json` is missing
- **THEN** editor displays warning dialog
- **AND** editor falls back to default material set (basic colors)

### Requirement: Cross-Platform Compatibility
The editor SHALL compile and run on Windows, Linux, and macOS without platform-specific code where possible.

#### Scenario: Build on Linux with optimizations
- **WHEN** developer runs `cargo build --bin planet` on Ubuntu 22.04 or NixOS
- **THEN** compilation succeeds without errors
- **AND** mold linker is used automatically (if available via .cargo/config.toml)
- **AND** cranelift codegen is used for debug builds (if nightly toolchain available)
- **AND** resulting binary runs without missing dependencies
- **AND** Nix users can use flake.nix for system dependencies

#### Scenario: Build on Windows
- **WHEN** developer runs `cargo build --bin planet` on Windows 11
- **THEN** compilation succeeds without errors
- **AND** resulting binary runs without requiring Visual C++ runtime

#### Scenario: Build on macOS
- **WHEN** developer runs `cargo build --bin planet` on macOS 14 (Sonoma)
- **THEN** compilation succeeds without errors
- **AND** resulting .app bundle or binary runs natively on Apple Silicon and Intel

### Requirement: Integration with Existing Crates
The editor SHALL use the existing `cube` and `world` crates without modification, calling their public APIs directly.

#### Scenario: Use cube crate for mesh generation
- **WHEN** editor needs to render voxels
- **THEN** it calls `cube::mesh::generate_mesh()` or `world::WorldCube::generate_frame()`
- **AND** converts returned GeometryData to Bevy Mesh
- **AND** no cube crate code is duplicated or forked

#### Scenario: Use cube crate for raycasting
- **WHEN** editor performs raycasting for voxel picking
- **THEN** it calls `cube::raycast::raycast(origin, direction, cube)`
- **AND** uses returned hit information (position, face, distance)
- **AND** no raycast logic is reimplemented

#### Scenario: Use world crate for voxel editing
- **WHEN** editor places or removes a voxel
- **THEN** it calls `WorldCube::set_voxel_at_depth()` or `remove_voxel_at_depth()`
- **AND** respects multi-depth octree structure (macro/micro depths)

### Requirement: Asset Compatibility
The editor SHALL produce and consume assets that are compatible with the web application, using the same file formats and conventions.

#### Scenario: CSM format compatibility
- **WHEN** editor saves a scene as CSM
- **THEN** the web application can parse and load it via `loadCsm()`
- **AND** all material IDs are preserved correctly
- **AND** octree structure is identical

#### Scenario: Material ID consistency
- **WHEN** editor assigns material ID 5 to a voxel
- **AND** scene is loaded in web application
- **THEN** voxel renders with the same material (color, properties)
- **AND** material indices match `assets/materials.json`

### Requirement: Performance Optimization
The editor SHALL optimize rendering and editing operations to maintain interactive framerates even with complex scenes.

#### Scenario: Efficient mesh regeneration
- **WHEN** user edits a single voxel
- **THEN** only the affected octree node's mesh is regenerated (if chunked)
- **OR** full mesh regeneration completes within 16ms (60 FPS)
- **AND** editor remains responsive during regeneration

#### Scenario: Raycast performance
- **WHEN** user moves mouse rapidly over viewport
- **THEN** raycasting completes within 1ms per frame
- **AND** does not block rendering or input handling

#### Scenario: Large scene handling
- **WHEN** scene contains 100,000 voxels
- **THEN** editor maintains at least 30 FPS
- **AND** editing operations remain responsive (<100ms latency)

### Requirement: User Preferences
The editor SHALL save and load user preferences such as camera position, window size, and UI layout.

#### Scenario: Save window size and position
- **WHEN** user resizes or moves the editor window
- **AND** closes the editor
- **THEN** window size and position are saved to config file
- **WHEN** user reopens editor
- **THEN** window restores previous size and position

#### Scenario: Save UI panel visibility
- **WHEN** user closes the material palette panel
- **AND** closes the editor
- **THEN** palette visibility state is saved
- **WHEN** user reopens editor
- **THEN** palette remains closed

#### Scenario: Save recent files list
- **WHEN** user opens a CSM file
- **THEN** file path is added to recent files list (max 10)
- **AND** recent files appear in File menu
- **AND** list persists across sessions

### Requirement: Documentation and Help
The editor SHALL provide in-application help and documentation for users unfamiliar with the tool.

#### Scenario: Display keyboard shortcuts help
- **WHEN** user selects Help → Keyboard Shortcuts
- **THEN** a panel opens displaying all shortcuts grouped by category
- **AND** shortcuts are searchable via text filter

#### Scenario: Tooltips on hover
- **WHEN** user hovers over a toolbar button for 1 second
- **THEN** a tooltip appears showing button name and keyboard shortcut
- **AND** tooltip disappears when cursor moves away
