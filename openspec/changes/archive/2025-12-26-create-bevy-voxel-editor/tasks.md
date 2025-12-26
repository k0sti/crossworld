# Implementation Tasks: Bevy Voxel Editor

## Phase 0: Build Environment Setup

### 0.1 Linux build optimizations
- [x] 0.1.1 Create `.cargo/config.toml` in project root
- [x] 0.1.2 Configure mold linker for Linux:
  ```toml
  [target.x86_64-unknown-linux-gnu]
  linker = "clang"
  rustflags = ["-C", "link-arg=-fuse-ld=mold"]
  ```
- [x] 0.1.3 Configure cranelift for nightly debug builds:
  ```toml
  [unstable]
  codegen-backend = true
  ```
- [x] 0.1.4 Add profile configurations for fast debug builds
- [ ] 0.1.5 Test that mold is used (check build output for "mold" mentions)

### 0.2 Nix flake for Linux dependencies
- [x] 0.2.1 Create `flake.nix` in crates/editor
- [x] 0.2.2 Add Bevy dependencies following https://github.com/bevyengine/bevy/blob/latest/docs/linux_dependencies.md:
  - pkg-config
  - udev, alsa-lib, vulkan-loader
  - X11 libs: libxkbcommon, libX11, libXcursor, libXi, libXrandr
  - Wayland: wayland, wayland-protocols
- [x] 0.2.3 Add development tools: rustc, cargo, mold, clang, lld
- [ ] 0.2.4 Test with `nix develop` and verify `cargo build` works

### 0.3 Justfile integration
- [x] 0.3.1 Add `just planet` command to run editor: `cargo run --bin planet`
- [x] 0.3.2 Add `just planet-release` for optimized builds
- [x] 0.3.3 Update justfile documentation with new commands

## Phase 1: Project Setup and Scaffold

### 1.1 Create editor crate
- [x] 1.1.1 Create `crates/editor/` directory
- [x] 1.1.2 Create `Cargo.toml` with dependencies:
  - bevy = "0.17.3"
  - bevy_egui (latest compatible with 0.17.3)
  - cube = { path = "../cube" }
  - crossworld-world = { path = "../world" }
  - serde = { version = "1.0", features = ["derive"] }
  - serde_json = "1.0"
  - rfd = "0.15" (file dialogs)
- [x] 1.1.3 Configure binary name in Cargo.toml: `[[bin]] name = "planet"`
- [x] 1.1.4 Add editor to workspace `Cargo.toml`
- [x] 1.1.5 Create `src/main.rs` with basic Bevy app setup

### 1.2 Basic window and rendering
- [x] 1.2.1 Initialize Bevy 0.17.3 app with DefaultPlugins
- [x] 1.2.2 Set window title to "Crossworld Voxel Editor"
- [x] 1.2.3 Configure window size (1280x720 default)
- [x] 1.2.4 Add bevy_egui plugin (verify compatibility with Bevy 0.17.3)
- [x] 1.2.5 Create a test 3D scene with camera and light

### 1.3 Test integration with cube crate
- [x] 1.3.1 Create a simple Cube instance in startup system
- [x] 1.3.2 Generate mesh via `generate_face_mesh()`
- [x] 1.3.3 Convert mesh data to Bevy Mesh format
- [x] 1.3.4 Spawn entity with mesh and material
- [x] 1.3.5 Verify voxels render correctly in viewport

### 1.4 Verify build optimizations
- [x] 1.4.1 Test `just planet` command runs successfully
- [ ] 1.4.2 Verify mold linker is used on Linux (check build logs)
- [x] 1.4.3 Test incremental rebuild time (<1 second for check)
- [ ] 1.4.4 Document editor in README.md with build instructions
- [x] 1.4.5 Create `crates/editor/README.md` with usage instructions

## Phase 2: Camera Controls

