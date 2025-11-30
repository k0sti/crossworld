use bevy::prelude::*;
use crate::cursor::EditorState;
use crate::voxel_scene::VoxelScene;

/// System that handles voxel placement via left-click
pub fn handle_voxel_placement(
    mut state: ResMut<EditorState>,
    mut scene: ResMut<VoxelScene>,
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

        // Place voxels based on cursor size
        let size = state.cursor.size as i32;
        let half_size = size / 2;

        let mut voxels_placed = 0;

        for x in 0..size {
            for y in 0..size {
                for z in 0..size {
                    let voxel_pos = cursor_pos + IVec3::new(x - half_size, y - half_size, z - half_size);

                    // Place voxel at micro depth (depth 5)
                    let mut world_cube = scene.world.lock();
                    world_cube.set_voxel_at_depth(
                        voxel_pos.x,
                        voxel_pos.y,
                        voxel_pos.z,
                        5, // micro depth
                        state.selected_material,
                    );
                    voxels_placed += 1;
                }
            }
        }

        if voxels_placed > 0 {
            scene.mesh_dirty = true;
            state.last_paint_position = Some(cursor_pos);
            info!(
                "Placed {} voxels (material {}) at {:?}",
                voxels_placed, state.selected_material, cursor_pos
            );
        }
    }
}

/// System that handles voxel removal via Shift+left-click or Delete key
pub fn handle_voxel_removal(
    state: Res<EditorState>,
    mut scene: ResMut<VoxelScene>,
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

    // Remove voxels based on cursor size
    let size = state.cursor.size as i32;
    let half_size = size / 2;

    let mut voxels_removed = 0;

    for x in 0..size {
        for y in 0..size {
            for z in 0..size {
                let voxel_pos = cursor_pos + IVec3::new(x - half_size, y - half_size, z - half_size);

                // Remove voxel at micro depth (set to air = 0)
                let world_cube = scene.world.lock();
                world_cube.set_voxel_at_depth(
                    voxel_pos.x,
                    voxel_pos.y,
                    voxel_pos.z,
                    5, // micro depth
                    0, // air material
                );
                voxels_removed += 1;
            }
        }
    }

    if voxels_removed > 0 {
        scene.mesh_dirty = true;
        info!("Removed {} voxels at {:?}", voxels_removed, cursor_pos);
    }
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
