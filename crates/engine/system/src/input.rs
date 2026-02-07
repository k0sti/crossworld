//! Input state management
//!
//! This module provides unified input state handling for keyboard, mouse,
//! and gamepad input across all platforms. Consolidates functionality from
//! both core/input and app/InputState.

use glam::Vec2;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use winit::keyboard::KeyCode;

// ============================================================================
// Mouse and Cursor Types
// ============================================================================

/// Mouse button state flags
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
pub struct MouseButtons {
    pub left: bool,
    pub right: bool,
    pub middle: bool,
}

/// Mouse button type for injection
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MouseButtonType {
    Left,
    Right,
    Middle,
}

/// Cursor mode for the application
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum CursorMode {
    /// Visible, free movement (default)
    #[default]
    Normal,
    /// Hidden but not grabbed
    Hidden,
    /// Hidden and confined/locked (for FPS camera)
    Grabbed,
}

// ============================================================================
// Controller Types
// ============================================================================

/// Controller information for enumeration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ControllerInfo {
    /// Unique identifier for this controller
    pub id: usize,
    /// Human-readable name of the controller
    pub name: String,
    /// Whether the controller is currently connected
    pub connected: bool,
}

/// Gamepad state tracker
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GamepadState {
    /// Left stick (movement) - processed with deadzone
    pub left_stick: Vec2,
    /// Right stick (camera look) - processed with deadzone
    pub right_stick: Vec2,
    /// Raw left stick values (before deadzone)
    raw_left_stick: Vec2,
    /// Raw right stick values (before deadzone)
    raw_right_stick: Vec2,
    /// Left trigger (0.0 to 1.0)
    pub left_trigger: f32,
    /// Right trigger (0.0 to 1.0)
    pub right_trigger: f32,
    /// D-pad as a vector
    pub dpad: Vec2,
    /// Whether the gamepad is connected
    pub connected: bool,
}

impl Default for GamepadState {
    fn default() -> Self {
        Self {
            left_stick: Vec2::ZERO,
            right_stick: Vec2::ZERO,
            raw_left_stick: Vec2::ZERO,
            raw_right_stick: Vec2::ZERO,
            left_trigger: 0.0,
            right_trigger: 0.0,
            dpad: Vec2::ZERO,
            connected: false,
        }
    }
}

/// Default deadzone for analog sticks
const STICK_DEADZONE: f32 = 0.15;

impl GamepadState {
    pub fn new() -> Self {
        Self::default()
    }

    /// Update left stick X axis (raw value)
    pub fn set_left_stick_x(&mut self, value: f32) {
        self.raw_left_stick.x = value;
        self.process_left_stick();
    }

    /// Update left stick Y axis (raw value)
    pub fn set_left_stick_y(&mut self, value: f32) {
        self.raw_left_stick.y = value;
        self.process_left_stick();
    }

    /// Update right stick X axis (raw value)
    pub fn set_right_stick_x(&mut self, value: f32) {
        self.raw_right_stick.x = value;
        self.process_right_stick();
    }

    /// Update right stick Y axis (raw value)
    pub fn set_right_stick_y(&mut self, value: f32) {
        self.raw_right_stick.y = value;
        self.process_right_stick();
    }

    /// Process left stick with deadzone
    fn process_left_stick(&mut self) {
        self.left_stick = Self::apply_deadzone(self.raw_left_stick, STICK_DEADZONE);
    }

    /// Process right stick with deadzone
    fn process_right_stick(&mut self) {
        self.right_stick = Self::apply_deadzone(self.raw_right_stick, STICK_DEADZONE);
    }

    /// Apply radial deadzone to a 2D vector
    fn apply_deadzone(input: Vec2, deadzone: f32) -> Vec2 {
        let magnitude = input.length();
        if magnitude < deadzone {
            Vec2::ZERO
        } else {
            // Renormalize to maintain smooth movement after deadzone
            let normalized = input.normalize_or_zero();
            let adjusted_magnitude = ((magnitude - deadzone) / (1.0 - deadzone)).clamp(0.0, 1.0);
            normalized * adjusted_magnitude
        }
    }

    /// Update D-pad state
    pub fn set_dpad(&mut self, x: f32, y: f32) {
        self.dpad = Vec2::new(x, y);
    }

    /// Update trigger values
    pub fn set_left_trigger(&mut self, value: f32) {
        self.left_trigger = value.clamp(0.0, 1.0);
    }

    pub fn set_right_trigger(&mut self, value: f32) {
        self.right_trigger = value.clamp(0.0, 1.0);
    }