### 2.1 Orbit camera system
- [x] 2.1.1 Create `src/camera.rs` module
- [x] 2.1.2 Define `OrbitCamera` component with target, distance, angles
- [x] 2.1.3 Implement camera controller system:
  - Right-click drag → rotate (azimuth/elevation) - **IMPROVED**: Increased rotation speed from 0.005 to 0.01 for more responsive rotation
  - Middle-click drag → pan - **DISABLED**: Panning disabled to keep cube fixed at origin
  - Scroll wheel → zoom
- [x] 2.1.4 Add camera distance limits (min: 1.0, max: 100.0)
- [x] 2.1.5 Add smooth interpolation for camera movement - **IMPROVED**: Increased smoothing from 0.1 to 0.5 for more responsive camera

### 2.2 Camera keyboard shortcuts
- [x] 2.2.1 Implement 'F' key to frame scene (fit all voxels)
- [x] 2.2.2 Add numpad shortcuts for orthographic views (1=front, 3=side, 7=top)
- [x] 2.2.3 Add camera reset to default position (Home key)

## Phase 3: Voxel Scene Management

### 3.1 VoxelScene resource
- [x] 3.1.1 Create `src/voxel_scene.rs` module
- [x] 3.1.2 Define `VoxelScene` resource wrapping WorldCube (with ThreadSafeWorldCube wrapper)
- [x] 3.1.3 Add `mesh_dirty` flag for tracking changes
- [x] 3.1.4 Initialize default scene in startup system (WorldCube macro_depth 3, micro_depth 5, border_depth 1)

### 3.2 Mesh synchronization system
- [x] 3.2.1 Create `src/mesh_sync.rs` module
- [x] 3.2.2 Implement system that checks `mesh_dirty` flag
- [x] 3.2.3 Call `generate_frame()` when dirty (via WorldCube.lock())
- [x] 3.2.4 Convert GeometryData (vertices, indices, normals, colors) to Bevy Mesh
- [x] 3.2.5 Spawn/despawn mesh entities as needed
- [x] 3.2.6 Reset `mesh_dirty` flag after update

### 3.3 Mesh conversion utilities
- [x] 3.3.1 Write helper function to convert GeometryData to Bevy Mesh
- [x] 3.3.2 Handle vertex colors (convert Vec<f32> RGB to Bevy RGBA VertexAttributeValues)
- [x] 3.3.3 Handle normals (convert flat Vec<f32> to Vec<[f32; 3]>)
- [x] 3.3.4 Use Bevy 0.17 API (PrimitiveTopology::TriangleList, Mesh::insert_attribute)

## Phase 4: Raycasting and Cursor System

### 4.1 Edit ray and raycast face
- [x] 4.1.1 Create `src/raycast.rs` module
- [x] 4.1.2 Define RaycastResult struct (hit_position, face_normal, distance, voxel_coord)
- [x] 4.1.3 Implement system that casts ray from camera through mouse cursor
- [x] 4.1.4 Convert window coordinates to world-space ray (Camera::viewport_to_world)
- [x] 4.1.5 Implement temporary plane raycast (TODO: integrate cube::raycast)
- [x] 4.1.6 Calculate face normal from raycast hit (axis-aligned)
- [x] 4.1.7 Store RaycastResult in EditorRaycast resource

### 4.2 Cube cursor implementation
- [x] 4.2.1 Create `src/cursor.rs` module
- [x] 4.2.2 Define CubeCursor struct (position, size, valid flag)
- [x] 4.2.3 Add cursor to EditorState resource
- [x] 4.2.4 Implement update_cursor system that positions cursor based on raycast
- [x] 4.2.5 Calculate cursor position using focus mode and face normal
- [x] 4.2.6 Support cursor sizes from 1x1x1 to 16x16x16

### 4.3 Focus mode system
- [x] 4.3.1 Define FocusMode enum (Near, Far) in EditorState
- [x] 4.3.2 Implement Near mode: cursor at hit position (for removal)
- [x] 4.3.3 Implement Far mode: cursor at hit position + face normal (for placement)
- [x] 4.3.4 Add Tab key binding to toggle focus mode
- [x] 4.3.5 Update cursor position when focus mode changes

