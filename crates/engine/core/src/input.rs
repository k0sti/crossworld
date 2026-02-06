//! Input types for game controllers and mouse/cursor handling
//!
//! This module provides generic input types that can be used across different
//! applications and compiled to WASM.

use glam::Vec2;

// ============================================================================
// Mouse and Cursor Types
// ============================================================================

/// Mouse button state flags
#[derive(Debug, Clone, Copy, Default)]
pub struct MouseButtons {
    pub left: bool,
    pub right: bool,
    pub middle: bool,
}

/// Mouse button type for injection
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MouseButtonType {
    Left,
    Right,
    Middle,
}

/// Cursor mode for the application
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
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
#[derive(Debug, Clone)]
pub struct ControllerInfo {
    /// Unique identifier for this controller
    pub id: usize,
    /// Human-readable name of the controller
    pub name: String,
    /// Whether the controller is currently connected
    pub connected: bool,
}

/// Gamepad state tracker
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
        const DEADZONE: f32 = 0.15;
        let vec = Vec2::new(self.raw_left_stick.x, self.raw_left_stick.y);

        if vec.length() < DEADZONE {
            self.left_stick = Vec2::ZERO;
        } else {
            // Apply deadzone and renormalize
            let normalized = vec.normalize_or_zero();
            let magnitude = ((vec.length() - DEADZONE) / (1.0 - DEADZONE)).clamp(0.0, 1.0);
            self.left_stick = normalized * magnitude;
        }
    }

    /// Process right stick with deadzone
    fn process_right_stick(&mut self) {
        const DEADZONE: f32 = 0.15;
        let vec = Vec2::new(self.raw_right_stick.x, self.raw_right_stick.y);

        if vec.length() < DEADZONE {
            self.right_stick = Vec2::ZERO;
        } else {
            // Apply deadzone and renormalize
            let normalized = vec.normalize_or_zero();
            let magnitude = ((vec.length() - DEADZONE) / (1.0 - DEADZONE)).clamp(0.0, 1.0);
            self.right_stick = normalized * magnitude;
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
