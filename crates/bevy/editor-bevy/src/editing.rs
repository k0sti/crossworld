use crate::config::EditorConfig;
use crate::cursor::EditorState;
use crate::voxel_scene::VoxelScene;
use bevy::prelude::*;

/// System that handles voxel placement via left-click
/// TODO: Implement proper voxel editing with NonSend Cube resource
pub fn handle_voxel_placement(
    mut state: ResMut<EditorState>,
    mut scene: ResMut<VoxelScene>,
    _config: Res<EditorConfig>,
    mouse_buttons: Res<ButtonInput<MouseButton>>,
    keyboard: Res<ButtonInput<KeyCode>>,
) {
    // Only place voxels if not holding Shift (which is for removal)
    if keyboard.pressed(KeyCode::ShiftLeft) || keyboard.pressed(KeyCode::ShiftRight) {
        return;
    }

    // Track continuous paint state
    if mouse_buttons.just_pressed(MouseButton::Left) {
        state.continuous_paint = true;
        state.last_paint_position = None;
    }

    if mouse_buttons.just_released(MouseButton::Left) {
        state.continuous_paint = false;
        state.last_paint_position = None;
    }

    // Place voxels if left mouse is pressed and cursor is valid
    if (mouse_buttons.just_pressed(MouseButton::Left) || state.continuous_paint)
        && state.cursor.valid
    {
        let cursor_pos = IVec3::new(
            state.cursor.position.x.floor() as i32,
            state.cursor.position.y.floor() as i32,
            state.cursor.position.z.floor() as i32,
        );

        // Check if we've already painted this position (for continuous paint)
        if state.continuous_paint {
            if let Some(last_pos) = state.last_paint_position {
                if last_pos == cursor_pos {
                    return; // Already painted here
                }
            }
        }

        // Mark for mesh update (actual cube update TODO)
        scene.mesh_dirty = true;
        state.last_paint_position = Some(cursor_pos);
        info!(
            "Placed voxel (material {}) at {:?}",
            state.selected_material, cursor_pos
        );
    }
}

/// System that handles voxel removal via Shift+left-click or Delete key
/// TODO: Implement proper voxel removal with NonSend Cube resource
pub fn handle_voxel_removal(
    state: Res<EditorState>,
    mut scene: ResMut<VoxelScene>,
    _config: Res<EditorConfig>,
    mouse_buttons: Res<ButtonInput<MouseButton>>,
    keyboard: Res<ButtonInput<KeyCode>>,
) {
    // Remove voxels on Shift+left-click or Delete key
    let should_remove = (mouse_buttons.just_pressed(MouseButton::Left)
        && (keyboard.pressed(KeyCode::ShiftLeft) || keyboard.pressed(KeyCode::ShiftRight)))
        || keyboard.just_pressed(KeyCode::Delete);

    if !should_remove || !state.cursor.valid {
        return;
    }

    let cursor_pos = IVec3::new(
        state.cursor.position.x.floor() as i32,
        state.cursor.position.y.floor() as i32,
        state.cursor.position.z.floor() as i32,
    );

    // Mark for mesh update (actual cube update TODO)
    scene.mesh_dirty = true;
    info!("Removed voxel at {:?}", cursor_pos);
}

/// System that handles material selection via number keys 1-9
pub fn handle_material_selection(
    mut state: ResMut<EditorState>,
    keyboard: Res<ButtonInput<KeyCode>>,
) {
    // Material selection: 1-9 keys select materials 1-9
    if keyboard.just_pressed(KeyCode::Digit1) {
        state.selected_material = 1;
        info!("Selected material: {}", state.selected_material);
    } else if keyboard.just_pressed(KeyCode::Digit2) {
        state.selected_material = 2;
        info!("Selected material: {}", state.selected_material);
    } else if keyboard.just_pressed(KeyCode::Digit3) {
        state.selected_material = 3;
        info!("Selected material: {}", state.selected_material);
    } else if keyboard.just_pressed(KeyCode::Digit4) {
        state.selected_material = 4;
        info!("Selected material: {}", state.selected_material);
    } else if keyboard.just_pressed(KeyCode::Digit5) {
        state.selected_material = 5;
        info!("Selected material: {}", state.selected_material);
    } else if keyboard.just_pressed(KeyCode::Digit6) {
        state.selected_material = 6;
        info!("Selected material: {}", state.selected_material);
    } else if keyboard.just_pressed(KeyCode::Digit7) {
        state.selected_material = 7;
        info!("Selected material: {}", state.selected_material);
    } else if keyboard.just_pressed(KeyCode::Digit8) {
        state.selected_material = 8;
        info!("Selected material: {}", state.selected_material);
    } else if keyboard.just_pressed(KeyCode::Digit9) {
        state.selected_material = 9;
        info!("Selected material: {}", state.selected_material);
    } else if keyboard.just_pressed(KeyCode::Digit0) {
        state.selected_material = 0; // Air (removal)
        info!("Selected material: {} (air)", state.selected_material);
    }
}

/// Plugin for voxel editing systems
pub struct EditingPlugin;

impl Plugin for EditingPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (
                handle_material_selection,
                handle_voxel_placement,
                handle_voxel_removal,
            )
                .chain(),
        );
    }
}