### 4.4 Cursor visual feedback
- [x] 4.4.1 Use Bevy Gizmos to draw wireframe cube at cursor position
- [x] 4.4.2 Scale gizmo based on cursor size (1x1x1, 2x2x2, etc.)
- [x] 4.4.3 Color gizmo red for Near mode (removal), green for Far mode (placement)
- [x] 4.4.4 Hide gizmo when no raycast intersection (cursor.valid = false)
- [ ] 4.4.5 Add optional fill transparency to visualize cursor volume (deferred)

### 4.5 Cursor size controls
- [x] 4.5.1 Implement [ key binding to decrease cursor size (min 1)
- [x] 4.5.2 Implement ] key binding to increase cursor size (max 16)
- [x] 4.5.3 Implement Shift+scroll wheel for cursor size adjustment
- [ ] 4.5.4 Display cursor size in status bar (requires UI - Phase 7)
- [ ] 4.5.5 Add cursor size indicator in inspector panel (requires UI - Phase 7)

## Phase 5: Paint Modes and Brush System

### 5.1 Paint mode infrastructure
- [ ] 5.1.1 Create `src/paint_mode.rs` module
- [ ] 5.1.2 Define PaintMode enum (Single, Brush)
- [ ] 5.1.3 Add paint_mode to EditorState
- [ ] 5.1.4 Implement M key binding to toggle paint mode
- [ ] 5.1.5 Update UI to display current paint mode

### 5.2 Voxel brush system
- [ ] 5.2.1 Create `src/brush.rs` module
- [ ] 5.2.2 Define VoxelBrush struct (name, cube, scale_depth, preview_mesh)
- [ ] 5.2.3 Implement brush loading from .vox files
- [ ] 5.2.4 Scan `assets/avatars/` and `assets/models/` for .vox files at startup
- [ ] 5.2.5 Store loaded brushes in BrushLibrary resource
- [ ] 5.2.6 Generate preview meshes for each brush

### 5.3 Brush scale system
- [ ] 5.3.1 Implement scale_depth parameter (0-4, default 0)
- [ ] 5.3.2 Calculate scaled size: base_size * 2^scale_depth
- [ ] 5.3.3 Apply scale when stamping brush into scene
- [ ] 5.3.4 Update [ and ] key bindings to adjust scale_depth in Brush mode
- [ ] 5.3.5 Display scale multiplier in UI (e.g., "2x", "4x")

### 5.4 Copy brush from content (Ctrl+C)
- [ ] 5.4.1 Implement Ctrl+C key binding
- [ ] 5.4.2 Extract voxels within cursor volume at current position
- [ ] 5.4.3 Create temporary VoxelBrush from extracted voxels
- [ ] 5.4.4 Set as active brush and switch to Brush mode
- [ ] 5.4.5 Generate preview mesh for copied content

## Phase 6: Voxel Editing

### 6.1 EditorState resource
- [x] 6.1.1 Create `src/editing.rs` module
- [x] 6.1.2 Extended EditorState in cursor.rs with:
  - selected_material: u8 ✓
  - continuous_paint: bool ✓
  - last_paint_position: Option<IVec3> ✓
  - Note: Full EditorTool/PaintMode enums deferred to Phase 5

### 6.2 Single material placement system
- [x] 6.2.1 Detect left-click or left-click-hold on mouse button input
- [x] 6.2.2 Check cursor is positioned (valid flag)
- [x] 6.2.3 Get cursor position and size from EditorState
- [x] 6.2.4 For each voxel in cursor volume (size^3 iterations):
  - Call `set_voxel_at_depth(x, y, z, depth, material_id)`
- [x] 6.2.5 Mark mesh_dirty = true
- [ ] 6.2.6 Create PlaceVoxelsCommand (batch) and add to history (undo/redo deferred to Phase 17)
- [x] 6.2.7 Handle single voxel (size=1) vs multi-voxel placement