    /// Get movement vector from left stick (normalized)
    pub fn get_movement(&self) -> Vec2 {
        self.left_stick
    }

    /// Get look vector from right stick (normalized)
    pub fn get_look(&self) -> Vec2 {
        self.right_stick
    }

    /// Get vertical movement from triggers (right - left)
    pub fn get_vertical(&self) -> f32 {
        self.right_trigger - self.left_trigger
    }

    /// Reset all inputs to default state
    pub fn reset(&mut self) {
        self.left_stick = Vec2::ZERO;
        self.right_stick = Vec2::ZERO;
        self.raw_left_stick = Vec2::ZERO;
        self.raw_right_stick = Vec2::ZERO;
        self.left_trigger = 0.0;
        self.right_trigger = 0.0;
        self.dpad = Vec2::ZERO;
    }
}

/// Controller input handler with sensitivity settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ControllerInput {
    /// Currently active gamepad state
    pub gamepad: GamepadState,
    /// Gamepad look sensitivity
    pub look_sensitivity: f32,
    /// Gamepad move speed multiplier
    pub move_speed_multiplier: f32,
}

impl Default for ControllerInput {
    fn default() -> Self {
        Self::new()
    }
}

impl ControllerInput {
    pub fn new() -> Self {
        Self {
            gamepad: GamepadState::new(),
            look_sensitivity: 3.0,
            move_speed_multiplier: 1.0,
        }
    }

    /// Calculate camera rotation delta from gamepad input
    pub fn get_camera_delta(&self, delta_time: f32) -> (f32, f32) {
        let look = self.gamepad.get_look();
        let sensitivity = self.look_sensitivity * delta_time;
        (look.x * sensitivity, look.y * sensitivity)
    }

    /// Calculate movement input from gamepad input
    pub fn get_movement_input(&self) -> (f32, f32, f32) {
        let movement = self.gamepad.get_movement();
        let vertical = self.gamepad.get_vertical();

        (
            movement.x * self.move_speed_multiplier,
            vertical,
            movement.y * self.move_speed_multiplier,
        )
    }

    /// Check if gamepad is providing any input
    pub fn has_input(&self) -> bool {
        self.gamepad.left_stick.length() > 0.01
            || self.gamepad.right_stick.length() > 0.01
            || self.gamepad.left_trigger > 0.01
            || self.gamepad.right_trigger > 0.01
    }

    /// Mark gamepad as connected
    pub fn connect(&mut self) {
        self.gamepad.connected = true;
    }

    /// Mark gamepad as disconnected and reset state
    pub fn disconnect(&mut self) {
        self.gamepad.connected = false;
        self.gamepad.reset();
    }
}

// ============================================================================
// Input State (Unified)
// ============================================================================

/// Input state snapshot for the current frame
///
/// Contains all input information aggregated from events.
/// This is the main input state type used by applications.
#[derive(Debug, Clone, Default)]
pub struct InputState {
    /// Currently pressed keys
    pub keys: HashSet<KeyCode>,
    /// Mouse position in window coordinates
    pub mouse_pos: Option<Vec2>,
    /// Mouse delta since last frame (from window events)
    pub mouse_delta: Vec2,
    /// Raw mouse motion (for FPS camera, not constrained by window)
    pub raw_mouse_delta: Vec2,
    /// Scroll delta
    pub scroll_delta: Vec2,
    /// Mouse buttons currently held
    pub mouse_buttons: MouseButtons,
    /// Gamepad state (if connected)
    pub gamepad: Option<GamepadState>,
}

impl InputState {
    /// Create a new empty input state
    pub fn new() -> Self {
        Self::default()
    }

    /// Check if a key is currently pressed
    #[inline]
    pub fn is_key_pressed(&self, key: KeyCode) -> bool {
        self.keys.contains(&key)
    }

    /// Check if any of the given keys are pressed
    pub fn any_key_pressed(&self, keys: &[KeyCode]) -> bool {
        keys.iter().any(|k| self.keys.contains(k))
    }

    /// Check if all of the given keys are pressed
    pub fn all_keys_pressed(&self, keys: &[KeyCode]) -> bool {
        keys.iter().all(|k| self.keys.contains(k))
    }

    /// Check if left mouse button is pressed
    #[inline]
    pub fn is_left_mouse_pressed(&self) -> bool {
        self.mouse_buttons.left
    }

    /// Check if right mouse button is pressed
    #[inline]
    pub fn is_right_mouse_pressed(&self) -> bool {
        self.mouse_buttons.right
    }

