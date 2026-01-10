use bevy::prelude::*;

/// Resource tracking file state
#[derive(Resource, Default)]
pub struct FileState {
    /// Current file path (None if unsaved)
    pub current_path: Option<String>,
    /// Whether there are unsaved changes
    pub dirty: bool,
}

#[allow(dead_code)]
impl FileState {
    /// Get display filename
    pub fn get_filename(&self) -> &str {
        self.current_path
            .as_ref()
            .and_then(|p| p.rsplit('/').next())
            .unwrap_or("Untitled")
    }

    /// Mark as having unsaved changes
    pub fn mark_dirty(&mut self) {
        self.dirty = true;
    }

    /// Mark as saved
    pub fn mark_saved(&mut self, path: String) {
        self.current_path = Some(path);
        self.dirty = false;
    }

    /// Clear state for new file
    pub fn clear(&mut self) {
        self.current_path = None;
        self.dirty = false;
    }
}

/// Plugin for file I/O state
pub struct FileIoPlugin;

impl Plugin for FileIoPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<FileState>();
    }
}