### 6.2.8 Material selection system
- [x] 6.2.8.1 Implement 0-9 key bindings to select materials 0-9
- [x] 6.2.8.2 Update selected_material in EditorState
- [x] 6.2.8.3 Log material selection for user feedback

### 6.3 Brush placement system
- [ ] 6.3.1 Detect left-click or left-click-hold when paint mode is Brush (Phase 5 required)
- [ ] 6.3.2 Check if brush is selected and cursor is positioned (Phase 5 required)
- [ ] 6.3.3 Get cursor position and active brush from EditorState (Phase 5 required)
- [ ] 6.3.4 Apply scale_depth to brush voxels (multiply coordinates by 2^scale_depth)
- [ ] 6.3.5 For each voxel in scaled brush:
  - Call `set_voxel_at_depth(cursor_pos + voxel_offset, depth, voxel_material)`
- [ ] 6.3.6 Mark mesh_dirty = true
- [ ] 6.3.7 Create PlaceBrushCommand and add to history

### 6.4 Continuous paint mode
- [x] 6.4.1 Track left mouse button state (pressed/released)
- [x] 6.4.2 Set continuous_paint = true when button pressed
- [x] 6.4.3 Set continuous_paint = false when button released
- [x] 6.4.4 During continuous_paint, check if cursor position changed
- [x] 6.4.5 If cursor moved to new coordinate, perform paint operation
- [x] 6.4.6 Store last_paint_position to prevent duplicate paints at same coord
- [x] 6.4.7 Reset last_paint_position when button released

### 6.5 Voxel removal system
- [x] 6.5.1 Detect Shift+left-click or Delete key
- [x] 6.5.2 Check if cursor is positioned (raycast valid)
- [x] 6.5.3 Handle removal (removes cursor volume, sets voxels to air)
- [x] 6.5.4 Mark mesh_dirty = true
- [ ] 6.5.5 Create RemoveVoxelsCommand (batch) and add to history (undo/redo deferred to Phase 17)

## Phase 7: Camera System

### 7.1 Camera mode infrastructure
- [ ] 7.1.1 Define CameraMode enum (LookAt, Free)
- [ ] 7.1.2 Add camera_mode to EditorState
- [ ] 7.1.3 Implement C key binding to toggle camera mode
- [ ] 7.1.4 Update status bar to display camera mode

### 7.2 LookAt camera mode
- [ ] 7.2.1 Calculate scene center from voxel bounding box
- [ ] 7.2.2 Update scene center when voxels are added/removed
- [ ] 7.2.3 Implement orbit rotation around scene center
- [ ] 7.2.4 Maintain camera distance from center during rotation
- [ ] 7.2.5 Ensure camera always faces center point

### 7.3 Free camera mode
- [ ] 7.3.1 Implement in-place camera rotation (no orbit)
- [ ] 7.3.2 Allow free camera movement via middle-click pan
- [ ] 7.3.3 Camera position stays fixed during rotation
- [ ] 7.3.4 Camera direction changes based on mouse drag

### 7.4 Right-click hold and drag rotation
- [ ] 7.4.1 Track right mouse button state (pressed/held/released)
- [ ] 7.4.2 Only rotate while right button is held down
- [ ] 7.4.3 Apply rotation continuously during drag
- [ ] 7.4.4 Stop rotation when button released
- [ ] 7.4.5 Adjust rotation behavior based on camera mode

### 7.5 Frame scene (F key)
- [ ] 7.5.1 Calculate bounding box of all visible voxels
- [ ] 7.5.2 Calculate optimal camera distance to fit all content
- [ ] 7.5.3 Animate camera to framed position
- [ ] 7.5.4 Update scene center in LookAt mode

## Phase 8: Material and Brush UI

### 8.1 Material loading
- [ ] 8.1.1 Create `src/materials.rs` module
- [ ] 8.1.2 Define Material struct (id, name, color, transparent)
- [ ] 8.1.3 Load materials from `assets/materials.json` at startup
- [ ] 8.1.4 Store in MaterialPalette resource
- [ ] 8.1.5 Handle missing materials.json (fallback to defaults)

