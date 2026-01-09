//! Editor state for voxel editing operations
//!
//! Manages the current editing state including selected material,
//! paint modes, and editing history.

/// Default material index (green-ish color in palette)
pub const DEFAULT_MATERIAL: u8 = 156;

/// Editor state for voxel editing
#[derive(Debug, Clone)]
pub struct EditorState {
    /// Currently selected material/color index (0-255)
    pub selected_material: u8,
    /// Whether continuous painting is enabled (paint while dragging)
    pub continuous_paint: bool,
}

impl Default for EditorState {
    fn default() -> Self {
        Self {
            selected_material: DEFAULT_MATERIAL,
            continuous_paint: false,
        }
    }
}

impl EditorState {
    /// Create a new editor state with default values
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the selected material index
    pub fn set_material(&mut self, material: u8) {
        self.selected_material = material;
    }

    /// Toggle continuous paint mode
    pub fn toggle_continuous_paint(&mut self) {
        self.continuous_paint = !self.continuous_paint;
    }

    /// Enable continuous paint mode
    pub fn enable_continuous_paint(&mut self) {
        self.continuous_paint = true;
    }

    /// Disable continuous paint mode
    pub fn disable_continuous_paint(&mut self) {
        self.continuous_paint = false;
    }

    /// Check if continuous paint mode is enabled
    pub fn is_continuous_paint(&self) -> bool {
        self.continuous_paint
    }

    /// Get the currently selected material
    pub fn material(&self) -> u8 {
        self.selected_material
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_editor_state_default() {
        let state = EditorState::default();
        assert_eq!(state.selected_material, DEFAULT_MATERIAL);
        assert!(!state.continuous_paint);
    }

    #[test]
    fn test_editor_state_new() {
        let state = EditorState::new();
        assert_eq!(state.selected_material, DEFAULT_MATERIAL);
        assert!(!state.continuous_paint);
    }

    #[test]
    fn test_set_material() {
        let mut state = EditorState::new();
        state.set_material(42);
        assert_eq!(state.material(), 42);
    }

    #[test]
    fn test_toggle_continuous_paint() {
        let mut state = EditorState::new();
        assert!(!state.is_continuous_paint());

        state.toggle_continuous_paint();
        assert!(state.is_continuous_paint());

        state.toggle_continuous_paint();
        assert!(!state.is_continuous_paint());
    }

    #[test]
    fn test_enable_disable_continuous_paint() {
        let mut state = EditorState::new();

        state.enable_continuous_paint();
        assert!(state.is_continuous_paint());

        state.disable_continuous_paint();
        assert!(!state.is_continuous_paint());
    }
}
