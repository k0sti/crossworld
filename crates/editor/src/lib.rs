//! Voxel cube editor using the app framework
//!
//! A native OpenGL voxel editor using glow/egui/winit.

pub mod config;
pub mod cursor;
pub mod editing;
pub mod lua_config;
pub mod palette;
pub mod raycast;
pub mod ui;

use app::{App, FrameContext, InputState};
use cube::Cube;
use glam::{IVec3, Quat, Vec2, Vec3};
use glow::HasContext;
use renderer::{Camera, MeshRenderer, OrbitController, OrbitControllerConfig};
use std::path::PathBuf;
use std::rc::Rc;
use winit::keyboard::KeyCode;

use crate::lua_config::{EditorTestConfig, MouseEvent};

/// Gizmo display options
#[derive(Debug, Clone, Copy)]
pub struct GizmoOptions {
    /// Show 2D mouse pointer crosshair at screen coordinates
    pub show_2d_pointer: bool,
    /// Show 3D axis arrows at world hit position
    pub show_3d_pointer: bool,
    /// Size of 2D crosshair in pixels
    pub crosshair_size: f32,
    /// Scale of 3D axis arrows
    pub axis_scale: f32,
}

impl Default for GizmoOptions {
    fn default() -> Self {
        Self {
            show_2d_pointer: true,
            show_3d_pointer: true,
            crosshair_size: 15.0,
            axis_scale: 0.1,
        }
    }
}

use crate::config::EditorConfig;
use crate::cursor::{CubeCursor, FocusMode};
use crate::editing::EditorState;
use crate::palette::{ColorPalette, MaterialPalette, ModelPalette};
use crate::raycast::{raycast_from_mouse, EditorHit};
use crate::ui::{FileOperation, FileState};

/// Constants for the edited cube
/// Position is the CENTER of the cube (renderer uses center-aligned coordinates)
const CUBE_POSITION: Vec3 = Vec3::ZERO;
/// World scale - edge size of 1 unit
const CUBE_SCALE: f32 = 1.0;
/// Depth for edited cube (4 = 16x16x16 voxels)
const EDIT_DEPTH: u32 = 4;

/// Main editor application struct
pub struct EditorApp {
    // Rendering
    mesh_renderer: MeshRenderer,
    camera: Camera,
    orbit_controller: OrbitController,

    // Cube data
    cube: Option<Rc<Cube<u8>>>,
    cube_mesh_index: Option<usize>,

    // Editor state
    editor_state: EditorState,
    last_hit: Option<EditorHit>,

    // Cursor state
    cursor: CubeCursor,
    prev_tab_pressed: bool,
    prev_space_pressed: bool,
    prev_left_mouse_pressed: bool,

    // Current mouse position (for 2D gizmo)
    current_mouse_pos: Option<Vec2>,

    // Palette state
    color_palette: ColorPalette,
    material_palette: MaterialPalette,
    model_palette: ModelPalette,

    // File state
    file_state: FileState,

    // Editor configuration (persisted)
    config: EditorConfig,

    // Gizmo options
    gizmo_options: GizmoOptions,

    // Test configuration (optional, for automated testing)
    test_config: Option<EditorTestConfig>,

    // Injected input state for testing
    injected_input: Option<InputState>,

    // Whether we've requested exit (for debug mode)
    exit_requested: bool,
}

impl Default for EditorApp {
    fn default() -> Self {
        Self::new()
    }
}