### 8.2 Material palette UI
- [ ] 8.2.1 Create `src/ui/palette.rs` module
- [ ] 8.2.2 Render egui panel with scrollable grid layout
- [ ] 8.2.3 Display all materials (not just first 9) with color swatches
- [ ] 8.2.4 Show material ID and name on each swatch
- [ ] 8.2.5 Show material name on hover (tooltip)
- [ ] 8.2.6 Highlight selected material with border
- [ ] 8.2.7 Handle click to select material (only in Single mode)
- [ ] 8.2.8 Disable/gray out palette when in Brush mode
- [ ] 8.2.9 Update EditorState.selected_material on selection

### 8.3 Brush selector UI
- [ ] 8.3.1 Create `src/ui/brush_selector.rs` module
- [ ] 8.3.2 Render egui panel with scrollable grid of brush thumbnails
- [ ] 8.3.3 Generate thumbnail images from brush preview meshes
- [ ] 8.3.4 Display brush name below each thumbnail
- [ ] 8.3.5 Show file path and dimensions in tooltip on hover
- [ ] 8.3.6 Highlight selected brush with border
- [ ] 8.3.7 Handle click to select brush (auto-switch to Brush mode)
- [ ] 8.3.8 Add scale_depth slider for each brush (0-4)
- [ ] 8.3.9 Update brush preview when scale_depth changes
- [ ] 8.3.10 Display scale multiplier (e.g., "1x", "2x", "4x", "8x", "16x")

### 8.4 Keyboard shortcuts for materials
- [ ] 8.4.1 Implement 1-9 key bindings to select materials 0-8
- [ ] 8.4.2 Switch to Single mode when material shortcut pressed
- [ ] 8.4.3 Display key number in palette UI for first 9 materials

## Phase 10: File I/O

### 10.1 File operations infrastructure
- [ ] 7.1.1 Create `src/ui/mod.rs` with UiPlugin
- [ ] 7.1.2 Register UI systems in Update schedule
- [ ] 7.1.3 Create egui context and configure style

### 7.2 Toolbar
- [ ] 7.2.1 Create `src/ui/toolbar.rs` module
- [ ] 7.2.2 Render top toolbar with egui::TopBottomPanel
- [ ] 7.2.3 Add File menu (New, Open, Save, Save As, Quit)
- [ ] 7.2.4 Add Edit menu (Undo, Redo)
- [ ] 7.2.5 Add View menu (Grid, Wireframe, Inspector, Palette)
- [ ] 7.2.6 Add Help menu (About, Keyboard Shortcuts)

### 7.3 Tool buttons
- [ ] 7.3.1 Add tool selection buttons to toolbar (Place, Remove, Paint, Select)
- [ ] 7.3.2 Highlight active tool button
- [ ] 7.3.3 Handle button clicks to change EditorState.current_tool
- [ ] 7.3.4 Display tool icons (optional, can use text labels initially)

### 7.4 Status bar
- [ ] 7.4.1 Create `src/ui/status.rs` module
- [ ] 7.4.2 Render bottom status bar with egui::TopBottomPanel
- [ ] 7.4.3 Display cursor coordinates (x, y, z) from EditorState
- [ ] 7.4.4 Display cursor size (e.g., "2x2x2")
- [ ] 7.4.5 Display focus mode ("Near" or "Far") with Tab hint
- [ ] 7.4.6 Display raycast face normal (e.g., "+X", "-Y")
- [ ] 7.4.7 Display FPS counter (from Bevy diagnostics)
- [ ] 7.4.8 Display current tool name and shortcut hint
- [ ] 7.4.9 Display selected material name

### 7.5 Inspector panel
- [ ] 7.5.1 Create `src/ui/inspector.rs` module
- [ ] 7.5.2 Render side panel with scene statistics:
  - Total voxel count
  - Mesh triangle count
  - Current depth level
  - Camera position and target
- [ ] 7.5.3 Add cursor information section:
  - Cursor position (x, y, z)
  - Cursor size (1-16)
  - Focus mode (Near/Far)
