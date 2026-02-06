//! Gamepad/controller state and input processing
//!
//! This module provides types for tracking gamepad state with deadzone processing
//! and sensitivity settings.

use glam::Vec2;

/// Default deadzone value for analog sticks
pub const DEFAULT_DEADZONE: f32 = 0.15;

/// Controller information for enumeration
#[derive(Debug, Clone)]
pub struct ControllerInfo {
    /// Unique identifier for this controller
    pub id: usize,
    /// Human-readable name of the controller
    pub name: String,
    /// Whether the controller is currently connected
    pub connected: bool,
}

/// Gamepad state tracker with deadzone processing
///
/// Tracks the state of a gamepad including analog sticks, triggers, and d-pad.
/// Automatically applies deadzone processing to analog inputs.
#[derive(Debug, Clone)]
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
    /// D-pad as a vector (-1 to 1 for each axis)
    pub dpad: Vec2,
    /// Whether the gamepad is connected
    pub connected: bool,
    /// Deadzone threshold for analog sticks
    deadzone: f32,
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
            deadzone: DEFAULT_DEADZONE,
        }
    }
}

impl GamepadState {
    /// Create a new GamepadState with default settings
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a new GamepadState with custom deadzone
    pub fn with_deadzone(deadzone: f32) -> Self {
        Self {
            deadzone: deadzone.clamp(0.0, 0.9),
            ..Self::default()
        }
    }

    /// Get the current deadzone value
    pub fn deadzone(&self) -> f32 {
        self.deadzone
    }

    /// Set the deadzone value
    pub fn set_deadzone(&mut self, deadzone: f32) {
        self.deadzone = deadzone.clamp(0.0, 0.9);
        // Reprocess sticks with new deadzone
        self.process_left_stick();
        self.process_right_stick();
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
        self.left_stick = apply_radial_deadzone(self.raw_left_stick, self.deadzone);
    }

    /// Process right stick with deadzone
    fn process_right_stick(&mut self) {
        self.right_stick = apply_radial_deadzone(self.raw_right_stick, self.deadzone);
    }

    /// Update D-pad state
    pub fn set_dpad(&mut self, x: f32, y: f32) {
        self.dpad = Vec2::new(x, y);
    }

    /// Update left trigger value
    pub fn set_left_trigger(&mut self, value: f32) {
        self.left_trigger = value.clamp(0.0, 1.0);
    }

    /// Update right trigger value
    pub fn set_right_trigger(&mut self, value: f32) {
        self.right_trigger = value.clamp(0.0, 1.0);
    }

    /// Get movement vector from left stick (normalized, deadzone applied)
    pub fn get_movement(&self) -> Vec2 {
        self.left_stick
    }

    /// Get look vector from right stick (normalized, deadzone applied)
    pub fn get_look(&self) -> Vec2 {
        self.right_stick
    }

    /// Get vertical movement from triggers (right - left)
    ///
    /// Returns a value from -1.0 (left trigger fully pressed) to 1.0 (right trigger fully pressed)
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

    /// Check if any input is active (beyond deadzone)
    pub fn has_input(&self) -> bool {
        self.left_stick.length() > 0.01
            || self.right_stick.length() > 0.01
            || self.left_trigger > 0.01
            || self.right_trigger > 0.01
            || self.dpad.length() > 0.01
    }
}

/// Apply radial deadzone to an analog stick input
///
/// Uses a circular deadzone which feels more natural than per-axis deadzone.
fn apply_radial_deadzone(raw: Vec2, deadzone: f32) -> Vec2 {
    let magnitude = raw.length();

    if magnitude < deadzone {
        Vec2::ZERO
    } else {
        // Renormalize to make the usable range 0-1
        let normalized = raw.normalize_or_zero();
        let adjusted_magnitude = ((magnitude - deadzone) / (1.0 - deadzone)).clamp(0.0, 1.0);
        normalized * adjusted_magnitude
    }
}

/// Controller input handler with sensitivity settings
///
/// Wraps a GamepadState with additional sensitivity configuration
/// for camera and movement controls.
pub struct ControllerInput {
    /// Currently active gamepad state
    pub gamepad: GamepadState,
    /// Gamepad look sensitivity multiplier
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
    /// Create a new ControllerInput with default settings
    pub fn new() -> Self {
        Self {
            gamepad: GamepadState::new(),
            look_sensitivity: 3.0,
            move_speed_multiplier: 1.0,
        }
    }

    /// Create a new ControllerInput with custom sensitivity
    pub fn with_sensitivity(look_sensitivity: f32, move_speed_multiplier: f32) -> Self {
        Self {
            gamepad: GamepadState::new(),
            look_sensitivity,
            move_speed_multiplier,
        }
    }

    /// Calculate camera rotation delta from gamepad input
    ///
    /// Returns (yaw_delta, pitch_delta) in radians based on right stick input.
    pub fn get_camera_delta(&self, delta_time: f32) -> (f32, f32) {
        let look = self.gamepad.get_look();
        let sensitivity = self.look_sensitivity * delta_time;
        (look.x * sensitivity, look.y * sensitivity)
    }

    /// Calculate movement input from gamepad input
    ///
    /// Returns (right, up, forward) movement values.
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
        self.gamepad.has_input()
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gamepad_state_default() {
        let state = GamepadState::default();
        assert!(!state.connected);
        assert_eq!(state.left_stick, Vec2::ZERO);
        assert_eq!(state.right_stick, Vec2::ZERO);
        assert_eq!(state.deadzone(), DEFAULT_DEADZONE);
    }

    #[test]
    fn test_deadzone_processing() {
        let mut state = GamepadState::new();

        // Small input should be filtered out
        state.set_left_stick_x(0.1);
        state.set_left_stick_y(0.1);
        assert_eq!(state.left_stick, Vec2::ZERO);

        // Large input should be processed
        state.set_left_stick_x(0.8);
        state.set_left_stick_y(0.0);
        assert!(state.left_stick.x > 0.0);
    }

    #[test]
    fn test_trigger_clamping() {
        let mut state = GamepadState::new();

        state.set_left_trigger(1.5);
        assert_eq!(state.left_trigger, 1.0);

        state.set_right_trigger(-0.5);
        assert_eq!(state.right_trigger, 0.0);
    }

    #[test]
    fn test_vertical_movement() {
        let mut state = GamepadState::new();

        state.set_left_trigger(0.5);
        state.set_right_trigger(0.0);
        assert_eq!(state.get_vertical(), -0.5);

        state.set_left_trigger(0.0);
        state.set_right_trigger(1.0);
        assert_eq!(state.get_vertical(), 1.0);
    }

    #[test]
    fn test_controller_input_sensitivity() {
        let input = ControllerInput::with_sensitivity(5.0, 2.0);
        assert_eq!(input.look_sensitivity, 5.0);
        assert_eq!(input.move_speed_multiplier, 2.0);
    }

    #[test]
    fn test_controller_input_connect_disconnect() {
        let mut input = ControllerInput::new();
        assert!(!input.gamepad.connected);

        input.connect();
        assert!(input.gamepad.connected);

        input.gamepad.set_left_stick_x(0.9);
        input.disconnect();

        assert!(!input.gamepad.connected);
        assert_eq!(input.gamepad.left_stick, Vec2::ZERO);
    }
}