impl EditorApp {
    /// Create a new editor application
    pub fn new() -> Self {
        // Load configuration from file
        let config = EditorConfig::load();

        // Camera target at origin (where voxel editing typically happens)
        let camera_target = Vec3::ZERO;
        // Camera positioned above and to the side, looking at the editing area
        let camera_distance = config.camera_distance;
        let camera_position = Vec3::new(
            camera_distance * 0.7,
            camera_distance * 0.5,
            camera_distance * 0.7,
        );

        // Configure orbit controller for editor-friendly controls
        let orbit_config = OrbitControllerConfig {
            mouse_sensitivity: 0.005,
            zoom_sensitivity: 0.5,
            min_distance: 1.0,
            max_distance: 50.0,
        };

        Self {
            mesh_renderer: MeshRenderer::new(),
            camera: Camera::look_at(camera_position, camera_target, Vec3::Y),
            orbit_controller: OrbitController::new(camera_target, orbit_config),
            cube: None,
            cube_mesh_index: None,
            editor_state: EditorState::new(),
            last_hit: None,
            cursor: CubeCursor::new(),
            prev_tab_pressed: false,
            prev_space_pressed: false,
            prev_left_mouse_pressed: false,
            current_mouse_pos: None,
            color_palette: ColorPalette::new(),
            material_palette: MaterialPalette::new(),
            model_palette: ModelPalette::new(),
            file_state: FileState::new(),
            config,
            gizmo_options: GizmoOptions::default(),
            test_config: None,
            injected_input: None,
            exit_requested: false,
        }
    }

    /// Create an editor with test configuration from a Lua file
    pub fn with_test_config(mut self, config: EditorTestConfig) -> Self {
        self.test_config = Some(config);
        self
    }

    /// Load test configuration from a file
    pub fn from_config_file(path: &std::path::Path) -> Self {
        let mut editor = Self::new();
        match EditorTestConfig::from_file(path) {
            Ok(test_config) => {
                println!("[Editor] Loaded test config from {:?}", path);
                if let Some(frames) = test_config.debug_frames {
                    println!("[Editor] Debug mode: running {} frames", frames);
                }
                println!("[Editor] {} scheduled events", test_config.events.len());
                println!("[Editor] {} scheduled captures", test_config.captures.len());
                editor.test_config = Some(test_config);
            }
            Err(e) => {
                eprintln!("[Editor] Failed to load test config: {}", e);
            }
        }
        editor
    }

    /// Check if exit has been requested (for debug mode)
    pub fn exit_requested(&self) -> bool {
        self.exit_requested
    }

    /// Get mutable access to gizmo options
    pub fn gizmo_options_mut(&mut self) -> &mut GizmoOptions {
        &mut self.gizmo_options
    }

    /// Get access to gizmo options
    pub fn gizmo_options(&self) -> &GizmoOptions {
        &self.gizmo_options
    }

    /// Place a voxel at the given position and re-upload the mesh
    ///
    /// # Arguments
    /// * `gl` - OpenGL context
    /// * `pos` - Position in [0, 2^depth) range
    /// * `depth` - Depth level for placement
    fn place_voxel(&mut self, gl: &glow::Context, pos: IVec3, depth: u32) {
        let Some(ref cube) = self.cube else { return };

        // Check bounds
        let max_coord = 1 << depth;
        if pos.x < 0 || pos.x >= max_coord
            || pos.y < 0 || pos.y >= max_coord
            || pos.z < 0 || pos.z >= max_coord
        {
            return;
        }

        // Create new cube with voxel set
        let material = self.editor_state.material();
        let new_cube = Rc::new(cube.set_voxel(pos.x, pos.y, pos.z, depth, material));
        self.cube = Some(Rc::clone(&new_cube));

        // Mark file as dirty
        self.file_state.mark_dirty();

        // Re-upload mesh
        unsafe {
            self.mesh_renderer.clear_meshes(gl);
            match self.mesh_renderer.upload_mesh(gl, &new_cube, EDIT_DEPTH) {
                Ok(idx) => {
                    self.cube_mesh_index = Some(idx);
                }
                Err(e) => eprintln!("[Editor] Failed to re-upload mesh: {}", e),
            }
        }
    }

    /// Remove a voxel at the given position and re-upload the mesh
    ///
    /// Removal is done by setting the voxel value to 0 (empty).
    ///
    /// # Arguments
    /// * `gl` - OpenGL context
    /// * `pos` - Position in [0, 2^depth) range
    /// * `depth` - Depth level for removal
    fn remove_voxel(&mut self, gl: &glow::Context, pos: IVec3, depth: u32) {
        let Some(ref cube) = self.cube else { return };

        // Check bounds
        let max_coord = 1 << depth;
        if pos.x < 0 || pos.x >= max_coord
            || pos.y < 0 || pos.y >= max_coord
            || pos.z < 0 || pos.z >= max_coord
        {
            return;
        }

        // Create new cube with voxel removed (set to 0)
        let new_cube = Rc::new(cube.set_voxel(pos.x, pos.y, pos.z, depth, 0));
        self.cube = Some(Rc::clone(&new_cube));

        // Mark file as dirty
        self.file_state.mark_dirty();

        // Re-upload mesh
        unsafe {
            self.mesh_renderer.clear_meshes(gl);
            match self.mesh_renderer.upload_mesh(gl, &new_cube, EDIT_DEPTH) {
                Ok(idx) => {
                    self.cube_mesh_index = Some(idx);
                }
                Err(e) => eprintln!("[Editor] Failed to re-upload mesh: {}", e),
            }
        }
    }

