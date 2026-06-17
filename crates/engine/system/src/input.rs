//! Input module - re-exports from core
//!
//! The canonical input implementation lives in [`core::input`].

pub use core::input::*;

#[cfg(test)]
mod tests {
    use super::*;
    use winit::keyboard::KeyCode;

    #[test]
    fn reexports_core_input_types() {
        let mut input = InputState::new();
        input.inject_key(KeyCode::KeyW, true);
        assert!(input.is_key_pressed(KeyCode::KeyW));
        let _ = CursorMode::default();
        let _ = GamepadState::new();
    }
}
