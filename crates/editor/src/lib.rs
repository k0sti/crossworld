//! Voxel cube editor using the app framework
//!
//! A native OpenGL voxel editor using glow/egui/winit.

pub mod cursor;
pub mod editing;
pub mod palette;
pub mod raycast;
pub mod ui;

use app::{App, FrameContext, InputState};
use cube::Cube;
use glam::{IVec3, Quat, Vec2, Vec3};
use glow::HasContext;
use renderer::{Camera, MeshRenderer, OrbitController, OrbitControllerConfig};
use std::rc::Rc;
use winit::keyboard::KeyCode;

use crate::cursor::{CubeCursor, FocusMode};
use crate::editing::EditorState;
use crate::palette::{ColorPalette, ModelPalette};
use crate::raycast::{raycast_from_mouse, EditorHit};

/// Constants for the edited cube
const CUBE_POSITION: Vec3 = Vec3::ZERO;
const CUBE_SCALE: f32 = 2.0;
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
    prev_left_mouse_pressed: bool,

    // Palette state
    color_palette: ColorPalette,
    model_palette: ModelPalette,
}

impl Default for EditorApp {
    fn default() -> Self {
        Self::new()
    }
}

impl EditorApp {
    /// Create a new editor application
    pub fn new() -> Self {
        // Camera target at origin (where voxel editing typically happens)
        let camera_target = Vec3::ZERO;
        // Camera positioned above and to the side, looking at the editing area
        let camera_position = Vec3::new(5.0, 4.0, 5.0);

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
            prev_left_mouse_pressed: false,
            color_palette: ColorPalette::new(),
            model_palette: ModelPalette::new(),
        }
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
        self.cube = Some(new_cube.clone());

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
        self.cube = Some(new_cube.clone());

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
}

impl App for EditorApp {
    fn init(&mut self, ctx: &FrameContext) {
        println!("[Editor] Initializing voxel editor");

        // Initialize mesh renderer
        if let Err(e) = unsafe { self.mesh_renderer.init_gl(ctx.gl) } {
            eprintln!("[Editor] Failed to initialize mesh renderer: {}", e);
            return;
        }

        // Create initial cube - solid with a colorful material (material index 156 = green-ish)
        let cube = Rc::new(Cube::Solid(156u8));
        self.cube = Some(cube.clone());

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
        // Handle camera orbit with right-mouse drag
        if input.is_right_mouse_pressed() {
            let yaw_delta = -input.mouse_delta.x * self.orbit_controller.config.mouse_sensitivity;
            let pitch_delta = -input.mouse_delta.y * self.orbit_controller.config.mouse_sensitivity;
            self.orbit_controller.rotate(yaw_delta, pitch_delta, &mut self.camera);
        }

        // Handle camera zoom with scroll wheel
        if input.scroll_delta.y.abs() > 0.01 {
            self.orbit_controller.zoom(input.scroll_delta.y, &mut self.camera);
        }

        // Handle Tab key to toggle Near/Far focus mode
        let tab_pressed = input.is_key_pressed(KeyCode::Tab);
        if tab_pressed && !self.prev_tab_pressed {
            self.cursor.toggle_mode();
        }
        self.prev_tab_pressed = tab_pressed;

        // Update cursor from mouse raycast (only if mouse position is available)
        if let (Some(ref cube), Some(mouse_pos)) = (&self.cube, input.mouse_pos) {
            let screen_size = Vec2::new(ctx.size.0 as f32, ctx.size.1 as f32);
            if let Some(hit) = raycast_from_mouse(
                mouse_pos,
                screen_size,
                &self.camera,
                cube,
                CUBE_POSITION,
                CUBE_SCALE,
                Some(EDIT_DEPTH),
            ) {
                self.cursor.update_from_raycast(
                    hit.world_pos,
                    hit.normal_vec3(),
                    hit.voxel_coord,
                );
                self.last_hit = Some(hit);
            } else {
                self.cursor.invalidate();
                self.last_hit = None;
            }
        }

        // Handle left-click for voxel placement/removal
        let left_mouse_pressed = input.is_left_mouse_pressed();
        let left_click = left_mouse_pressed && !self.prev_left_mouse_pressed;
        self.prev_left_mouse_pressed = left_mouse_pressed;

        // Check if Shift is held (for removal in Near mode)
        let shift_held = input.is_key_pressed(KeyCode::ShiftLeft)
            || input.is_key_pressed(KeyCode::ShiftRight);

        if left_click {
            if let Some(ref hit) = self.last_hit {
                if shift_held && self.cursor.focus_mode == FocusMode::Near {
                    // Shift+left-click in Near mode: remove the hit voxel
                    let removal_pos = hit.voxel_at_depth(EDIT_DEPTH, CUBE_POSITION, CUBE_SCALE);
                    self.remove_voxel(ctx.gl, removal_pos, EDIT_DEPTH);
                } else if self.cursor.focus_mode == FocusMode::Far {
                    // Left-click in Far mode: place voxel adjacent to hit face
                    let placement_pos = hit.placement_at_depth(EDIT_DEPTH, CUBE_POSITION, CUBE_SCALE);
                    self.place_voxel(ctx.gl, placement_pos, EDIT_DEPTH);
                }
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

        // Render cursor wireframe when valid
        if self.cursor.valid {
            // Calculate cursor wireframe position and size
            // The cursor position is the corner of the voxel, we need to center the wireframe
            let cursor_size = self.cursor.render_size();
            let cursor_center = self.cursor.position + cursor_size * 0.5;
            let cursor_color = self.cursor.wireframe_color();

            unsafe {
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
    }

    fn ui(&mut self, ctx: &FrameContext, egui_ctx: &egui::Context) {
        // Top panel with title
        egui::TopBottomPanel::top("editor_top_panel").show(egui_ctx, |ui| {
            ui.horizontal(|ui| {
                ui.heading("Crossworld Voxel Editor");
                ui.separator();
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

        // Color palette panel on the right side
        ui::show_color_palette_panel(egui_ctx, &mut self.color_palette, &mut self.editor_state);

        // Model palette panel on the left side
        ui::show_model_palette_panel(egui_ctx, &mut self.model_palette);
    }
}

/// Export the create_app function for dynamic loading (hot-reload support)
#[no_mangle]
#[allow(improper_ctypes_definitions)]
pub extern "C" fn create_app() -> *mut dyn App {
    Box::into_raw(Box::new(EditorApp::new()))
}