    // ========================================================================
    // File Operations
    // ========================================================================

    /// Create a new empty cube and reset file state
    fn new_cube(&mut self, gl: &glow::Context) {
        // Create fresh solid cube (same as init)
        let cube = Rc::new(Cube::Solid(156u8));
        self.cube = Some(Rc::clone(&cube));

        // Clear file state
        self.file_state.clear();

        // Re-upload mesh
        unsafe {
            self.mesh_renderer.clear_meshes(gl);
            match self.mesh_renderer.upload_mesh(gl, &cube, EDIT_DEPTH) {
                Ok(idx) => {
                    self.cube_mesh_index = Some(idx);
                    println!("[Editor] New cube created");
                }
                Err(e) => eprintln!("[Editor] Failed to create new cube mesh: {}", e),
            }
        }
    }

    /// Load a CSM file and replace the current cube
    fn load_csm(&mut self, gl: &glow::Context, path: PathBuf) {
        // Read file contents
        let content = match std::fs::read_to_string(&path) {
            Ok(c) => c,
            Err(e) => {
                eprintln!("[Editor] Failed to read file {:?}: {}", path, e);
                return;
            }
        };

        // Parse CSM
        let cube = match cube::parse_csm(&content) {
            Ok(c) => Rc::new(c),
            Err(e) => {
                eprintln!("[Editor] Failed to parse CSM file {:?}: {}", path, e);
                return;
            }
        };

        // Update editor state
        self.cube = Some(Rc::clone(&cube));
        self.file_state.set_file(path.clone());

        // Update config with last model path
        self.config.set_last_model_path(Some(path.clone()));

        // Re-upload mesh
        unsafe {
            self.mesh_renderer.clear_meshes(gl);
            match self.mesh_renderer.upload_mesh(gl, &cube, EDIT_DEPTH) {
                Ok(idx) => {
                    self.cube_mesh_index = Some(idx);
                    println!("[Editor] Loaded CSM file: {:?}", path);
                }
                Err(e) => eprintln!("[Editor] Failed to upload loaded cube mesh: {}", e),
            }
        }
    }

    /// Save the current cube to a CSM file
    fn save_csm(&mut self, path: PathBuf) {
        let Some(ref cube) = self.cube else {
            eprintln!("[Editor] No cube to save");
            return;
        };

        // Serialize to CSM format
        let content = cube::serialize_csm(cube);

        // Write to file
        match std::fs::write(&path, content) {
            Ok(()) => {
                self.file_state.set_file(path.clone());
                // Update config with last model path
                self.config.set_last_model_path(Some(path.clone()));
                println!("[Editor] Saved CSM file: {:?}", path);
            }
            Err(e) => {
                eprintln!("[Editor] Failed to write file {:?}: {}", path, e);
            }
        }
    }

    /// Import a VOX file into the model palette
    fn import_vox(&mut self, path: PathBuf) {
        // Read file bytes
        let bytes = match std::fs::read(&path) {
            Ok(b) => b,
            Err(e) => {
                eprintln!("[Editor] Failed to read VOX file {:?}: {}", path, e);
                return;
            }
        };

        // Get filename for display
        let name = path
            .file_stem()
            .and_then(|n| n.to_str())
            .unwrap_or("unnamed")
            .to_string();

        // Load into model palette
        match self.model_palette.load_from_bytes(&bytes, &name) {
            Ok(id) => {
                println!("[Editor] Imported VOX model '{}' (id: {})", name, id);
            }
            Err(e) => {
                eprintln!("[Editor] Failed to load VOX file {:?}: {}", path, e);
            }
        }
    }

