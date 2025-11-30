use bevy::prelude::*;
use bevy::input::mouse::MouseWheel;
use crate::raycast::{EditorRaycast, RaycastResult};

/// Focus mode determines where cursor appears relative to raycast hit
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FocusMode {
    /// Cursor at hit position (for removing voxels)
    Near,
    /// Cursor at hit position + face normal (for placing voxels)
    Far,
}

impl FocusMode {
    pub fn toggle(&self) -> Self {
        match self {
            FocusMode::Near => FocusMode::Far,
            FocusMode::Far => FocusMode::Near,
        }
    }
}

/// 3D cursor for voxel editing
#[derive(Debug, Clone)]
pub struct CubeCursor {
    /// Current position in world space
    pub position: Vec3,
    /// Size of cursor (1-16 voxels)
    pub size: u32,
    /// Whether cursor is currently valid (raycast hit something)
    pub valid: bool,
}

impl Default for CubeCursor {
    fn default() -> Self {
        Self {
            position: Vec3::ZERO,
            size: 1,
            valid: false,
        }
    }
}

impl CubeCursor {
    /// Increase cursor size (max 16)
    pub fn increase_size(&mut self) {
        self.size = (self.size + 1).min(16);
    }

    /// Decrease cursor size (min 1)
    pub fn decrease_size(&mut self) {
        self.size = (self.size - 1).max(1);
    }

    /// Get cursor bounds as min and max voxel coordinates
    pub fn bounds(&self) -> (IVec3, IVec3) {
        let half_size = (self.size / 2) as i32;
        let center = IVec3::new(
            self.position.x.floor() as i32,
            self.position.y.floor() as i32,
            self.position.z.floor() as i32,
        );

        let min = center - IVec3::splat(half_size);
        let max = center + IVec3::splat(half_size);

        (min, max)
    }
}

/// Resource holding editor state including cursor and focus mode
#[derive(Resource)]
pub struct EditorState {
    pub cursor: CubeCursor,
    pub focus_mode: FocusMode,
}

impl Default for EditorState {
    fn default() -> Self {
        Self {
            cursor: CubeCursor::default(),
            focus_mode: FocusMode::Far, // Start in Far mode (for placing)
        }
    }
}

/// System that updates cursor position based on raycast
pub fn update_cursor(
    mut state: ResMut<EditorState>,
    raycast: Res<EditorRaycast>,
) {
    if let Some(result) = &raycast.result {
        state.cursor.valid = true;
        state.cursor.position = calculate_cursor_position(result, state.focus_mode);
    } else {
        state.cursor.valid = false;
    }
}

/// Calculate cursor position based on raycast result and focus mode
fn calculate_cursor_position(result: &RaycastResult, focus_mode: FocusMode) -> Vec3 {
    match focus_mode {
        FocusMode::Near => {
            // Cursor at hit position (for removing voxels)
            Vec3::new(
                result.voxel_coord.x as f32,
                result.voxel_coord.y as f32,
                result.voxel_coord.z as f32,
            )
        }
        FocusMode::Far => {
            // Cursor at hit position + face normal (for placing voxels)
            let placement_pos = result.hit_position + result.face_normal;
            Vec3::new(
                placement_pos.x.floor(),
                placement_pos.y.floor(),
                placement_pos.z.floor(),
            )
        }
    }
}

/// System that handles cursor keyboard controls
pub fn cursor_keyboard_controls(
    mut state: ResMut<EditorState>,
    keyboard: Res<ButtonInput<KeyCode>>,
) {
    // Tab: Toggle focus mode
    if keyboard.just_pressed(KeyCode::Tab) {
        state.focus_mode = state.focus_mode.toggle();
        info!("Focus mode: {:?}", state.focus_mode);
    }

    // [ key: Decrease cursor size
    if keyboard.just_pressed(KeyCode::BracketLeft) {
        state.cursor.decrease_size();
        info!("Cursor size: {}", state.cursor.size);
    }

    // ] key: Increase cursor size
    if keyboard.just_pressed(KeyCode::BracketRight) {
        state.cursor.increase_size();
        info!("Cursor size: {}", state.cursor.size);
    }
}

/// System that handles cursor size adjustment via scroll wheel (with Shift)
pub fn cursor_scroll_controls(
    mut state: ResMut<EditorState>,
    mut scroll_events: MessageReader<MouseWheel>,
    keyboard: Res<ButtonInput<KeyCode>>,
) {
    // Only adjust size if Shift is held
    if !keyboard.pressed(KeyCode::ShiftLeft) && !keyboard.pressed(KeyCode::ShiftRight) {
        return;
    }

    let scroll_delta: f32 = scroll_events.read().map(|e| e.y).sum();

    if scroll_delta > 0.1 {
        state.cursor.increase_size();
        info!("Cursor size: {}", state.cursor.size);
    } else if scroll_delta < -0.1 {
        state.cursor.decrease_size();
        info!("Cursor size: {}", state.cursor.size);
    }
}

/// System that draws cursor visualization using Bevy Gizmos
pub fn draw_cursor_gizmo(
    state: Res<EditorState>,
    mut gizmos: Gizmos,
) {
    if !state.cursor.valid {
        return;
    }

    // Color based on focus mode
    let color = match state.focus_mode {
        FocusMode::Near => Color::srgb(1.0, 0.3, 0.3), // Red for removal
        FocusMode::Far => Color::srgb(0.3, 1.0, 0.3),  // Green for placement
    };

    // Draw wireframe cube at cursor position
    let size = state.cursor.size as f32;
    let half_size = size / 2.0;
    let center = state.cursor.position + Vec3::splat(half_size);

    // Draw cube wireframe
    gizmos.cuboid(
        Transform::from_translation(center).with_scale(Vec3::splat(size)),
        color,
    );
}

/// Plugin for cursor system
pub struct CursorPlugin;

impl Plugin for CursorPlugin {
    fn build(&self, app: &mut App) {
        app
            .init_resource::<EditorState>()
            .add_systems(Update, (
                update_cursor,
                cursor_keyboard_controls,
                cursor_scroll_controls,
                draw_cursor_gizmo,
            ).chain());
    }
}
