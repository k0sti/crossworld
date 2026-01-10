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

use app::{App, Camera, FrameContext, InputState, OrbitController, OrbitControllerConfig};
use cube::Cube;
use glam::{IVec3, Quat, Vec2, Vec3};
use glow::HasContext;
use renderer::{MeshRenderer, SkyboxRenderer};
use std::path::PathBuf;
use std::rc::Rc;
use winit::keyboard::KeyCode;

use crate::lua_config::{EditorTestConfig, MouseEvent};
use cube::io::vox::load_vox_to_cubebox_compact;
use image::ImageBuffer;
use std::sync::mpsc::{channel, Receiver};

/// Type alias for thumbnail image
type ThumbnailImage = ImageBuffer<image::Rgb<u8>, Vec<u8>>;

/// Load a model thumbnail in background thread
fn load_model_thumbnail(path: &PathBuf) -> Result<(IVec3, ThumbnailImage), String> {
    // Load VOX file
    let bytes = std::fs::read(path).map_err(|e| format!("Failed to read file: {}", e))?;
    let cubebox = load_vox_to_cubebox_compact(&bytes)?;
    let size = cubebox.size;

    // Generate thumbnail
    let cube_rc = Rc::new(cubebox.cube.clone());
    let thumbnail = renderer::thumbnail::generate_thumbnail_default(cube_rc);

    Ok((size, thumbnail))
}

/// Message from background model loading thread
#[derive(Debug)]
enum ModelLoadMessage {
    /// A model has been loaded with its size and thumbnail
    ModelLoaded {
        id: usize,
        size: IVec3,
        thumbnail: image::ImageBuffer<image::Rgb<u8>, Vec<u8>>,
    },
    /// Loading completed for all models
    LoadingComplete {
        total: usize,
        succeeded: usize,
        failed: usize,
    },
}

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
use crate::raycast::{mouse_to_ray, raycast_from_mouse, EditorHit};
use crate::ui::{FileOperation, FileState};

/// Constants for the edited cube
/// Position is the CENTER of the cube (renderer uses center-aligned coordinates)
const CUBE_POSITION: Vec3 = Vec3::ZERO;
/// Base world scale - edge size of 1 unit (before world_scale factor)
const BASE_CUBE_SCALE: f32 = 1.0;
/// Depth for edited cube (4 = 16x16x16 voxels)
const EDIT_DEPTH: u32 = 4;
/// Line width/strength for all wireframes and gizmo lines (relative to voxel size)
const LINE_WIDTH_FACTOR: f32 = 0.01;

/// Edit plane for drag operations
#[derive(Debug, Clone)]
struct EditPlane {
    /// Origin point of the plane in world space
    origin: Vec3,
    /// Normal of the plane
    normal: Vec3,
    /// Depth at which editing occurs
    depth: u32,
    /// Right vector for plane coordinates
    right: Vec3,
    /// Up vector for plane coordinates
    up: Vec3,
    /// Original raycast hit position
    hit_pos: Vec3,
}

impl EditPlane {
    /// Create a new edit plane from cursor position (which already accounts for Far/Near mode)
    fn from_cursor(
        _cursor_pos: IVec3,
        normal: Vec3,
        depth: u32,
        _cube_position: Vec3,
        _cube_scale: f32,
        hit_pos: Vec3,
    ) -> Self {
        // Calculate right and up vectors for the plane
        let (right, up) = if normal.x.abs() > 0.5 {
            // Normal is mostly along X axis
            (Vec3::new(0.0, 1.0, 0.0), Vec3::new(0.0, 0.0, 1.0))
        } else if normal.y.abs() > 0.5 {
            // Normal is mostly along Y axis
            (Vec3::new(1.0, 0.0, 0.0), Vec3::new(0.0, 0.0, 1.0))
        } else {
            // Normal is mostly along Z axis
            (Vec3::new(1.0, 0.0, 0.0), Vec3::new(0.0, 1.0, 0.0))
        };

        // Use the actual hit position as the plane origin
        // This ensures the plane passes through the exact raycast hit point
        let origin = hit_pos;

        Self {
            origin,
            normal,
            depth,
            right,
            up,
            hit_pos,
        }
    }

