//! Gilrs-based controller backend implementation

use crate::controller::{ControllerBackend, ControllerInfo, ControllerInput};
use gilrs::{Gilrs, GamepadId};
use std::collections::HashMap;

/// Gilrs-based controller backend
pub struct GilrsBackend {
    gilrs: Gilrs,
    /// Map from gilrs GamepadId to our controller state
    controllers: HashMap<GamepadId, ControllerInput>,
    /// Ordered list of gamepad IDs for enumeration
    gamepad_ids: Vec<GamepadId>,
}

impl GilrsBackend {
    /// Create a new gilrs backend
    pub fn new() -> Result<Self, gilrs::Error> {
        let gilrs = Gilrs::new()?;
        println!("[Controller] Gilrs backend initialized");

        let mut backend = Self {
            gilrs,
            controllers: HashMap::new(),
            gamepad_ids: Vec::new(),
        };

        // Initialize state for already-connected controllers
        backend.refresh_controllers();

        Ok(backend)
    }

    /// Refresh the list of connected controllers
    fn refresh_controllers(&mut self) {
        self.gamepad_ids.clear();

        for (id, gamepad) in self.gilrs.gamepads() {
            if gamepad.is_connected() {
                self.gamepad_ids.push(id);

                // Initialize controller state if not already present
                if !self.controllers.contains_key(&id) {
                    let mut input = ControllerInput::new();
                    input.connect();
                    self.controllers.insert(id, input);
                    println!("[Controller] Gamepad '{}' connected", gamepad.name());
                }
            }
        }

        // Remove disconnected controllers from the map
        self.controllers.retain(|id, _| {
            self.gilrs.gamepad(*id).is_connected()
        });
    }

    /// Get the gilrs GamepadId for a given enumeration index
    fn get_gamepad_id(&self, index: usize) -> Option<GamepadId> {
        self.gamepad_ids.get(index).copied()
    }
}

impl ControllerBackend for GilrsBackend {
    fn poll(&mut self) {
        while let Some(event) = self.gilrs.next_event() {
            match event.event {
                gilrs::EventType::Connected => {
                    self.refresh_controllers();
                }
                gilrs::EventType::Disconnected => {
                    if let Some(controller) = self.controllers.get_mut(&event.id) {
                        controller.disconnect();
                    }
                    self.refresh_controllers();
                    println!("[Controller] Gamepad disconnected");
                }
                gilrs::EventType::AxisChanged(axis, value, _) => {
                    if let Some(controller) = self.controllers.get_mut(&event.id) {
                        use gilrs::Axis;
                        match axis {
                            Axis::LeftStickX => {
                                controller.gamepad.set_left_stick_x(value);
                            }
                            Axis::LeftStickY => {
                                controller.gamepad.set_left_stick_y(value);
                            }
                            Axis::RightStickX => {
                                controller.gamepad.set_right_stick_x(value);
                            }
                            Axis::RightStickY => {
                                controller.gamepad.set_right_stick_y(value);
                            }
                            Axis::LeftZ => {
                                controller.gamepad.set_left_trigger(value);
                            }
                            Axis::RightZ => {
                                controller.gamepad.set_right_trigger(value);
                            }
                            Axis::DPadX => {
                                let current_y = controller.gamepad.dpad.y;
                                controller.gamepad.set_dpad(value, current_y);
                            }
                            Axis::DPadY => {
                                let current_x = controller.gamepad.dpad.x;
                                controller.gamepad.set_dpad(current_x, value);
                            }
                            _ => {}
                        }
                    }
                }
                _ => {}
            }
        }
    }

    fn enumerate(&self) -> Vec<ControllerInfo> {
        self.gamepad_ids
            .iter()
            .enumerate()
            .filter_map(|(index, id)| {
                let gamepad = self.gilrs.gamepad(*id);
                if gamepad.is_connected() {
                    Some(ControllerInfo {
                        id: index,
                        name: gamepad.name().to_string(),
                        connected: true,
                    })
                } else {
                    None
                }
            })
            .collect()
    }

    fn get_controller(&mut self, id: usize) -> Option<&mut ControllerInput> {
        let gamepad_id = self.get_gamepad_id(id)?;
        self.controllers.get_mut(&gamepad_id)
    }

    fn get_first_controller(&mut self) -> Option<&mut ControllerInput> {
        let first_id = self.gamepad_ids.first().copied()?;
        self.controllers.get_mut(&first_id)
    }
}
