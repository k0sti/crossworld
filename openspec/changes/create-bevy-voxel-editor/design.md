# Design: Bevy Voxel Editor

## Context

The Crossworld project has mature voxel infrastructure:
- `cube` crate: Octree data structure, mesh generation, CSM parsing, raycasting
- `world` crate: Multi-depth world generation, procedural terrain, voxel editing API
- Web application with Three.js renderer and TypeScript editor components

The native renderer (`crates/renderer`) uses low-level OpenGL (glow/glutin) with egui, which works for testing raycasting but lacks the ergonomics and extensibility for a full editor.

**Stakeholders:**
- Artists/content creators building voxel models and scenes
- Developers testing voxel features without web overhead
- Project maintainers needing a reference implementation of native rendering

**Constraints:**
- Must reuse existing `cube` and `world` crates (no forking/duplication)
- Desktop-only (Windows, Linux, macOS via Bevy's native targets)
- No WASM target (unlike web editor)
- Must produce assets compatible with web application (CSM format, .vox files)

## Goals / Non-Goals

**Goals:**
1. **Full voxel editing workflow**: Place, remove, paint voxels with intuitive controls
2. **CSM format support**: Load/save CSM text files for version control and web integration
3. **MagicaVoxel interop**: Import/export `.vox` files for compatibility
4. **Responsive 3D viewport**: Orbit camera, raycasting, visual feedback
5. **Material system**: Palette-based colors matching Crossworld's material specification
6. **Undo/redo**: Standard editing history with keyboard shortcuts
7. **Performance**: Handle large scenes (128x128x128 voxels) at 60 FPS

**Non-Goals:**
1. Multiplayer editing (out of scope)
2. Animation/scripting (separate tool concern)
3. Terrain generation UI (world crate handles this, editor just edits)
4. Asset pipeline integration (no automatic exports to web app)
5. Plugin system (keep simple, monolithic for v1)

## Architecture Overview

### Technology Stack

**Core Engine:** Bevy 0.17.3
- ECS architecture for clean separation of concerns
- Built-in renderer (wgpu-based, cross-platform)
- Transform/hierarchy system for scene management
- Input handling (keyboard, mouse, gamepad)
- Asset loading system
- Latest stable release with improved performance and API stability

**UI Framework:** `bevy_egui` (compatible with Bevy 0.17)
- Immediate-mode UI for toolbars, inspector panels
- Native look-and-feel, flexible layouts
- Minimal boilerplate

**Build Optimizations (Linux):**
- **mold linker:** Fast incremental linking (10x faster than GNU ld)
- **cranelift:** Fast debug builds via Rust nightly's `-Zcodegen-backend=cranelift`
- Configured via `.cargo/config.toml` for seamless development experience

**Development Environment:**
- Nix flake for reproducible Bevy dependencies on Linux
- Handles system libraries: libudev, alsa, X11/Wayland, vulkan

**Voxel Integration:**
- Use `cube::Cube` directly (via `rlib` crate type)
- Call `cube::mesh::generate_mesh()` for rendering
- Call `cube::raycast::raycast()` for voxel picking
- Use `world::WorldCube` for multi-depth editing

### Application Structure

```
crates/editor/
├── Cargo.toml               # Binary name: [[bin]] name = "planet"
├── src/
│   ├── main.rs              # Bevy app setup, plugin registration
│   ├── camera.rs            # Orbit camera controller
│   ├── voxel_scene.rs       # Bevy resources wrapping Cube/WorldCube
│   ├── input.rs             # Mouse/keyboard input handling
│   ├── raycast.rs           # Raycasting system (voxel picking)
│   ├── editing.rs           # Voxel add/remove/paint logic
│   ├── mesh_sync.rs         # Sync Cube → Bevy Mesh
│   ├── ui/
│   │   ├── mod.rs           # UI plugin
│   │   ├── toolbar.rs       # Top toolbar (file, edit, view)
│   │   ├── palette.rs       # Material palette panel
│   │   ├── inspector.rs     # Scene inspector (layer list)
│   │   └── status.rs        # Bottom status bar
│   ├── history.rs           # Undo/redo command pattern
│   └── file_io.rs           # CSM and .vox load/save
└── assets/
    ├── materials.json       # Material definitions (copy from main assets/)
    └── shaders/             # Custom voxel shaders (if needed)
```

## Key Decisions

### Decision 1: Use Bevy over raw OpenGL

**Options considered:**
1. **Extend existing renderer crate** (glow/glutin/egui)
   - Pros: Less new code, already integrated with cube crate
   - Cons: Low-level manual rendering, no ECS, harder to extend
2. **Bevy game engine**
   - Pros: Modern architecture, ECS, built-in renderer, active ecosystem
   - Cons: Learning curve, larger dependency tree, opinionated structure
3. **Other Rust game engines** (e.g., macroquad, ggez)
   - Pros: Simpler than Bevy
   - Cons: Less mature, fewer features, smaller community

**Decision:** Use Bevy
**Rationale:**
- ECS is ideal for editor tools (separation of state, logic, rendering)
- wgpu-based renderer is modern and cross-platform (no OpenGL version pain)
- `bevy_egui` provides production-quality UI
- Active community, frequent releases, good docs
- Positions project for future enhancements (physics preview, animation)

### Decision 2: Reuse cube/world crates unchanged

**Alternatives:**
1. **Fork/duplicate cube logic into editor**
   - Cons: Code duplication, drift over time, harder to maintain
2. **Extract shared code into new crate**
   - Cons: Premature abstraction, cube/world already work well
3. **Use cube/world as-is**
   - Pros: Single source of truth, bug fixes benefit both web and native
   - Cons: Must work within existing API (but API is already good)

**Decision:** Reuse unchanged
**Rationale:**
- `cube` crate already has `crate-type = ["cdylib", "rlib"]` - works for WASM and native
- Public API (`Cube::new()`, `generate_mesh()`, `raycast()`) is sufficient
- `world` crate similarly has all needed operations
- Keeps test coverage unified

### Decision 3: Use `bevy_egui` for UI (not native Bevy UI)

**Options:**
1. **Bevy's built-in UI** (retained-mode, declarative)
   - Pros: No extra dependency
   - Cons: Verbose for complex UIs, less mature than egui
2. **bevy_egui** (immediate-mode)
   - Pros: Ergonomic, feature-rich (file dialogs, color pickers, docking)
   - Cons: Extra dependency, different paradigm from Bevy UI

**Decision:** Use `bevy_egui`
**Rationale:**
- Editor UIs are inherently imperative (react to user input immediately)
- egui is proven in Rust gamedev (used by many tools)
- Rich widget library (collapsing headers, drag values, combo boxes)

### Decision 4: Command pattern for undo/redo

**Implementation:**
```rust
trait EditorCommand {
    fn execute(&self, world: &mut WorldCube);
    fn undo(&self, world: &mut WorldCube);
}

struct PlaceVoxelCommand {
    position: IVec3,
    depth: u32,
    color_index: i32,
    previous_state: Option<i32>, // None if was empty
}

struct History {
    commands: Vec<Box<dyn EditorCommand>>,
    cursor: usize,
}
```

**Rationale:**
- Standard pattern for editors
- Easy to serialize for session recovery
- Supports macro-commands (e.g., paint stroke = multiple voxel placements)

## System Design

### Bevy Plugin Architecture

```rust
fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(EguiPlugin)
        .add_plugins(EditorPlugin)
        .run();
}

struct EditorPlugin;
impl Plugin for EditorPlugin {
    fn build(&self, app: &mut App) {
        app
            .init_resource::<VoxelScene>()
            .init_resource::<EditorState>()
            .init_resource::<CommandHistory>()
            .add_systems(Startup, setup_scene)
            .add_systems(Update, (
                camera_controller_system,
                raycast_system,
                voxel_editing_system,
                mesh_sync_system,
                ui_system,
            ));
    }
}
```

### Core Resources

```rust
#[derive(Resource)]
struct VoxelScene {
    world: WorldCube, // From world crate
    // or
    cube: Cube,       // From cube crate (for single models)
    mesh_dirty: bool,
}

#[derive(Resource)]
struct EditorState {
    tool: EditorTool, // Place, Remove, Paint, Select
    paint_mode: PaintMode, // Single or Brush
    selected_material: u8,
    selected_brush: Option<VoxelBrush>,
    cursor: CubeCursor,
    raycast_result: Option<RaycastResult>,
    focus_mode: FocusMode, // Near or Far
    camera_mode: CameraMode, // LookAt or Free
    continuous_paint: bool, // True when left mouse held down
    last_paint_position: Option<IVec3>, // For continuous paint deduplication
}

#[derive(Clone, Copy)]
struct CubeCursor {
    position: IVec3,  // World position of cursor
    size: u32,        // Size of edit cursor (1x1x1, 2x2x2, etc.)
}

#[derive(Clone, Copy)]
struct RaycastResult {
    hit_position: IVec3,  // Voxel that was hit
    face_normal: IVec3,   // Face that was hit (unit vector)
    distance: f32,        // Distance from camera
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum FocusMode {
    Near,  // Cursor placed on near side of edit face (towards camera)
    Far,   // Cursor placed on far side of edit face (away from camera)
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum PaintMode {
    Single,  // Paint with single material
    Brush,   // Paint with voxel brush (loaded model)
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum CameraMode {
    LookAt,  // Camera always looks at center of model/scene
    Free,    // Camera can move freely without target constraint
}

#[derive(Clone)]
struct VoxelBrush {
    name: String,
    cube: Cube,          // Loaded from .vox file
    scale_depth: u32,    // Scale factor: 2^scale_depth
    preview_mesh: Handle<Mesh>, // Cached preview mesh
}

#[derive(Resource)]
struct CommandHistory {
    commands: Vec<Box<dyn EditorCommand>>,
    cursor: usize,
}
```

### Key Systems

#### 1. Raycast System
```rust
fn raycast_system(
    camera_query: Query<(&Camera, &GlobalTransform)>,
    windows: Query<&Window>,
    mut editor_state: ResMut<EditorState>,
    voxel_scene: Res<VoxelScene>,
) {
    // Get mouse position in viewport
    // Cast ray from camera through mouse
    // Call cube::raycast::raycast()
    // Store hit result in EditorState.raycast_result
    // Result includes hit_position, face_normal, distance
}
```

#### 1b. Cursor Update System
```rust
fn cursor_update_system(
    editor_state: Res<EditorState>,
    mut gizmos: Gizmos,
) {
    if let Some(raycast) = &editor_state.raycast_result {
        // Calculate cursor position based on focus mode and face normal
        let cursor_pos = match editor_state.focus_mode {
            FocusMode::Near => {
                // Place cursor on near side (hit position itself)
                raycast.hit_position
            }
            FocusMode::Far => {
                // Place cursor on far side (hit position + face normal)
                raycast.hit_position + raycast.face_normal
            }
        };

        // Update editor cursor
        let cursor = CubeCursor {
            position: cursor_pos,
            size: editor_state.cursor.size,
        };

        // Draw cursor gizmo (wireframe cube)
        draw_cursor_gizmo(&mut gizmos, &cursor);
    }
}
```

#### 2. Voxel Editing System
```rust
fn voxel_editing_system(
    input: Res<ButtonInput<MouseButton>>,
    keyboard: Res<ButtonInput<KeyCode>>,
    editor_state: Res<EditorState>,
    mut voxel_scene: ResMut<VoxelScene>,
    mut history: ResMut<CommandHistory>,
) {
    // If left-click + raycast hit:
    //   Create PlaceVoxelCommand
    //   Execute command
    //   Push to history
    //   Mark mesh_dirty = true
}
```

#### 3. Mesh Sync System
```rust
fn mesh_sync_system(
    mut voxel_scene: ResMut<VoxelScene>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    voxel_entity_query: Query<Entity, With<VoxelMeshMarker>>,
) {
    if voxel_scene.mesh_dirty {
        // Call voxel_scene.world.generate_frame()
        // Convert GeometryData to Bevy Mesh
        // Update mesh asset
        voxel_scene.mesh_dirty = false;
    }
}
```

### File I/O Integration

```rust
fn load_csm(path: &Path) -> Result<Cube, CubeError> {
    let csm_text = std::fs::read_to_string(path)?;
    cube::csm::load_csm(&csm_text)
}

fn save_csm(cube: &Cube, path: &Path) -> Result<(), std::io::Error> {
    let csm_text = cube.to_csm(); // Assuming we add this method
    std::fs::write(path, csm_text)
}

fn load_vox(path: &Path) -> Result<Cube, CubeError> {
    let bytes = std::fs::read(path)?;
    cube::vox::load_vox(&bytes)
}
```

## Material System Integration

The editor must use the same material IDs as the web application:
- Load `assets/materials.json` at startup
- Display material names in palette UI
- Store material index (u8) in voxel data
- Render with appropriate colors (read from materials.json)

**Material JSON structure:**
```json
{
  "materials": [
    { "id": 0, "name": "Air", "color": [0, 0, 0], "transparent": true },
    { "id": 1, "name": "Grass", "color": [34, 139, 34], "transparent": false },
    ...
  ]
}
```

## Performance Considerations

### Mesh Generation Optimization
- Only regenerate mesh when `mesh_dirty` flag is true
- Use `generate_frame()` from world crate (already optimized)
- Consider chunking for very large worlds (>256^3 voxels)

### Raycasting Optimization
- Raycast only when mouse moves or camera changes
- Cache raycast results for current frame
- Use cube crate's optimized octree traversal

### Rendering
- Use instancing if repeating patterns detected (future optimization)
- Leverage Bevy's frustum culling automatically
- Consider LOD for distant voxel chunks (future)

## Camera Controls

**Camera Controls:**
- **Right-click + hold + drag:** Rotate camera (LookAt mode: orbit around center, Free mode: rotate view)
- **Middle-click + drag:** Pan camera
- **Scroll wheel:** Zoom in/out
- **F:** Frame all content (fit to view)
- **C:** Toggle camera mode (LookAt ↔ Free)

**Painting Controls:**
- **Left-click:** Place voxel/brush at cursor position (single click)
- **Left-click + hold:** Continuous paint mode (paint as cursor moves to new coordinates)
- **Shift + left-click:** Remove voxel/brush volume at cursor

**Cursor Controls:**
- **Mouse movement:** Update edit ray and raycast face
- **Tab:** Toggle focus mode (Near ↔ Far)
- **[ / ]:** Decrease / Increase cursor size or brush scale depth
- **Scroll wheel (with Shift):** Adjust cursor size / brush scale

**Keyboard shortcuts:**
- **Ctrl+Z:** Undo
- **Ctrl+Y:** Redo
- **Ctrl+O:** Open file
- **Ctrl+S:** Save file
- **Ctrl+C:** Copy selection/brush from cursor position
- **Delete:** Remove voxel at cursor
- **1-9:** Quick-select materials (paint mode: Single)
- **M:** Toggle paint mode (Single ↔ Brush)
- **C:** Toggle camera mode (LookAt ↔ Free)
- **Space:** Toggle tool mode
- **Tab:** Toggle focus mode (Near/Far)

## Risks / Trade-offs

### Risk: Bevy API instability
- **Impact**: Bevy has frequent breaking changes between minor versions
- **Mitigation**: Pin to specific Bevy version (e.g., 0.15.x), plan for migration when upgrading
- **Fallback**: If Bevy proves too unstable, renderer crate architecture can be salvaged

### Risk: Performance with large worlds
- **Impact**: 256x256x256 voxel worlds may exceed memory/rendering budget
- **Mitigation**: Start with smaller default sizes (128^3), add chunking if needed
- **Monitoring**: Profile with cargo flamegraph, add metrics overlay

### Risk: Undo/redo complexity
- **Impact**: Complex edits (e.g., flood fill) hard to undo efficiently
- **Mitigation**: Start with simple commands (single voxel), expand later
- **Fallback**: Disable undo for complex operations in v1

### Trade-off: Immediate-mode UI vs. Bevy UI
- **Chosen**: bevy_egui (immediate-mode)
- **Cost**: Different mental model than rest of Bevy (ECS)
- **Benefit**: Faster development, richer widgets
- **Future**: Can migrate to Bevy UI if it matures (low priority)

## Migration Plan

### Phase 0: Build Environment Setup
- Create `.cargo/config.toml` with mold linker and cranelift configuration
- Create `flake.nix` for Nix users with Bevy system dependencies
- Add `just planet` command to run the editor

### Phase 1: Scaffold (Week 1)
- Create `crates/editor/` with Cargo.toml (Bevy 0.17.3)
- Configure binary name as `planet` in Cargo.toml
- Set up Bevy app with basic scene
- Render a test voxel mesh from cube crate
- Integrate bevy_egui with placeholder UI

### Phase 2: Core Editing (Week 2-3)
- Implement orbit camera controls
- Raycast system with visual feedback
- Place/remove voxels with mouse clicks
- Mesh sync system

### Phase 3: UI & UX (Week 4)
- Material palette panel
- File menu (open/save)
- Inspector panel (scene info)
- Status bar (coordinates, FPS)

### Phase 4: History & Polish (Week 5)
- Undo/redo system
- Keyboard shortcuts
- Save/load preferences
- Error handling and user feedback

### Phase 5: File I/O (Week 6)
- CSM import/export
- .vox import/export
- File format validation
- Cross-platform file dialogs (via rfd crate)

### Rollout Strategy
- Initially for internal use only
- Distribute as binary via GitHub releases
- Document in `doc/tools/editor.md`
- No changes to existing web app or build system

### Rollback Plan
- If editor proves too complex, continue using web editor + renderer crate
- No damage to existing crates (they remain unchanged)
- Delete `crates/editor/` and remove from workspace

## Open Questions

1. **Should editor support live reload of materials.json?**
   - Proposal: Yes via file watcher (notify crate), low priority
2. **Should editor have multiple viewports (top/side/front)?**
   - Proposal: No for v1, can add later via bevy render targets
3. **How to handle very large worlds (>128^3)?**
   - Proposal: Implement chunking when needed, profile first
4. **Should undo history persist across sessions?**
   - Proposal: No for v1, save/load separately from scene files
5. **Should editor support plugins/scripting?**
   - Proposal: No, keep monolithic for simplicity
6. **Should we integrate physics preview (drop objects, test collisions)?**
   - Proposal: Future enhancement, would use bevy_rapier3d
