//! Lua configuration for the editor
//!
//! Allows defining mouse events and test scenarios in Lua files.
//!
//! # Example Lua Configuration
//!
//! ```lua
//! -- Editor test configuration
//! debug_frames = 60  -- Run for 60 frames then exit
//!
//! -- Mouse events at specific frames
//! events = {
//!     { frame = 10, type = "mouse_move", x = 400, y = 300 },
//!     { frame = 20, type = "mouse_click", button = "left", pressed = true },
//!     { frame = 25, type = "mouse_click", button = "left", pressed = false },
//!     { frame = 30, type = "mouse_move", x = 600, y = 400 },
//! }
//!
//! -- Optional: capture frames at specific points
//! captures = {
//!     { frame = 15, path = "output/frame_015.png" },
//!     { frame = 35, path = "output/frame_035.png" },
//! }
//! ```

use app::lua_config::mlua::prelude::*;
use app::lua_config::{extract_u32, LuaConfig};
use app::MouseButtonType;
use glam::Vec2;
use std::path::{Path, PathBuf};

/// A mouse event to inject at a specific frame
#[derive(Debug, Clone)]
pub enum MouseEvent {
    /// Move mouse to position
    Move { x: f32, y: f32 },
    /// Click or release a mouse button
    Click { button: MouseButtonType, pressed: bool },
}

/// A scheduled input event
#[derive(Debug, Clone)]
pub struct ScheduledEvent {
    /// Frame number when this event should occur
    pub frame: u64,
    /// The event to inject
    pub event: MouseEvent,
}

/// A scheduled frame capture
#[derive(Debug, Clone)]
pub struct ScheduledCapture {
    /// Frame number when to capture
    pub frame: u64,
    /// Output path for the captured frame
    pub path: PathBuf,
}

/// Editor test configuration loaded from Lua
#[derive(Debug, Clone, Default)]
pub struct EditorTestConfig {
    /// Number of frames to run before exiting (None = run indefinitely)
    pub debug_frames: Option<u64>,
    /// Mouse events to inject at specific frames
    pub events: Vec<ScheduledEvent>,
    /// Frames to capture
    pub captures: Vec<ScheduledCapture>,
    /// Output directory for captures (relative to config file)
    pub output_dir: Option<PathBuf>,
}

impl EditorTestConfig {
    /// Load configuration from a Lua file
    pub fn from_file(path: &Path) -> Result<Self, String> {
        let mut lua_config = LuaConfig::new().map_err(|e| format!("Failed to create Lua: {}", e))?;

        // Load the file
        lua_config.load_file(path)?;

        Self::from_lua_config(&lua_config, path.parent())
    }

    /// Parse configuration from a LuaConfig instance
    fn from_lua_config(lua_config: &LuaConfig, base_dir: Option<&Path>) -> Result<Self, String> {
        let lua = lua_config.lua();
        let globals = lua.globals();

        // Parse debug_frames
        let debug_frames: Option<u64> = globals
            .get::<LuaValue>("debug_frames")
            .ok()
            .and_then(|v| match v {
                LuaValue::Integer(i) if i > 0 => Some(i as u64),
                LuaValue::Number(n) if n > 0.0 => Some(n as u64),
                _ => None,
            });

        // Parse events
        let events = Self::parse_events(&globals)?;

        // Parse captures
        let captures = Self::parse_captures(&globals, base_dir)?;

        // Parse output_dir
        let output_dir: Option<PathBuf> = globals
            .get::<String>("output_dir")
            .ok()
            .map(|s| {
                let p = PathBuf::from(&s);
                if p.is_absolute() {
                    p
                } else if let Some(base) = base_dir {
                    base.join(p)
                } else {
                    p
                }
            });

        Ok(Self {
            debug_frames,
            events,
            captures,
            output_dir,
        })
    }

