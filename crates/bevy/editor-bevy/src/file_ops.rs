use bevy::prelude::*;

use crate::file_io::FileState;
use crate::voxel_scene::VoxelScene;

/// System that handles file keyboard shortcuts
/// TODO: Implement proper file I/O with NonSend Cube resource
fn handle_file_shortcuts(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut file_state: ResMut<FileState>,
    mut scene: ResMut<VoxelScene>,
) {
    let ctrl = keyboard.pressed(KeyCode::ControlLeft) || keyboard.pressed(KeyCode::ControlRight);
    let shift = keyboard.pressed(KeyCode::ShiftLeft) || keyboard.pressed(KeyCode::ShiftRight);

    // Ctrl+N: New scene
    if ctrl && keyboard.just_pressed(KeyCode::KeyN) {
        info!("New scene");
        scene.mesh_dirty = true;
        file_state.clear();
    }

    // Ctrl+S: Save
    if ctrl && !shift && keyboard.just_pressed(KeyCode::KeyS) {
        if let Some(path) = &file_state.current_path {
            info!("Saving to: {}", path);
            // TODO: Implement actual save
            file_state.dirty = false;
        } else {
            info!("No file path set, use Save As (Ctrl+Shift+S)");
        }
    }

    // Ctrl+Shift+S: Save As
    if ctrl && shift && keyboard.just_pressed(KeyCode::KeyS) {
        info!("Save As dialog (not implemented)");
        // TODO: Implement file dialog
    }

    // Ctrl+O: Open
    if ctrl && keyboard.just_pressed(KeyCode::KeyO) {
        info!("Open dialog (not implemented)");
        // TODO: Implement file dialog
    }
}

/// Plugin for file operations
pub struct FileOpsPlugin;

impl Plugin for FileOpsPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, handle_file_shortcuts);
    }
}