- [ ] 7.5.4 Add collapsible sections (egui::CollapsingHeader)
- [ ] 7.5.5 Add toggle for panel visibility

### 10.1 File operations infrastructure
- [ ] 10.1.1 Create `src/file_io.rs` module
- [ ] 10.1.2 Use `rfd` crate for native file dialogs
- [ ] 10.1.3 Define file filter for CSM and .vox files

### 10.2 New scene
- [ ] 8.2.1 Implement new_scene() function
- [ ] 8.2.2 Show confirmation dialog if unsaved changes exist
- [ ] 8.2.3 Clear current VoxelScene and reinitialize default WorldCube
- [ ] 8.2.4 Reset undo/redo history
- [ ] 8.2.5 Update window title to "Untitled - Crossworld Voxel Editor"

### 8.3 Open CSM file
- [ ] 8.3.1 Show file open dialog filtered to .csm files
- [ ] 8.3.2 Read file contents to string
- [ ] 8.3.3 Parse CSM via cube crate API (assuming `load_csm` exists)
- [ ] 8.3.4 Load parsed Cube into VoxelScene
- [ ] 8.3.5 Mark mesh_dirty and update viewport
- [ ] 8.3.6 Update window title with filename
- [ ] 8.3.7 Handle parse errors with user-friendly dialog

### 8.4 Save CSM file
- [ ] 8.4.1 Implement save_scene() function
- [ ] 8.4.2 If no filename set, show save-as dialog
- [ ] 8.4.3 Export scene via `export_to_csm()` from world crate
- [ ] 8.4.4 Write CSM text to file path
- [ ] 8.4.5 Handle write errors with user-friendly dialog
- [ ] 8.4.6 Mark scene as saved (clear dirty flag)

### 8.5 Import .vox file
- [ ] 8.5.1 Show file open dialog filtered to .vox files
- [ ] 8.5.2 Read file bytes
- [ ] 8.5.3 Parse .vox via `dot_vox` crate (already in world dependencies)
- [ ] 8.5.4 Convert voxel data to Cube format
- [ ] 8.5.5 Load into VoxelScene and update mesh
- [ ] 8.5.6 Handle parse errors with user-friendly dialog

### 8.6 Export .vox file
- [ ] 8.6.1 Show save-as dialog filtered to .vox files
- [ ] 8.6.2 Convert Cube to dot_vox format
- [ ] 8.6.3 Write .vox bytes to file
- [ ] 8.6.4 Handle conversion/write errors

## Phase 17: Undo/Redo System

### 9.1 Command pattern infrastructure
- [ ] 9.1.1 Create `src/history.rs` module
- [ ] 9.1.2 Define EditorCommand trait with execute() and undo() methods
- [ ] 9.1.3 Define CommandHistory resource with Vec<Box<dyn EditorCommand>>
- [ ] 9.1.4 Add cursor field for tracking position in history

### 11.2 Voxel commands
- [ ] 9.2.1 Implement PlaceVoxelsCommand struct (batch support)
  - Store Vec<(position, depth, new_material, previous_material)>
  - Implement execute() to call set_voxel_at_depth() for all voxels
  - Implement undo() to restore all previous materials or remove voxels
  - Handle single voxel (cursor size 1) and multi-voxel (cursor size >1)
- [ ] 9.2.2 Implement RemoveVoxelsCommand struct (batch support)
  - Store Vec<(position, depth, removed_material)>
  - Implement execute() to call remove_voxel_at_depth() for all voxels
  - Implement undo() to restore all removed voxels
  - Handle cursor volume efficiently

### 11.3 Undo/redo systems
- [ ] 9.3.1 Implement undo_system triggered by Ctrl+Z
- [ ] 9.3.2 Call undo() on command at cursor position
- [ ] 9.3.3 Decrement cursor
- [ ] 9.3.4 Mark mesh_dirty
- [ ] 9.3.5 Implement redo_system triggered by Ctrl+Y
- [ ] 9.3.6 Increment cursor and call execute() on command