    /// Check if middle mouse button is pressed
    #[inline]
    pub fn is_middle_mouse_pressed(&self) -> bool {
        self.mouse_buttons.middle
    }

    /// Check if any mouse button is pressed
    pub fn any_mouse_pressed(&self) -> bool {
        self.mouse_buttons.left || self.mouse_buttons.right || self.mouse_buttons.middle
    }

    /// Get the mouse position if available
    pub fn mouse_position(&self) -> Option<Vec2> {
        self.mouse_pos
    }

    /// Check if a gamepad is connected
    pub fn has_gamepad(&self) -> bool {
        self.gamepad.as_ref().is_some_and(|g| g.connected)
    }

    /// Inject a mouse position event
    ///
    /// Used for automated testing and config-driven input.
    pub fn inject_mouse_pos(&mut self, x: f32, y: f32) {
        let new_pos = Vec2::new(x, y);
        if let Some(old_pos) = self.mouse_pos {
            self.mouse_delta = new_pos - old_pos;
        }
        self.mouse_pos = Some(new_pos);
    }

    /// Inject a mouse click event
    ///
    /// Used for automated testing and config-driven input.
    pub fn inject_mouse_click(&mut self, button: MouseButtonType, pressed: bool) {
        match button {
            MouseButtonType::Left => self.mouse_buttons.left = pressed,
            MouseButtonType::Right => self.mouse_buttons.right = pressed,
            MouseButtonType::Middle => self.mouse_buttons.middle = pressed,
        }
    }

    /// Inject a key press event
    ///
    /// Used for automated testing and config-driven input.
    pub fn inject_key(&mut self, key: KeyCode, pressed: bool) {
        if pressed {
            self.keys.insert(key);
        } else {
            self.keys.remove(&key);
        }
    }

    /// Inject scroll delta
    pub fn inject_scroll(&mut self, delta: Vec2) {
        self.scroll_delta = delta;
    }

    /// Clear all input state
    pub fn clear(&mut self) {
        self.keys.clear();
        self.mouse_pos = None;
        self.mouse_delta = Vec2::ZERO;
        self.raw_mouse_delta = Vec2::ZERO;
        self.scroll_delta = Vec2::ZERO;
        self.mouse_buttons = MouseButtons::default();
        self.gamepad = None;
    }

    /// Clear per-frame delta values (call at end of frame)
    ///
    /// This resets delta values that should not accumulate across frames.
    pub fn clear_deltas(&mut self) {
        self.mouse_delta = Vec2::ZERO;
        self.raw_mouse_delta = Vec2::ZERO;
        self.scroll_delta = Vec2::ZERO;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gamepad_deadzone() {
        let mut state = GamepadState::new();

        // Small input should be zeroed
        state.set_left_stick_x(0.1);
        state.set_left_stick_y(0.1);
        assert_eq!(state.left_stick, Vec2::ZERO);

        // Large input should pass through (adjusted)
        state.set_left_stick_x(0.8);
        state.set_left_stick_y(0.0);
        assert!(state.left_stick.x > 0.5);
    }

    #[test]
    fn test_input_state_keys() {
        let mut input = InputState::new();

        assert!(!input.is_key_pressed(KeyCode::KeyW));

        input.inject_key(KeyCode::KeyW, true);
        assert!(input.is_key_pressed(KeyCode::KeyW));

        input.inject_key(KeyCode::KeyW, false);
        assert!(!input.is_key_pressed(KeyCode::KeyW));
    }

    #[test]
    fn test_input_state_mouse() {
        let mut input = InputState::new();

        assert!(!input.is_left_mouse_pressed());

        input.inject_mouse_click(MouseButtonType::Left, true);
        assert!(input.is_left_mouse_pressed());

        input.inject_mouse_pos(100.0, 200.0);
        assert_eq!(input.mouse_pos, Some(Vec2::new(100.0, 200.0)));
    }

    #[test]
    fn test_input_state_clear() {
        let mut input = InputState::new();
        input.inject_key(KeyCode::KeyW, true);
        input.inject_mouse_click(MouseButtonType::Left, true);
        input.inject_mouse_pos(50.0, 50.0);

        input.clear();

        assert!(!input.is_key_pressed(KeyCode::KeyW));
        assert!(!input.is_left_mouse_pressed());
        assert!(input.mouse_pos.is_none());
    }

    #[test]
    fn test_cursor_mode() {
        assert_eq!(CursorMode::default(), CursorMode::Normal);
        assert!(CursorMode::Grabbed != CursorMode::Normal);
    }
}
