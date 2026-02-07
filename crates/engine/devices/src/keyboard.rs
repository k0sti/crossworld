//! Keyboard input abstraction
//!
//! This module provides platform-agnostic keyboard input types.
//! The actual key codes are intentionally kept simple and platform-independent.

use std::collections::HashSet;

/// Key state for a single key
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct KeyState {
    /// Whether the key is currently pressed
    pub pressed: bool,
    /// Whether the key was just pressed this frame
    pub just_pressed: bool,
    /// Whether the key was just released this frame
    pub just_released: bool,
}

impl KeyState {
    /// Create a new unpressed key state
    pub fn new() -> Self {
        Self::default()
    }

    /// Update the key state for a new frame
    ///
    /// Clears the "just_pressed" and "just_released" flags.
    pub fn update(&mut self) {
        self.just_pressed = false;
        self.just_released = false;
    }

    /// Record a key press event
    pub fn press(&mut self) {
        if !self.pressed {
            self.just_pressed = true;
        }
        self.pressed = true;
    }

    /// Record a key release event
    pub fn release(&mut self) {
        if self.pressed {
            self.just_released = true;
        }
        self.pressed = false;
    }
}

/// Platform-independent key codes
///
/// This enum provides a minimal set of common keys that are available
/// across all platforms. Platform-specific backends should map their
/// native key codes to these values.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum Key {
    // Letters
    A,
    B,
    C,
    D,
    E,
    F,
    G,
    H,
    I,
    J,
    K,
    L,
    M,
    N,
    O,
    P,
    Q,
    R,
    S,
    T,
    U,
    V,
    W,
    X,
    Y,
    Z,

    // Numbers
    Digit0,
    Digit1,
    Digit2,
    Digit3,
    Digit4,
    Digit5,
    Digit6,
    Digit7,
    Digit8,
    Digit9,

    // Function keys
    F1,
    F2,
    F3,
    F4,
    F5,
    F6,
    F7,
    F8,
    F9,
    F10,
    F11,
    F12,

    // Navigation
    Up,
    Down,
    Left,
    Right,
    Home,
    End,
    PageUp,
    PageDown,

    // Editing
    Backspace,
    Delete,
    Insert,
    Enter,
    Tab,

    // Modifiers
    ShiftLeft,
    ShiftRight,
    ControlLeft,
    ControlRight,
    AltLeft,
    AltRight,
    SuperLeft,
    SuperRight,

    // Special
    Escape,
    Space,
    CapsLock,

    // Numpad
    Numpad0,
    Numpad1,
    Numpad2,
    Numpad3,
    Numpad4,
    Numpad5,
    Numpad6,
    Numpad7,
    Numpad8,
    Numpad9,
    NumpadAdd,
    NumpadSubtract,
    NumpadMultiply,
    NumpadDivide,
    NumpadEnter,
    NumpadDecimal,

    // Punctuation (common)
    Minus,
    Equal,
    BracketLeft,
    BracketRight,
    Backslash,
    Semicolon,
    Quote,
    Comma,
    Period,
    Slash,
    Grave,
}

/// Keyboard state tracker
///
/// Tracks the state of all keys and provides convenient query methods.
#[derive(Debug, Clone, Default)]
pub struct KeyboardState {
    /// Set of currently pressed keys
    pressed: HashSet<Key>,
    /// Keys that were just pressed this frame
    just_pressed: HashSet<Key>,
    /// Keys that were just released this frame
    just_released: HashSet<Key>,
}

impl KeyboardState {
    /// Create a new keyboard state with no keys pressed
    pub fn new() -> Self {
        Self::default()
    }

    /// Update the keyboard state for a new frame
    ///
    /// Clears the "just_pressed" and "just_released" sets.
    pub fn update(&mut self) {
        self.just_pressed.clear();
        self.just_released.clear();
    }

    /// Record a key press event
    pub fn press(&mut self, key: Key) {
        if !self.pressed.contains(&key) {
            self.just_pressed.insert(key);
        }
        self.pressed.insert(key);
    }

    /// Record a key release event
    pub fn release(&mut self, key: Key) {
        if self.pressed.contains(&key) {
            self.just_released.insert(key);
        }
        self.pressed.remove(&key);
    }

    /// Check if a key is currently pressed
    pub fn is_pressed(&self, key: Key) -> bool {
        self.pressed.contains(&key)
    }

    /// Check if a key was just pressed this frame
    pub fn is_just_pressed(&self, key: Key) -> bool {
        self.just_pressed.contains(&key)
    }

    /// Check if a key was just released this frame
    pub fn is_just_released(&self, key: Key) -> bool {
        self.just_released.contains(&key)
    }

    /// Check if shift is pressed (either left or right)
    pub fn is_shift_pressed(&self) -> bool {
        self.is_pressed(Key::ShiftLeft) || self.is_pressed(Key::ShiftRight)
    }

    /// Check if control is pressed (either left or right)
    pub fn is_ctrl_pressed(&self) -> bool {
        self.is_pressed(Key::ControlLeft) || self.is_pressed(Key::ControlRight)
    }

    /// Check if alt is pressed (either left or right)
    pub fn is_alt_pressed(&self) -> bool {
        self.is_pressed(Key::AltLeft) || self.is_pressed(Key::AltRight)
    }

    /// Check if super/meta/command is pressed (either left or right)
    pub fn is_super_pressed(&self) -> bool {
        self.is_pressed(Key::SuperLeft) || self.is_pressed(Key::SuperRight)
    }

    /// Get the set of currently pressed keys
    pub fn pressed_keys(&self) -> &HashSet<Key> {
        &self.pressed
    }

    /// Get the number of currently pressed keys
    pub fn pressed_count(&self) -> usize {
        self.pressed.len()
    }

    /// Check if any key is currently pressed
    pub fn any_pressed(&self) -> bool {
        !self.pressed.is_empty()
    }

    /// Clear all key states
    pub fn clear(&mut self) {
        self.pressed.clear();
        self.just_pressed.clear();
        self.just_released.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_key_state_press_release() {
        let mut state = KeyState::new();
        assert!(!state.pressed);
        assert!(!state.just_pressed);

        state.press();
        assert!(state.pressed);
        assert!(state.just_pressed);

        state.update();
        assert!(state.pressed);
        assert!(!state.just_pressed);

        state.release();
        assert!(!state.pressed);
        assert!(state.just_released);

        state.update();
        assert!(!state.pressed);
        assert!(!state.just_released);
    }

    #[test]
    fn test_keyboard_state() {
        let mut kb = KeyboardState::new();
        assert!(!kb.any_pressed());

        kb.press(Key::W);
        kb.press(Key::ShiftLeft);

        assert!(kb.is_pressed(Key::W));
        assert!(kb.is_just_pressed(Key::W));
        assert!(kb.is_shift_pressed());
        assert_eq!(kb.pressed_count(), 2);

        kb.update();
        assert!(kb.is_pressed(Key::W));
        assert!(!kb.is_just_pressed(Key::W));

        kb.release(Key::W);
        assert!(!kb.is_pressed(Key::W));
        assert!(kb.is_just_released(Key::W));
    }

    #[test]
    fn test_modifier_detection() {
        let mut kb = KeyboardState::new();

        kb.press(Key::ControlRight);
        assert!(kb.is_ctrl_pressed());

        kb.press(Key::AltLeft);
        assert!(kb.is_alt_pressed());

        kb.press(Key::SuperLeft);
        assert!(kb.is_super_pressed());
    }
}
