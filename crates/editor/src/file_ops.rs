use bevy::prelude::*;
use crate::file_io::{FileState, FileOperation, FileOperationEvent, show_save_dialog, show_open_dialog, confirm_discard_changes};
use crate::voxel_scene::VoxelScene;
use std::fs;

/// System that handles file operation messages
pub fn handle_file_operations(
    mut events: MessageReader<FileOperationEvent>,
    mut file_state: ResMut<FileState>,
    mut scene: ResMut<VoxelScene>,
) {
    for event in events.read() {
        match event.0 {
            FileOperation::New => handle_new_scene(&mut file_state, &mut scene),
            FileOperation::Open => handle_open_file(&mut file_state, &mut scene),
            FileOperation::Save => handle_save_file(&mut file_state, &scene),
            FileOperation::SaveAs => handle_save_as_file(&mut file_state, &scene),
        }
    }
}

/// Create a new scene (with confirmation if dirty)
fn handle_new_scene(file_state: &mut FileState, scene: &mut VoxelScene) {
    // Check if we have unsaved changes
    if file_state.dirty && !confirm_discard_changes() {
        return;
    }

    info!("Creating new scene");

    // Reset the scene by creating a new WorldCube
    // Note: VoxelScene::default() creates a new WorldCube with default parameters
    *scene = VoxelScene::default();

    file_state.current_file = None;
    file_state.mark_clean();

    info!("New scene created successfully");
}

/// Open a CSM file
fn handle_open_file(file_state: &mut FileState, scene: &mut VoxelScene) {
    // Check if we have unsaved changes
    if file_state.dirty && !confirm_discard_changes() {
        return;
    }

    // Show file dialog
    let Some(path) = show_open_dialog() else {
        info!("Open dialog cancelled");
        return;
    };

    info!("Opening file: {:?}", path);

    // Read file contents
    let csm_code = match fs::read_to_string(&path) {
        Ok(content) => content,
        Err(e) => {
            error!("Failed to read file {:?}: {}", path, e);
            // TODO: Show error dialog to user
            return;
        }
    };

    // Parse and load CSM into scene
    {
        let world = scene.world.lock();
        if let Err(e) = world.set_root(&csm_code) {
            error!("Failed to parse CSM: {:?}", e);
            // TODO: Show error dialog to user
            return;
        }
    }

    // Mark mesh as dirty to trigger regeneration
    scene.mesh_dirty = true;

    // Update file state
    file_state.set_file(path.clone());
    file_state.mark_clean();

    info!("File loaded successfully: {:?}", path);
}

/// Save to current file (or show save-as dialog if no file)
fn handle_save_file(file_state: &mut FileState, scene: &VoxelScene) {
    if let Some(path) = &file_state.current_file {
        // Save to existing file
        save_to_path(path.clone(), file_state, scene);
    } else {
        // No current file, show save-as dialog
        handle_save_as_file(file_state, scene);
    }
}

/// Save to a new file (always show dialog)
fn handle_save_as_file(file_state: &mut FileState, scene: &VoxelScene) {
    // Show file dialog
    let Some(path) = show_save_dialog() else {
        info!("Save dialog cancelled");
        return;
    };

    save_to_path(path, file_state, scene);
}

/// Save scene to the specified path
fn save_to_path(path: std::path::PathBuf, file_state: &mut FileState, scene: &VoxelScene) {
    info!("Saving to file: {:?}", path);

    // Export scene to CSM format
    let csm_code = {
        let world = scene.world.lock();
        world.export_to_csm()
    };

    // Write to file
    if let Err(e) = fs::write(&path, csm_code) {
        error!("Failed to write file {:?}: {}", path, e);
        // TODO: Show error dialog to user
        return;
    }

    // Update file state
    file_state.set_file(path.clone());
    file_state.mark_clean();

    info!("File saved successfully: {:?}", path);
}

/// System that handles keyboard shortcuts for file operations
pub fn file_shortcuts_system(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut events: MessageWriter<FileOperationEvent>,
) {
    // Ctrl+N: New file
    if keyboard.pressed(KeyCode::ControlLeft) || keyboard.pressed(KeyCode::ControlRight) {
        if keyboard.just_pressed(KeyCode::KeyN) {
            events.write(FileOperationEvent(FileOperation::New));
        }
        // Ctrl+O: Open file
        else if keyboard.just_pressed(KeyCode::KeyO) {
            events.write(FileOperationEvent(FileOperation::Open));
        }
        // Ctrl+S: Save file
        else if keyboard.just_pressed(KeyCode::KeyS) {
            // Ctrl+Shift+S: Save As
            if keyboard.pressed(KeyCode::ShiftLeft) || keyboard.pressed(KeyCode::ShiftRight) {
                events.write(FileOperationEvent(FileOperation::SaveAs));
            } else {
                events.write(FileOperationEvent(FileOperation::Save));
            }
        }
    }
}

/// System that marks file as dirty when scene is modified
pub fn track_scene_changes(
    scene: Res<VoxelScene>,
    mut file_state: ResMut<FileState>,
) {
    // If mesh is dirty, it means the scene was modified
    if scene.mesh_dirty && !file_state.dirty {
        file_state.mark_dirty();
    }
}

/// Plugin for file operations
pub struct FileOpsPlugin;

impl Plugin for FileOpsPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (
                file_shortcuts_system,
                handle_file_operations,
                track_scene_changes,
            )
                .chain(),
        );
    }
}