    /// Project a ray onto the plane and return the intersection point
    fn project_ray(&self, ray_origin: Vec3, ray_direction: Vec3) -> Option<Vec3> {
        let denom = ray_direction.dot(self.normal);
        if denom.abs() < 1e-6 {
            return None; // Ray is parallel to plane
        }

        let t = (self.origin - ray_origin).dot(self.normal) / denom;
        if t < 0.0 {
            return None; // Plane is behind ray origin
        }

        Some(ray_origin + ray_direction * t)
    }

    /// Convert world position to the nearest voxel corner at the plane's depth
    /// Uses rounding to get the nearest corner, not floor
    fn world_to_nearest_voxel_corner(
        &self,
        world_pos: Vec3,
        cube_position: Vec3,
        cube_scale: f32,
    ) -> IVec3 {
        let half_scale = cube_scale * 0.5;
        let cube_pos = (world_pos - cube_position) / half_scale;

        let scale = (1 << self.depth) as f32 / 2.0;
        IVec3::new(
            ((cube_pos.x + 1.0) * scale).round() as i32,
            ((cube_pos.y + 1.0) * scale).round() as i32,
            ((cube_pos.z + 1.0) * scale).round() as i32,
        )
    }

    /// Convert voxel corner coordinate back to world position
    fn voxel_corner_to_world(
        &self,
        voxel_pos: IVec3,
        cube_position: Vec3,
        cube_scale: f32,
    ) -> Vec3 {
        let half_scale = cube_scale * 0.5;
        let scale = (1 << self.depth) as f32 / 2.0;

        let cube_pos = Vec3::new(
            voxel_pos.x as f32 / scale - 1.0,
            voxel_pos.y as f32 / scale - 1.0,
            voxel_pos.z as f32 / scale - 1.0,
        );

        cube_position + cube_pos * half_scale
    }
}

/// Main editor application struct
pub struct EditorApp {
    // Rendering
    mesh_renderer: MeshRenderer,
    skybox_renderer: SkyboxRenderer,
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

    // Edit plane state (for drag operations)
    edit_plane: Option<EditPlane>,

    // Current drag target world position (exact ray-plane intersection, not snapped)
    drag_target_world_pos: Option<Vec3>,

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

    // Model loading progress
    model_load_receiver: Option<Receiver<ModelLoadMessage>>,
    models_loading: bool,
    models_loaded_count: usize,
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
            skybox_renderer: SkyboxRenderer::new(),
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
            edit_plane: None,
            drag_target_world_pos: None,
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
            model_load_receiver: None,
            models_loading: false,
            models_loaded_count: 0,
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

    /// Get the effective cube scale (base scale * 2^world_scale)
    fn effective_cube_scale(&self) -> f32 {
        BASE_CUBE_SCALE * self.editor_state.cube_scale_factor()
    }

    /// Get the effective edit depth (EDIT_DEPTH + 2 * world_scale)
    /// Each world_scale increment corresponds to 2 octree levels from expansion
    #[allow(dead_code)]
    fn effective_edit_depth(&self) -> u32 {
        EDIT_DEPTH + 2 * self.editor_state.world_scale()
    }