    /// Parse mouse events from Lua table
    fn parse_events(globals: &LuaTable) -> Result<Vec<ScheduledEvent>, String> {
        let events_table: Option<LuaTable> = globals.get("events").ok();
        let Some(table) = events_table else {
            return Ok(Vec::new());
        };

        let mut events = Vec::new();

        for pair in table.pairs::<LuaValue, LuaTable>() {
            let (_, event_table) = pair.map_err(|e| format!("Invalid event entry: {}", e))?;

            let frame_val: LuaValue = event_table
                .get("frame")
                .map_err(|e| format!("Event missing 'frame': {}", e))?;
            let frame = extract_u32(&frame_val).map_err(|e| format!("Invalid frame: {}", e))? as u64;

            let event_type: String = event_table
                .get("type")
                .map_err(|e| format!("Event missing 'type': {}", e))?;

            let event = match event_type.as_str() {
                "mouse_move" => {
                    let x: f32 = event_table
                        .get("x")
                        .map_err(|e| format!("mouse_move missing 'x': {}", e))?;
                    let y: f32 = event_table
                        .get("y")
                        .map_err(|e| format!("mouse_move missing 'y': {}", e))?;
                    MouseEvent::Move { x, y }
                }
                "mouse_click" => {
                    let button_str: String = event_table
                        .get("button")
                        .map_err(|e| format!("mouse_click missing 'button': {}", e))?;
                    let button = match button_str.as_str() {
                        "left" => MouseButtonType::Left,
                        "right" => MouseButtonType::Right,
                        "middle" => MouseButtonType::Middle,
                        other => return Err(format!("Unknown button: {}", other)),
                    };
                    let pressed: bool = event_table
                        .get("pressed")
                        .map_err(|e| format!("mouse_click missing 'pressed': {}", e))?;
                    MouseEvent::Click { button, pressed }
                }
                other => return Err(format!("Unknown event type: {}", other)),
            };

            events.push(ScheduledEvent { frame, event });
        }

        // Sort by frame number
        events.sort_by_key(|e| e.frame);

        Ok(events)
    }

    /// Parse capture requests from Lua table
    fn parse_captures(globals: &LuaTable, base_dir: Option<&Path>) -> Result<Vec<ScheduledCapture>, String> {
        let captures_table: Option<LuaTable> = globals.get("captures").ok();
        let Some(table) = captures_table else {
            return Ok(Vec::new());
        };

        let mut captures = Vec::new();

        for pair in table.pairs::<LuaValue, LuaTable>() {
            let (_, capture_table) = pair.map_err(|e| format!("Invalid capture entry: {}", e))?;

            let frame_val: LuaValue = capture_table
                .get("frame")
                .map_err(|e| format!("Capture missing 'frame': {}", e))?;
            let frame = extract_u32(&frame_val).map_err(|e| format!("Invalid frame: {}", e))? as u64;

            let path_str: String = capture_table
                .get("path")
                .map_err(|e| format!("Capture missing 'path': {}", e))?;

            let path = {
                let p = PathBuf::from(&path_str);
                if p.is_absolute() {
                    p
                } else if let Some(base) = base_dir {
                    base.join(p)
                } else {
                    p
                }
            };

            captures.push(ScheduledCapture { frame, path });
        }

        // Sort by frame number
        captures.sort_by_key(|c| c.frame);

        Ok(captures)
    }

    /// Get events scheduled for a specific frame
    pub fn events_for_frame(&self, frame: u64) -> Vec<&ScheduledEvent> {
        self.events.iter().filter(|e| e.frame == frame).collect()
    }

    /// Get captures scheduled for a specific frame
    pub fn captures_for_frame(&self, frame: u64) -> Vec<&ScheduledCapture> {
        self.captures.iter().filter(|c| c.frame == frame).collect()
    }

    /// Check if we should exit at this frame
    pub fn should_exit(&self, frame: u64) -> bool {
        self.debug_frames.is_some_and(|max| frame >= max)
    }