    /// Handle a file operation triggered from the UI
    fn handle_file_operation(&mut self, gl: &glow::Context, operation: FileOperation) {
        match operation {
            FileOperation::New => {
                self.new_cube(gl);
            }
            FileOperation::Open => {
                // Open file dialog for CSM files
                if let Some(path) = rfd::FileDialog::new()
                    .add_filter("CSM files", &["csm"])
                    .add_filter("All files", &["*"])
                    .pick_file()
                {
                    self.load_csm(gl, path);
                }
            }
            FileOperation::Save => {
                // Save to current file or prompt for new file
                if let Some(path) = self.file_state.current_file.clone() {
                    self.save_csm(path);
                } else {
                    // No current file, do Save As
                    if let Some(path) = rfd::FileDialog::new()
                        .add_filter("CSM files", &["csm"])
                        .set_file_name("untitled.csm")
                        .save_file()
                    {
                        self.save_csm(path);
                    }
                }
            }
            FileOperation::SaveAs => {
                // Always prompt for new file
                if let Some(path) = rfd::FileDialog::new()
                    .add_filter("CSM files", &["csm"])
                    .set_file_name("untitled.csm")
                    .save_file()
                {
                    self.save_csm(path);
                }
            }
            FileOperation::ImportVox => {
                // Open file dialog for VOX files
                if let Some(path) = rfd::FileDialog::new()
                    .add_filter("VOX files", &["vox"])
                    .add_filter("All files", &["*"])
                    .pick_file()
                {
                    self.import_vox(path);
                }
            }
        }
    }
}

impl App for EditorApp {
    fn init(&mut self, ctx: &FrameContext) {
        println!("[Editor] Initializing voxel editor");

        // Initialize mesh renderer
        if let Err(e) = unsafe { self.mesh_renderer.init_gl(ctx.gl) } {
            eprintln!("[Editor] Failed to initialize mesh renderer: {}", e);
            return;
        }

        // Try to load the last opened model, or create a fresh one
        if let Some(ref last_path) = self.config.last_model_path.clone() {
            if last_path.exists() {
                println!("[Editor] Loading last model: {:?}", last_path);
                self.load_csm(ctx.gl, last_path.clone());
                return;
            } else {
                println!(
                    "[Editor] Last model not found: {:?}, creating new cube",
                    last_path
                );
            }
        }

        // Create initial cube - solid with a colorful material (material index 156 = green-ish)
        let cube = Rc::new(Cube::Solid(156u8));
        self.cube = Some(Rc::clone(&cube));

        // Upload cube mesh at EDIT_DEPTH for voxel editing
        match unsafe { self.mesh_renderer.upload_mesh(ctx.gl, &cube, EDIT_DEPTH) } {
            Ok(idx) => {
                self.cube_mesh_index = Some(idx);
                println!("[Editor] Cube mesh uploaded (index: {})", idx);
            }
            Err(e) => eprintln!("[Editor] Failed to upload cube mesh: {}", e),
        }
    }

    fn shutdown(&mut self, ctx: &FrameContext) {
        println!("[Editor] Shutting down");
        unsafe { self.mesh_renderer.destroy_gl(ctx.gl) };
    }