### 11.4 History management
- [ ] 9.4.1 Truncate history when new command is added (discard redo branch)
- [ ] 9.4.2 Implement history depth limit (e.g., 500 commands)
- [ ] 9.4.3 Add history display in Edit menu (optional: list of command names)

## Phase 12: Keyboard Shortcuts

### 12.1 Input handling system
- [ ] 10.1.1 Create `src/input.rs` module
- [ ] 10.1.2 Define keyboard_shortcut_system
- [ ] 10.1.3 Handle Ctrl+N for new scene
- [ ] 10.1.4 Handle Ctrl+O for open file
- [ ] 10.1.5 Handle Ctrl+S for save file
- [ ] 10.1.6 Handle Ctrl+Shift+S for save-as
- [ ] 10.1.7 Handle Ctrl+Z for undo
- [ ] 10.1.8 Handle Ctrl+Y or Ctrl+Shift+Z for redo
- [ ] 10.1.9 Handle Delete for remove voxel
- [ ] 10.1.10 Handle F for frame scene
- [ ] 10.1.11 Handle G for toggle grid
- [ ] 10.1.12 Handle P for place tool
- [ ] 10.1.13 Handle E for erase/remove tool
- [ ] 10.1.14 Handle Tab for toggle focus mode (Near/Far)
- [ ] 10.1.15 Handle [ for decrease cursor size
- [ ] 10.1.16 Handle ] for increase cursor size
- [ ] 10.1.17 Handle Shift+scroll wheel for cursor size adjustment

### 12.2 Keyboard shortcuts help
- [ ] 10.2.1 Create `src/ui/shortcuts.rs` module
- [ ] 10.2.2 Define list of shortcuts with categories
- [ ] 10.2.3 Render egui window when Help → Keyboard Shortcuts clicked
- [ ] 10.2.4 Display shortcuts in table format (Action | Shortcut)
- [ ] 10.2.5 Add search/filter capability

## Phase 13: User Preferences

### 13.1 Config file handling
- [ ] 11.1.1 Define EditorConfig struct with serde Serialize/Deserialize
- [ ] 11.1.2 Include fields: window_size, window_position, panel_visibility, recent_files
- [ ] 11.1.3 Implement save_config() to write JSON to platform config dir
  - Linux: `~/.config/crossworld-editor/config.json`
  - Windows: `%APPDATA%/crossworld-editor/config.json`
  - macOS: `~/Library/Application Support/crossworld-editor/config.json`
- [ ] 11.1.4 Implement load_config() to read JSON at startup

### 13.2 Preference persistence
- [ ] 11.2.1 Save config when window closes (via AppExit event)
- [ ] 11.2.2 Save config when panel visibility changes
- [ ] 11.2.3 Add recent files to config when file is opened/saved (max 10)
- [ ] 11.2.4 Apply loaded config at startup (restore window, panels, etc.)

### 13.3 Recent files menu
- [ ] 11.3.1 Add "Recent Files" submenu to File menu
- [ ] 11.3.2 Display list of recent file paths
- [ ] 11.3.3 Handle click to open recent file
- [ ] 11.3.4 Add "Clear Recent Files" option

## Phase 14: Error Handling and Polish

### 14.1 Error dialogs
- [ ] 12.1.1 Create utility function for showing error dialogs (via egui modal)
- [ ] 12.1.2 Display user-friendly error messages for:
  - Invalid CSM format
  - File read/write errors
  - Missing materials.json
  - Out-of-memory errors
- [ ] 12.1.3 Log detailed errors to console for debugging

### 14.2 Tooltips
- [ ] 12.2.1 Add tooltips to all toolbar buttons (show name + shortcut)
- [ ] 12.2.2 Add tooltips to material palette swatches (show material name)
- [ ] 12.2.3 Add tooltips to inspector panel fields