    /// Place a voxel at the given position and re-upload the mesh
    ///
    /// If the position is outside the current cube bounds, the cube will be
    /// expanded using Cube::expand_once with material 0 (empty) borders, and the
    /// world_scale will be incremented. The expansion adds 2 octree levels, so
    /// coordinates shift by 2^depth to place original content in center.
    ///
    /// # Arguments
    /// * `gl` - OpenGL context
    /// * `pos` - Position in [0, 2^effective_depth) range
    /// * `depth` - Depth level for placement
    fn place_voxel(&mut self, gl: &glow::Context, pos: IVec3, depth: u32) {
        let Some(ref cube) = self.cube else { return };

        // Calculate effective depth based on world_scale
        // Each expansion adds 2 octree levels
        let effective_depth = depth + 2 * self.editor_state.world_scale();

        // Check bounds at effective depth
        let max_coord = 1i32 << effective_depth;
        let in_bounds = pos.x >= 0
            && pos.x < max_coord
            && pos.y >= 0
            && pos.y < max_coord
            && pos.z >= 0
            && pos.z < max_coord;

        let (new_cube, new_pos, new_effective_depth) = if in_bounds {
            // Normal case: position is within bounds
            ((**cube).clone(), pos, effective_depth)
        } else {
            // Position is outside bounds - expand the cube
            // expand_once creates a 4x4x4 grid (2 octree levels) with original in center 2x2x2
            let expanded = Cube::expand_once(cube, [0, 0, 0, 0]);

            // After expansion, coordinates shift by 2^effective_depth to center the original
            // New coordinate space is [0, 2^(effective_depth+2))
            // Original [0, 2^effective_depth) maps to [2^effective_depth, 2^(effective_depth+1))
            let offset = 1i32 << effective_depth;
            let new_pos = pos + IVec3::splat(offset);
            let new_effective_depth = effective_depth + 2;

            // Increment world scale
            self.editor_state.increment_world_scale();

            println!(
                "[Editor] Expanded cube, new world_scale: {}, new_pos: {:?}",
                self.editor_state.world_scale(),
                new_pos
            );

            (expanded, new_pos, new_effective_depth)
        };

        // Check bounds again after expansion
        let max_coord = 1i32 << new_effective_depth;
        if new_pos.x < 0
            || new_pos.x >= max_coord
            || new_pos.y < 0
            || new_pos.y >= max_coord
            || new_pos.z < 0
            || new_pos.z >= max_coord
        {
            // Still out of bounds after one expansion - would need multiple expansions
            eprintln!(
                "[Editor] Position {:?} still out of bounds after expansion (max: {})",
                new_pos, max_coord
            );
            return;
        }

        // Create new cube with voxel set at the effective depth
        let material = self.editor_state.effective_material();
        let new_cube = Rc::new(new_cube.set_voxel(
            new_pos.x,
            new_pos.y,
            new_pos.z,
            new_effective_depth,
            material,
        ));
        self.cube = Some(Rc::clone(&new_cube));

        // Mark file as dirty
        self.file_state.mark_dirty();

        // Re-upload mesh at the effective depth
        unsafe {
            self.mesh_renderer.clear_meshes(gl);
            match self
                .mesh_renderer
                .upload_mesh(gl, &new_cube, new_effective_depth)
            {
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
    /// Uses effective_edit_depth to account for world_scale.
    ///
    /// # Arguments
    /// * `gl` - OpenGL context
    /// * `pos` - Position in [0, 2^effective_depth) range
    /// * `depth` - Base depth level for removal (will use effective depth)
    fn remove_voxel(&mut self, gl: &glow::Context, pos: IVec3, depth: u32) {
        let Some(ref cube) = self.cube else { return };

        // Calculate effective depth based on world_scale
        let effective_depth = depth + 2 * self.editor_state.world_scale();

        // Check bounds at effective depth
        let max_coord = 1i32 << effective_depth;
        if pos.x < 0
            || pos.x >= max_coord
            || pos.y < 0
            || pos.y >= max_coord
            || pos.z < 0
            || pos.z >= max_coord
        {
            return;
        }

        // Create new cube with voxel removed (set to 0)
        let new_cube = Rc::new(cube.set_voxel(pos.x, pos.y, pos.z, effective_depth, 0));
        self.cube = Some(Rc::clone(&new_cube));

        // Mark file as dirty
        self.file_state.mark_dirty();

        // Re-upload mesh at effective depth
        unsafe {
            self.mesh_renderer.clear_meshes(gl);
            match self
                .mesh_renderer
                .upload_mesh(gl, &new_cube, effective_depth)
            {
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

        // Reset world scale to 0
        self.editor_state.set_world_scale(0);

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

        // Reset world scale to 0 when loading a new file
        self.editor_state.set_world_scale(0);

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

    /// Load model filenames from the assets/models/vox directory
    fn load_models_from_assets(&mut self) {
        let assets_path = PathBuf::from("assets/models/vox");

        // Check if the directory exists
        if !assets_path.exists() || !assets_path.is_dir() {
            eprintln!("[Editor] Models directory not found: {:?}", assets_path);
            return;
        }

        // Read all files in the directory
        let entries = match std::fs::read_dir(&assets_path) {
            Ok(entries) => entries,
            Err(e) => {
                eprintln!("[Editor] Failed to read models directory: {}", e);
                return;
            }
        };

        let mut count = 0;
        for entry in entries {
            let entry = match entry {
                Ok(e) => e,
                Err(_) => continue,
            };

            let path = entry.path();

            // Only process .vox files
            if path.extension().and_then(|s| s.to_str()) != Some("vox") {
                continue;
            }

            // Get filename for display
            let name = path
                .file_stem()
                .and_then(|n| n.to_str())
                .unwrap_or("unnamed")
                .to_string();

            // Add model entry to palette (without loading the file)
            match self.model_palette.add_model(path, &name) {
                Ok(_id) => {
                    count += 1;
                }
                Err(e) => {
                    eprintln!("[Editor] Failed to add model '{}': {}", name, e);
                }
            }
        }

        println!("[Editor] Found {} VOX models in {:?}", count, assets_path);
        println!("[Editor] Placeholder thumbnails generated, starting background loading...");

        // Start background thread to load thumbnails
        self.start_background_loading();
    }

    /// Start a background thread to load model thumbnails
    fn start_background_loading(&mut self) {
        // Create channel for communication
        let (tx, rx) = channel();
        self.model_load_receiver = Some(rx);
        self.models_loading = true;

        // Collect model loading info (id and path)
        let models_to_load: Vec<(usize, PathBuf)> = self
            .model_palette
            .iter()
            .map(|model| (model.id, model.file_path.clone()))
            .collect();

        let total_models = models_to_load.len();
        println!(
            "[Editor] Starting background loading for {} models",
            total_models
        );

        // Spawn background thread
        std::thread::spawn(move || {
            let mut succeeded = 0;
            let mut failed = 0;

            for (id, path) in models_to_load {
                // Load model and generate thumbnail
                match load_model_thumbnail(&path) {
                    Ok((size, thumbnail)) => {
                        // Send loaded data back
                        if tx
                            .send(ModelLoadMessage::ModelLoaded {
                                id,
                                size,
                                thumbnail,
                            })
                            .is_err()
                        {
                            eprintln!(
                                "[Editor] Background thread: failed to send loaded model {}",
                                id
                            );
                            break;
                        }
                        succeeded += 1;
                    }
                    Err(e) => {
                        eprintln!(
                            "[Editor] Background thread: failed to load model {}: {}",
                            id, e
                        );
                        failed += 1;
                    }
                }
            }

            // Send completion message
            let _ = tx.send(ModelLoadMessage::LoadingComplete {
                total: total_models,
                succeeded,
                failed,
            });
        });
    }

    /// Check for model loading updates from the background thread
    fn process_model_loading_updates(&mut self) {
        let Some(ref receiver) = self.model_load_receiver else {
            return;
        };

        // Process all available messages (non-blocking)
        while let Ok(msg) = receiver.try_recv() {
            match msg {
                ModelLoadMessage::ModelLoaded {
                    id,
                    size,
                    thumbnail,
                } => {
                    // Update the model in the palette
                    if let Some(model) = self.model_palette.get_model_by_id_mut(id) {
                        model.size = Some(size);
                        model.thumbnail = Some(thumbnail);
                        self.models_loaded_count += 1;
                    }
                }
                ModelLoadMessage::LoadingComplete {
                    total,
                    succeeded,
                    failed,
                } => {
                    println!(
                        "[Editor] Model loading complete: {}/{} succeeded, {} failed",
                        succeeded, total, failed
                    );
                    self.models_loading = false;
                }
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

        // Initialize skybox renderer
        if let Err(e) = unsafe { self.skybox_renderer.init_gl(ctx.gl) } {
            eprintln!("[Editor] Failed to initialize skybox renderer: {}", e);
            return;
        }

        // Load all VOX models from assets directory
        self.load_models_from_assets();

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
        unsafe {
            self.skybox_renderer.destroy_gl(ctx.gl);
            self.mesh_renderer.destroy_gl(ctx.gl);
        }
    }

    fn update(&mut self, ctx: &FrameContext, input: &InputState) {
        // Process model loading updates from background thread
        self.process_model_loading_updates();

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
                    if let Some(pos) =
                        test_config.mouse_position_at_frame(ctx.frame.saturating_sub(1))
                    {
                        state.mouse_pos = Some(pos);
                    }
                    state
                });

                for event in events {
                    match &event.event {
                        MouseEvent::Move { x, y } => {
                            println!(
                                "[Editor] Frame {}: Injecting mouse move to ({}, {})",
                                ctx.frame, x, y
                            );
                            injected.inject_mouse_pos(*x, *y);
                        }
                        MouseEvent::Click { button, pressed } => {
                            println!(
                                "[Editor] Frame {}: Injecting {:?} click (pressed={})",
                                ctx.frame, button, pressed
                            );
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
            let yaw_delta =
                -effective_input.mouse_delta.x * self.orbit_controller.config.mouse_sensitivity;
            let pitch_delta =
                -effective_input.mouse_delta.y * self.orbit_controller.config.mouse_sensitivity;
            self.orbit_controller
                .rotate(yaw_delta, pitch_delta, &mut self.camera);
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
                println!(
                    "[DEBUG Frame {}] Toggled focus mode to: {}",
                    ctx.frame, mode_name
                );
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
                self.effective_cube_scale(),
                Some(EDIT_DEPTH),
            ) {
                // Use the new coord selection logic that handles far/near based on boundary detection
                // Select at cursor's depth, not EDIT_DEPTH
                let far_mode = self.cursor.focus_mode == FocusMode::Far;
                let (selected_coord, is_boundary) = hit.select_coord_at_depth(
                    cursor_depth,
                    far_mode,
                    CUBE_POSITION,
                    self.effective_cube_scale(),
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
                    let base_voxel = hit.voxel_at_depth(
                        cursor_depth,
                        CUBE_POSITION,
                        self.effective_cube_scale(),
                    );
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

        // Handle left-click and drag for voxel placement/removal
        let left_mouse_pressed = effective_input.is_left_mouse_pressed();
        let left_click = left_mouse_pressed && !self.prev_left_mouse_pressed;
        let left_release = !left_mouse_pressed && self.prev_left_mouse_pressed;

        // Check if Shift is held (for removal mode)
        let shift_held = effective_input.is_key_pressed(KeyCode::ShiftLeft)
            || effective_input.is_key_pressed(KeyCode::ShiftRight);

        // Extract mouse position before mutable borrows
        let mouse_pos = effective_input.mouse_pos;

        // On mouse down: create edit plane from cursor (which respects Far/Near mode)
        if left_click && self.cursor.valid {
            if let Some(ref hit) = self.last_hit {
                let cursor_pos = self.cursor.coord.pos;
                let cursor_depth = self.cursor.coord.depth;
                let normal = hit.normal.to_vec3();

                self.edit_plane = Some(EditPlane::from_cursor(
                    cursor_pos,
                    normal,
                    cursor_depth,
                    CUBE_POSITION,
                    self.effective_cube_scale(),
                    hit.world_pos,
                ));

                // Place/remove first voxel
                if shift_held {
                    self.remove_voxel(ctx.gl, cursor_pos, cursor_depth);
                } else {
                    self.place_voxel(ctx.gl, cursor_pos, cursor_depth);
                }
            }
        }

        // On mouse drag: project mouse to edit plane and place voxels
        if left_mouse_pressed && !left_click {
            if let (Some(ref plane), Some(mouse_pos)) = (&self.edit_plane, mouse_pos) {
                let screen_size = Vec2::new(ctx.size.0 as f32, ctx.size.1 as f32);
                let ray = mouse_to_ray(mouse_pos, screen_size, &self.camera);

                if let Some(world_pos) = plane.project_ray(ray.origin, ray.direction) {
                    // Store the exact world position for gizmo rendering (not snapped to voxel grid)
                    self.drag_target_world_pos = Some(world_pos);

                    // Get the nearest voxel corner to the ray-plane intersection
                    let nearest_corner = plane.world_to_nearest_voxel_corner(
                        world_pos,
                        CUBE_POSITION,
                        self.effective_cube_scale(),
                    );
                    let cursor_depth = plane.depth;

                    // Apply Far/Near mode offset: Far mode places voxels one step further
                    let normal_ivec = IVec3::new(
                        plane.normal.x.round() as i32,
                        plane.normal.y.round() as i32,
                        plane.normal.z.round() as i32,
                    );

                    let voxel_coord = if self.cursor.focus_mode == FocusMode::Far {
                        // Far mode: offset two voxels in the normal direction from nearest corner
                        nearest_corner
                    } else {
                        // Near mode: offset one voxel in the normal direction from nearest corner
                        nearest_corner - normal_ivec
                    };

                    // Update cursor to show where we're drawing
                    self.cursor.coord.pos = voxel_coord;
                    self.cursor.coord.depth = cursor_depth;
                    self.cursor.valid = true;

                    // Place/remove voxel at projected position
                    if shift_held {
                        self.remove_voxel(ctx.gl, voxel_coord, cursor_depth);
                    } else {
                        self.place_voxel(ctx.gl, voxel_coord, cursor_depth);
                    }
                }
            }
        }

        // On mouse release: clear edit plane and drag target
        if left_release {
            self.edit_plane = None;
            self.drag_target_world_pos = None;
        }

        self.prev_left_mouse_pressed = left_mouse_pressed;
    }

    fn render(&mut self, ctx: &FrameContext) {
        let width = ctx.size.0 as i32;
        let height = ctx.size.1 as i32;

        // Clear the framebuffer
        unsafe {
            ctx.gl.viewport(0, 0, width, height);
            ctx.gl.clear_color(0.1, 0.1, 0.15, 1.0);
            ctx.gl
                .clear(glow::COLOR_BUFFER_BIT | glow::DEPTH_BUFFER_BIT);
        }

        // Render skybox first (depth test disabled)
        unsafe {
            self.skybox_renderer
                .render(ctx.gl, &self.camera, width, height);
        }

        // Render the cube mesh at the center
        if let Some(mesh_index) = self.cube_mesh_index {
            unsafe {
                self.mesh_renderer.render_mesh_with_scale(
                    ctx.gl,
                    mesh_index,
                    CUBE_POSITION,
                    Quat::IDENTITY,
                    self.effective_cube_scale(),
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
                self.effective_cube_scale(),
                [1.0, 1.0, 1.0], // White
                &self.camera,
                width,
                height,
            );
        }

        // Render axis gizmo at world center with length 1
        unsafe {
            self.mesh_renderer.render_3d_axis_arrows(
                ctx.gl,
                CUBE_POSITION, // World center
                1.0,           // Length 1
                &self.camera,
                width,
                height,
            );
        }

        // Render edit plane 2D grid when active (during drag)
        if let Some(ref plane) = self.edit_plane {
            let grid_size = 5; // Draw 5x5 voxel grid
            let voxel_size = self.effective_cube_scale() / (1 << plane.depth) as f32;
            let line_thickness = voxel_size * LINE_WIDTH_FACTOR;

            // Use exact drag target world position (ray-plane intersection) for gizmo
            // This is the unsnapped position that follows the mouse precisely
            let target_world_pos = self.drag_target_world_pos.unwrap_or(plane.hit_pos);

            // Snap grid origin to the global voxel grid at the current depth
            // Get nearest voxel corner, convert back to world, then project onto plane
            let nearest_corner = plane.world_to_nearest_voxel_corner(
                target_world_pos,
                CUBE_POSITION,
                self.effective_cube_scale(),
            );
            let corner_world_pos = plane.voxel_corner_to_world(
                nearest_corner,
                CUBE_POSITION,
                self.effective_cube_scale(),
            );

            // Project the corner onto the plane to ensure grid stays on the edit plane
            let offset = corner_world_pos - plane.origin;
            let distance_along_normal = offset.dot(plane.normal);
            let grid_origin = corner_world_pos - plane.normal * distance_along_normal;

            unsafe {
                // Draw grid lines parallel to right axis
                for i in -grid_size..=grid_size {
                    let offset = plane.up * (i as f32 * voxel_size);
                    let start =
                        grid_origin - plane.right * (grid_size as f32 * voxel_size) + offset;
                    let end = grid_origin + plane.right * (grid_size as f32 * voxel_size) + offset;

                    let line_center = (start + end) * 0.5;

                    // Calculate distance-based opacity for gradient fade (fully transparent at distance 3)
                    let dist_from_hit = (line_center - target_world_pos).length();
                    let max_dist = 3.0 * voxel_size;
                    let alpha = (1.0 - (dist_from_hit / max_dist)).clamp(0.0, 1.0);

                    // Skip lines beyond fade distance
                    if alpha < 0.01 {
                        continue;
                    }

                    let line_length = (end - start).length();
                    let line_dir = (end - start).normalize();
                    let rotation = Quat::from_rotation_arc(Vec3::X, line_dir);

                    // Use actual alpha transparency (blue grid)
                    let color = [0.3, 0.6, 1.0, alpha];

                    self.mesh_renderer.render_cubebox_wireframe_colored_alpha(
                        ctx.gl,
                        line_center,
                        rotation,
                        Vec3::new(line_length, line_thickness, line_thickness),
                        1.0,
                        color,
                        &self.camera,
                        width,
                        height,
                    );
                }

                // Draw grid lines parallel to up axis
                for j in -grid_size..=grid_size {
                    let offset = plane.right * (j as f32 * voxel_size);
                    let start = grid_origin - plane.up * (grid_size as f32 * voxel_size) + offset;
                    let end = grid_origin + plane.up * (grid_size as f32 * voxel_size) + offset;

                    let line_center = (start + end) * 0.5;

                    // Calculate distance-based opacity for gradient fade (fully transparent at distance 3)
                    let dist_from_hit = (line_center - target_world_pos).length();
                    let max_dist = 3.0 * voxel_size;
                    let alpha = (1.0 - (dist_from_hit / max_dist)).clamp(0.0, 1.0);

                    // Skip lines beyond fade distance
                    if alpha < 0.01 {
                        continue;
                    }

                    let line_length = (end - start).length();
                    let line_dir = (end - start).normalize();
                    let rotation = Quat::from_rotation_arc(Vec3::X, line_dir);

                    // Use actual alpha transparency (blue grid)
                    let color = [0.3, 0.6, 1.0, alpha];

                    self.mesh_renderer.render_cubebox_wireframe_colored_alpha(
                        ctx.gl,
                        line_center,
                        rotation,
                        Vec3::new(line_length, line_thickness, line_thickness),
                        1.0,
                        color,
                        &self.camera,
                        width,
                        height,
                    );
                }

                // Draw gizmo dot at paint start position (initial hit point)
                self.mesh_renderer.render_cubebox_wireframe_colored(
                    ctx.gl,
                    plane.hit_pos,
                    Quat::IDENTITY,
                    Vec3::ONE,
                    voxel_size * 0.1,
                    [1.0, 1.0, 0.0], // Yellow for start position
                    &self.camera,
                    width,
                    height,
                );

                // Draw short line from start hit point toward camera
                let line_length = voxel_size * 2.0;
                let to_camera = (self.camera.position - plane.hit_pos).normalize();
                let line_end = plane.hit_pos + to_camera * line_length;
                let line_center = (plane.hit_pos + line_end) * 0.5;
                let line_dir = to_camera;
                let rotation = Quat::from_rotation_arc(Vec3::X, line_dir);

                self.mesh_renderer.render_cubebox_wireframe_colored(
                    ctx.gl,
                    line_center,
                    rotation,
                    Vec3::new(line_length, line_thickness * 2.0, line_thickness * 2.0),
                    1.0,
                    [1.0, 1.0, 0.0], // Yellow for start position
                    &self.camera,
                    width,
                    height,
                );

                // Draw gizmo dot at current paint target position (exact ray-plane intersection)
                self.mesh_renderer.render_cubebox_wireframe_colored(
                    ctx.gl,
                    target_world_pos,
                    Quat::IDENTITY,
                    Vec3::ONE,
                    voxel_size * 0.15, // Slightly larger to stand out
                    [0.0, 1.0, 1.0],   // Cyan for target position
                    &self.camera,
                    width,
                    height,
                );

                // Draw short line from target position toward camera
                let to_camera_target = (self.camera.position - target_world_pos).normalize();
                let line_end_target = target_world_pos + to_camera_target * line_length;
                let line_center_target = (target_world_pos + line_end_target) * 0.5;
                let rotation_target = Quat::from_rotation_arc(Vec3::X, to_camera_target);

                self.mesh_renderer.render_cubebox_wireframe_colored(
                    ctx.gl,
                    line_center_target,
                    rotation_target,
                    Vec3::new(line_length, line_thickness * 2.5, line_thickness * 2.5),
                    1.0,
                    [0.0, 1.0, 1.0], // Cyan for target position
                    &self.camera,
                    width,
                    height,
                );
            }
        }

        // Render cursor wireframe when valid
        if self.cursor.valid {
            // Get cursor position and size in world space
            let cursor_center = self
                .cursor
                .world_center(CUBE_POSITION, self.effective_cube_scale());
            let cursor_size = self.cursor.world_size(self.effective_cube_scale());
            // Use magenta color when erase mode is active, otherwise use normal cursor color
            let cursor_color = if self.editor_state.is_erase_mode() {
                [1.0, 0.0, 1.0] // Magenta for erase mode
            } else {
                self.cursor.wireframe_color()
            };

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
                    Vec3::ONE,     // Normalized size (full box)
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
                println!(
                    "[Editor] Frame {}: Capturing to {:?}",
                    ctx.frame, capture.path
                );

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
                if let Err(e) = self
                    .mesh_renderer
                    .save_framebuffer_to_file(ctx.gl, ctx.size.0, ctx.size.1, &path_str)
                {
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

                ui.separator();

                // World scale control
                ui.label("Scale:");
                if ui.button("-").clicked() && self.editor_state.world_scale() > 0 {
                    self.editor_state.decrement_world_scale();
                }
                ui.label(format!("{}", self.editor_state.world_scale()));
                if ui.button("+").clicked() {
                    self.editor_state.increment_world_scale();
                }
                ui.label(format!(
                    "({}x)",
                    self.editor_state.cube_scale_factor() as i32
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