    fn update(&mut self, ctx: &FrameContext, input: &InputState) {
        // Process test configuration events if present
        if let Some(ref test_config) = self.test_config {
            // Check if we should exit
            if test_config.should_exit(ctx.frame) {
                println!("[Editor] Debug mode: exiting after {} frames", ctx.frame);
                self.exit_requested = true;
            }

            // Process scheduled events for this frame
            let events = test_config.events_for_frame(ctx.frame);
            if !events.is_empty() {
                // Create or get injected input state
                let injected = self.injected_input.get_or_insert_with(|| {
                    let mut state = InputState::default();
                    // Initialize with last known mouse position
                    if let Some(pos) = test_config.mouse_position_at_frame(ctx.frame.saturating_sub(1)) {
                        state.mouse_pos = Some(pos);
                    }
                    state
                });

                for event in events {
                    match &event.event {
                        MouseEvent::Move { x, y } => {
                            println!("[Editor] Frame {}: Injecting mouse move to ({}, {})", ctx.frame, x, y);
                            injected.inject_mouse_pos(*x, *y);
                        }
                        MouseEvent::Click { button, pressed } => {
                            println!("[Editor] Frame {}: Injecting {:?} click (pressed={})", ctx.frame, button, pressed);
                            injected.inject_mouse_click(*button, *pressed);
                        }
                    }
                }
            }
        }

        // Use injected input if available, otherwise use real input
        let effective_input = self.injected_input.as_ref().unwrap_or(input);

        // Handle camera orbit with right-mouse drag
        if effective_input.is_right_mouse_pressed() {
            let yaw_delta = -effective_input.mouse_delta.x * self.orbit_controller.config.mouse_sensitivity;
            let pitch_delta = -effective_input.mouse_delta.y * self.orbit_controller.config.mouse_sensitivity;
            self.orbit_controller.rotate(yaw_delta, pitch_delta, &mut self.camera);
        }

        // Handle camera zoom with scroll wheel
        // Scale scroll delta to get reasonable zoom amount
        // (OrbitController's apply_zoom uses 0.01 multiplier designed for egui smooth_scroll)
        if effective_input.scroll_delta.y.abs() > 0.01 {
            let zoom_delta = effective_input.scroll_delta.y * 100.0; // Scale up for OrbitController
            self.orbit_controller.zoom(zoom_delta, &mut self.camera);
        }

        // Handle Tab key to toggle Near/Far focus mode
        let tab_pressed = effective_input.is_key_pressed(KeyCode::Tab);
        if tab_pressed && !self.prev_tab_pressed {
            self.cursor.toggle_mode();
        }
        self.prev_tab_pressed = tab_pressed;

        // Handle Space key to toggle Near/Far focus mode (alternative binding)
        let space_pressed = effective_input.is_key_pressed(KeyCode::Space);
        if space_pressed && !self.prev_space_pressed {
            self.cursor.toggle_mode();
            if self.test_config.is_some() {
                let mode_name = match self.cursor.focus_mode {
                    FocusMode::Near => "Near",
                    FocusMode::Far => "Far",
                };
                println!("[DEBUG Frame {}] Toggled focus mode to: {}", ctx.frame, mode_name);
            }
        }
        self.prev_space_pressed = space_pressed;

        // Store current mouse position for 2D gizmo rendering
        self.current_mouse_pos = effective_input.mouse_pos;

        // Update cursor from mouse raycast (only if mouse position is available)
        if let (Some(ref cube), Some(mouse_pos)) = (&self.cube, effective_input.mouse_pos) {
            let screen_size = Vec2::new(ctx.size.0 as f32, ctx.size.1 as f32);

            // Use the cursor's current depth for raycast selection
            let cursor_depth = self.cursor.coord.depth;

            if let Some(hit) = raycast_from_mouse(
                mouse_pos,
                screen_size,
                &self.camera,
                cube,
                CUBE_POSITION,
                CUBE_SCALE,
                Some(EDIT_DEPTH),
            ) {
                // Use the new coord selection logic that handles far/near based on boundary detection
                // Select at cursor's depth, not EDIT_DEPTH
                let far_mode = self.cursor.focus_mode == FocusMode::Far;
                let (selected_coord, is_boundary) = hit.select_coord_at_depth(
                    cursor_depth,
                    far_mode,
                    CUBE_POSITION,
                    CUBE_SCALE,
                );

                // Debug logging for selected CubeCoord
                if self.test_config.is_some() {
                    let mode_name = if far_mode { "Far" } else { "Near" };
                    let boundary_str = if is_boundary { "boundary" } else { "interior" };
                    println!(
                        "[DEBUG Frame {}] Raycast hit: world_pos=({:.3}, {:.3}, {:.3}), normal={:?}, hit_depth={}, cursor_depth={}, hit_voxel=({}, {}, {})",
                        ctx.frame, hit.world_pos.x, hit.world_pos.y, hit.world_pos.z,
                        hit.normal, hit.cube_coord.depth, cursor_depth,
                        hit.voxel_coord.x, hit.voxel_coord.y, hit.voxel_coord.z
                    );
                    // Also log the voxel_at_depth result before far/near adjustment
                    let base_voxel = hit.voxel_at_depth(cursor_depth, CUBE_POSITION, CUBE_SCALE);
                    println!(
                        "[DEBUG Frame {}] voxel_at_depth({})=({}, {}, {}), Selected=({}, {}, {}), mode={}, face_type={}",
                        ctx.frame, cursor_depth, base_voxel.x, base_voxel.y, base_voxel.z,
                        selected_coord.x, selected_coord.y, selected_coord.z,
                        mode_name, boundary_str
                    );
                }

                // Update cursor position without changing its depth
                self.cursor.update_from_selected_coord_preserve_depth(
                    hit.world_pos,
                    hit.normal,
                    selected_coord,
                    is_boundary,
                );
                self.last_hit = Some(hit);
            } else {
                self.cursor.invalidate();
                self.last_hit = None;
            }
        }

        // Handle left-click for voxel placement/removal
        let left_mouse_pressed = effective_input.is_left_mouse_pressed();
        let left_click = left_mouse_pressed && !self.prev_left_mouse_pressed;
        self.prev_left_mouse_pressed = left_mouse_pressed;

        // Check if Shift is held (for removal mode)
        let shift_held = effective_input.is_key_pressed(KeyCode::ShiftLeft)
            || effective_input.is_key_pressed(KeyCode::ShiftRight);

        if left_click && self.cursor.valid {
            // Use the cursor's selected coordinate (which already handles far/near mode)
            let selected_pos = self.cursor.coord.pos;

            if shift_held {
                // Shift+left-click: remove voxel at selected position
                self.remove_voxel(ctx.gl, selected_pos, EDIT_DEPTH);
            } else {
                // Left-click: place voxel at selected position
                self.place_voxel(ctx.gl, selected_pos, EDIT_DEPTH);
            }
        }
    }

