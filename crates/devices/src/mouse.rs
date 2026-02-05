//! Mouse and cursor input types
//!
//! This module provides types for mouse button state and cursor mode management.

/// Mouse button state flags
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct MouseButtons {
    /// Left mouse button pressed
    pub left: bool,
    /// Right mouse button pressed
    pub right: bool,
    /// Middle mouse button (scroll wheel click) pressed
    pub middle: bool,
}

impl MouseButtons {
    /// Create a new MouseButtons with all buttons released
    pub fn new() -> Self {
        Self::default()
    }

    /// Check if any button is pressed
    pub fn any_pressed(&self) -> bool {
        self.left || self.right || self.middle
    }

    /// Reset all buttons to released state
    pub fn reset(&mut self) {
        self.left = false;
        self.right = false;
        self.middle = false;
    }
}

/// Mouse button type for event handling and injection
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MouseButtonType {
    /// Left mouse button (primary)
    Left,
    /// Right mouse button (secondary/context)
    Right,
    /// Middle mouse button (scroll wheel click)
    Middle,
}

/// Cursor mode for the application
///
/// Controls how the cursor behaves in the application window.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CursorMode {
    /// Visible, free movement (default)
    ///
    /// Normal cursor behavior - visible and can move freely.
    #[default]
    Normal,
    /// Hidden but not grabbed
    ///
    /// Cursor is invisible but can still move and leave the window.
    Hidden,
    /// Hidden and confined/locked (for FPS camera)
    ///
    /// Cursor is invisible and locked to the center of the window.
    /// Used for first-person camera controls.
    Grabbed,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mouse_buttons_default() {
        let buttons = MouseButtons::default();
        assert!(!buttons.left);
        assert!(!buttons.right);
        assert!(!buttons.middle);
        assert!(!buttons.any_pressed());
    }

    #[test]
    fn test_mouse_buttons_any_pressed() {
        let mut buttons = MouseButtons::new();
        assert!(!buttons.any_pressed());

        buttons.left = true;
        assert!(buttons.any_pressed());

        buttons.reset();
        assert!(!buttons.any_pressed());
    }

    #[test]
    fn test_cursor_mode_default() {
        let mode = CursorMode::default();
        assert_eq!(mode, CursorMode::Normal);
    }
}
