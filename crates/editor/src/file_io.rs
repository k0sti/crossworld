use bevy::prelude::*;
use rfd::FileDialog;
use std::path::PathBuf;

/// Resource tracking the current file path and modification state
#[derive(Resource)]
pub struct FileState {
    /// Current file path (None if never saved)
    pub current_file: Option<PathBuf>,
    /// Whether the scene has unsaved changes
    pub dirty: bool,
}

impl Default for FileState {
    fn default() -> Self {
        Self {
            current_file: None,
            dirty: false,
        }
    }
}

impl FileState {
    pub fn mark_dirty(&mut self) {
        self.dirty = true;
    }

    pub fn mark_clean(&mut self) {
        self.dirty = false;
    }

    pub fn set_file(&mut self, path: PathBuf) {
        self.current_file = Some(path);
    }

    pub fn get_filename(&self) -> String {
        self.current_file
            .as_ref()
            .and_then(|p| p.file_name())
            .and_then(|n| n.to_str())
            .map(|s| s.to_string())
            .unwrap_or_else(|| "Untitled".to_string())
    }
}

/// File operation commands that can be triggered
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileOperation {
    New,
    Open,
    Save,
    SaveAs,
}

/// Message for file operations
#[derive(Message)]
pub struct FileOperationEvent(pub FileOperation);

/// Show a file save dialog and return the selected path
pub fn show_save_dialog() -> Option<PathBuf> {
    FileDialog::new()
        .add_filter("CSM Voxel File", &["csm"])
        .add_filter("All Files", &["*"])
        .set_title("Save CSM File")
        .save_file()
}

/// Show a file open dialog and return the selected path
pub fn show_open_dialog() -> Option<PathBuf> {
    FileDialog::new()
        .add_filter("CSM Voxel File", &["csm"])
        .add_filter("All Files", &["*"])
        .set_title("Open CSM File")
        .pick_file()
}

/// Show a confirmation dialog for unsaved changes
pub fn confirm_discard_changes() -> bool {
    // TODO: Implement proper modal dialog with egui
    // For now, always confirm (return true)
    warn!("Discarding unsaved changes (no confirmation dialog yet)");
    true
}

/// Plugin for file I/O systems
pub struct FileIoPlugin;

impl Plugin for FileIoPlugin {
    fn build(&self, app: &mut App) {
        app
            .init_resource::<FileState>()
            .add_message::<FileOperationEvent>();
    }
}