    fn render(&mut self, ctx: &FrameContext) {
        let width = ctx.size.0 as i32;
        let height = ctx.size.1 as i32;

        // Clear the framebuffer
        unsafe {
            ctx.gl.viewport(0, 0, width, height);
            ctx.gl.clear_color(0.1, 0.1, 0.15, 1.0);
            ctx.gl.clear(glow::COLOR_BUFFER_BIT | glow::DEPTH_BUFFER_BIT);
        }

        // Render the cube mesh at the center
        if let Some(mesh_index) = self.cube_mesh_index {
            unsafe {
                self.mesh_renderer.render_mesh_with_scale(
                    ctx.gl,
                    mesh_index,
                    CUBE_POSITION,
                    Quat::IDENTITY,
                    CUBE_SCALE,
                    &self.camera,
                    width,
                    height,
                );
            }
        }

        // Render white wireframe around the ground cube
        // CUBE_POSITION is already the center (renderer uses center-aligned coordinates)
        unsafe {
            self.mesh_renderer.render_cubebox_wireframe_colored(
                ctx.gl,
                CUBE_POSITION,
                Quat::IDENTITY,
                Vec3::ONE,
                CUBE_SCALE,
                [1.0, 1.0, 1.0], // White
                &self.camera,
                width,
                height,
            );
        }

        // Render cursor wireframe when valid
        if self.cursor.valid {
            // Get cursor position and size in world space
            let cursor_center = self.cursor.world_center(CUBE_POSITION, CUBE_SCALE);
            let cursor_size = self.cursor.world_size(CUBE_SCALE);
            let cursor_color = self.cursor.wireframe_color();

            // Debug output for cursor rendering
            if self.test_config.is_some() {
                println!(
                    "[DEBUG Frame {}] Cursor render: center=({:.3}, {:.3}, {:.3}), size=({:.3}, {:.3}, {:.3}), color=({:.1}, {:.1}, {:.1})",
                    ctx.frame,
                    cursor_center.x, cursor_center.y, cursor_center.z,
                    cursor_size.x, cursor_size.y, cursor_size.z,
                    cursor_color[0], cursor_color[1], cursor_color[2]
                );
            }

            unsafe {
                // Render cursor at calculated position with cursor size
                self.mesh_renderer.render_cubebox_wireframe_colored(
                    ctx.gl,
                    cursor_center,
                    Quat::IDENTITY,
                    Vec3::ONE, // Normalized size (full box)
                    cursor_size.x, // Scale to cursor size
                    cursor_color,
                    &self.camera,
                    width,
                    height,
                );
            }
        }

        // Render 3D axis arrows gizmo at hit position
        if self.gizmo_options.show_3d_pointer {
            if let Some(ref hit) = self.last_hit {
                unsafe {
                    self.mesh_renderer.render_3d_axis_arrows(
                        ctx.gl,
                        hit.world_pos,
                        self.gizmo_options.axis_scale,
                        &self.camera,
                        width,
                        height,
                    );
                }
            }
        }

        // Render 2D crosshair gizmo at mouse position
        if self.gizmo_options.show_2d_pointer {
            if let Some(mouse_pos) = self.current_mouse_pos {
                // Yellow crosshair at mouse position
                unsafe {
                    self.mesh_renderer.render_2d_crosshair(
                        ctx.gl,
                        mouse_pos,
                        self.gizmo_options.crosshair_size,
                        [1.0, 1.0, 0.0], // Yellow
                        width,
                        height,
                    );
                }
            }
        }

        // Handle frame captures from test configuration
        if let Some(ref test_config) = self.test_config {
            let captures = test_config.captures_for_frame(ctx.frame);
            for capture in captures {
                println!("[Editor] Frame {}: Capturing to {:?}", ctx.frame, capture.path);

                // Ensure parent directory exists
                if let Some(parent) = capture.path.parent() {
                    if !parent.exists() {
                        if let Err(e) = std::fs::create_dir_all(parent) {
                            eprintln!("[Editor] Failed to create output directory: {}", e);
                            continue;
                        }
                    }
                }

                // Use the renderer's save function
                let path_str = capture.path.to_string_lossy();
                if let Err(e) = self.mesh_renderer.save_framebuffer_to_file(
                    ctx.gl,
                    ctx.size.0,
                    ctx.size.1,
                    &path_str,
                ) {
                    eprintln!("[Editor] Failed to capture frame: {}", e);
                }
            }
        }
    }