    /// Get the current mouse position from events up to the given frame
    pub fn mouse_position_at_frame(&self, frame: u64) -> Option<Vec2> {
        // Find the last move event at or before this frame
        self.events
            .iter()
            .filter(|e| e.frame <= frame)
            .filter_map(|e| match &e.event {
                MouseEvent::Move { x, y } => Some(Vec2::new(*x, *y)),
                _ => None,
            })
            .next_back()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_empty_config() {
        let mut lua_config = LuaConfig::new().unwrap();
        lua_config.load_string("-- empty config").unwrap();

        let config = EditorTestConfig::from_lua_config(&lua_config, None).unwrap();
        assert!(config.debug_frames.is_none());
        assert!(config.events.is_empty());
        assert!(config.captures.is_empty());
    }

    #[test]
    fn test_parse_debug_frames() {
        let mut lua_config = LuaConfig::new().unwrap();
        lua_config.load_string("debug_frames = 100").unwrap();

        let config = EditorTestConfig::from_lua_config(&lua_config, None).unwrap();
        assert_eq!(config.debug_frames, Some(100));
    }

    #[test]
    fn test_parse_mouse_events() {
        let mut lua_config = LuaConfig::new().unwrap();
        lua_config
            .load_string(
                r#"
            events = {
                { frame = 10, type = "mouse_move", x = 100.0, y = 200.0 },
                { frame = 20, type = "mouse_click", button = "left", pressed = true },
            }
        "#,
            )
            .unwrap();

        let config = EditorTestConfig::from_lua_config(&lua_config, None).unwrap();
        assert_eq!(config.events.len(), 2);

        assert_eq!(config.events[0].frame, 10);
        match &config.events[0].event {
            MouseEvent::Move { x, y } => {
                assert_eq!(*x, 100.0);
                assert_eq!(*y, 200.0);
            }
            _ => panic!("Expected Move event"),
        }

        assert_eq!(config.events[1].frame, 20);
        match &config.events[1].event {
            MouseEvent::Click { button, pressed } => {
                assert_eq!(*button, MouseButtonType::Left);
                assert!(*pressed);
            }
            _ => panic!("Expected Click event"),
        }
    }

    #[test]
    fn test_parse_captures() {
        let mut lua_config = LuaConfig::new().unwrap();
        lua_config
            .load_string(
                r#"
            captures = {
                { frame = 50, path = "output/test.png" },
            }
        "#,
            )
            .unwrap();

        let config = EditorTestConfig::from_lua_config(&lua_config, None).unwrap();
        assert_eq!(config.captures.len(), 1);
        assert_eq!(config.captures[0].frame, 50);
        assert_eq!(config.captures[0].path, PathBuf::from("output/test.png"));
    }

    #[test]
    fn test_events_for_frame() {
        let config = EditorTestConfig {
            debug_frames: None,
            events: vec![
                ScheduledEvent {
                    frame: 10,
                    event: MouseEvent::Move { x: 100.0, y: 100.0 },
                },
                ScheduledEvent {
                    frame: 10,
                    event: MouseEvent::Click {
                        button: MouseButtonType::Left,
                        pressed: true,
                    },
                },
                ScheduledEvent {
                    frame: 20,
                    event: MouseEvent::Move { x: 200.0, y: 200.0 },
                },
            ],
            captures: vec![],
            output_dir: None,
        };

        let events = config.events_for_frame(10);
        assert_eq!(events.len(), 2);

        let events = config.events_for_frame(20);
        assert_eq!(events.len(), 1);

        let events = config.events_for_frame(15);
        assert_eq!(events.len(), 0);
    }

    #[test]
    fn test_should_exit() {
        let config = EditorTestConfig {
            debug_frames: Some(50),
            ..Default::default()
        };

        assert!(!config.should_exit(0));
        assert!(!config.should_exit(49));
        assert!(config.should_exit(50));
        assert!(config.should_exit(100));
    }
}