### 14.3 Visual polish
- [ ] 12.3.1 Add grid rendering in viewport (XZ plane, 10 unit spacing)
- [ ] 12.3.2 Add axis gizmo in corner of viewport (RGB for XYZ)
- [ ] 12.3.3 Configure lighting for better voxel visibility (ambient + directional)
- [ ] 12.3.4 Add anti-aliasing (MSAA or FXAA via Bevy config)

### 14.4 Performance profiling
- [ ] 12.4.1 Test with large scenes (100k+ voxels)
- [ ] 12.4.2 Profile mesh generation time (should be <16ms for 60 FPS)
- [ ] 12.4.3 Profile raycast time (should be <1ms)
- [ ] 12.4.4 Add performance metrics to inspector panel (frame time graph)

## Phase 15: Testing and Validation

### 15.1 Manual testing checklist
- [ ] 13.1.1 Test on Linux (Ubuntu 22.04 and NixOS via flake)
- [ ] 13.1.2 Test on Windows (Windows 11)
- [ ] 13.1.3 Test on macOS (Intel and Apple Silicon if possible)
- [ ] 13.1.4 Verify mold linker speedup on Linux (compare with GNU ld)
- [ ] 13.1.5 Verify cranelift debug builds on nightly (compare times)
- [ ] 13.1.6 Verify all keyboard shortcuts work
- [ ] 13.1.7 Verify all menu items work
- [ ] 13.1.8 Test undo/redo with 100+ operations
- [ ] 13.1.9 Test file save/load round-trip (CSM and .vox)
- [ ] 13.1.10 Test with large scenes (measure performance)

### 15.2 Integration testing
- [ ] 13.2.1 Export CSM from editor and load in web app (verify compatibility)
- [ ] 13.2.2 Import .vox created in MagicaVoxel (verify parsing)
- [ ] 13.2.3 Verify material IDs match between editor and web app

### 15.3 Bug fixes
- [ ] 13.3.1 Fix any crashes or panics discovered during testing
- [ ] 13.3.2 Fix any UI layout issues (overlapping panels, etc.)
- [ ] 13.3.3 Fix any input handling issues (stuck keys, etc.)

## Phase 16: Documentation

### 16.1 User documentation
- [ ] 14.1.1 Create `doc/tools/editor.md` with:
  - Installation instructions
  - Quick start guide
  - Interface overview (viewport, panels, toolbar)
  - Editing workflow (place, remove, paint)
  - Keyboard shortcuts reference
  - File format support (CSM, .vox)
- [ ] 14.1.2 Add screenshots of editor UI
- [ ] 14.1.3 Add troubleshooting section

### 16.2 Developer documentation
- [ ] 14.2.1 Update `crates/editor/README.md` with:
  - Architecture overview (Bevy plugins, systems, resources)
  - How to build and run
  - How to add new tools
  - How to add new file formats
- [ ] 14.2.2 Add inline code documentation (rustdoc comments)
- [ ] 14.2.3 Update main `CLAUDE.md` to reference editor

### 16.3 Update project docs
- [ ] 14.3.1 Add editor to `doc/README.md` navigation
- [ ] 14.3.2 Update `justfile` documentation section
- [ ] 14.3.3 Update `Cargo.toml` workspace documentation

## Phase 17: Deployment and Release

### 17.1 Build configuration
- [ ] 15.1.1 Configure release profile in Cargo.toml (opt-level = 3, lto = true)
- [ ] 15.1.2 Test release builds on all platforms
- [ ] 15.1.3 Add strip = true to reduce binary size

### 17.2 Binary distribution
- [ ] 15.2.1 Create GitHub release with binaries for Linux, Windows, macOS
- [ ] 15.2.2 Include assets folder (materials.json) in distribution
- [ ] 15.2.3 Add installation instructions to release notes

### 17.3 CI/CD
- [ ] 15.3.1 Add editor build step to CI pipeline (if exists)
- [ ] 15.3.2 Run `cargo check --bin planet` in pre-commit hooks
- [ ] 15.3.3 Add editor to `just check` command
- [ ] 15.3.4 Test flake.nix in CI (if using NixOS runners)