    fn should_exit(&self) -> bool {
        self.exit_requested
    }

    fn ui(&mut self, ctx: &FrameContext, egui_ctx: &egui::Context) {
        // Track file operation to handle after UI
        let mut file_operation: Option<FileOperation> = None;

        // Top panel with menu bar and title
        egui::TopBottomPanel::top("editor_top_panel").show(egui_ctx, |ui| {
            // Menu bar
            egui::MenuBar::new().ui(ui, |ui| {
                // File menu
                if let Some(op) = ui::show_file_menu(ui, &self.file_state) {
                    file_operation = Some(op);
                }

                ui.separator();

                // Title with filename
                ui.heading(format!(
                    "Crossworld Voxel Editor - {}",
                    self.file_state.display_name()
                ));

                ui.separator();

                // Mode indicator
                ui.label(format!(
                    "Mode: {}",
                    if self.cursor.focus_mode == FocusMode::Near {
                        "Near (Remove)"
                    } else {
                        "Far (Place)"
                    }
                ));
            });
        });

        // Status bar at the bottom
        let status_info =
            ui::StatusBarInfo::from_state(&self.cursor, &self.editor_state, ctx.delta_time);
        ui::show_status_bar(egui_ctx, &status_info);

        // Unified sidebar on the left with all palettes and cursor info
        let _sidebar_result = ui::show_unified_sidebar(
            egui_ctx,
            &mut self.cursor,
            EDIT_DEPTH,
            &mut self.color_palette,
            &mut self.material_palette,
            &mut self.model_palette,
            &mut self.editor_state,
        );

        // Handle file operations after UI rendering
        if let Some(operation) = file_operation {
            self.handle_file_operation(ctx.gl, operation);
        }
    }
}

/// Export the create_app function for dynamic loading (hot-reload support)
#[no_mangle]
#[allow(improper_ctypes_definitions)]
pub extern "C" fn create_app() -> *mut dyn App {
    Box::into_raw(Box::new(EditorApp::new()))
}
