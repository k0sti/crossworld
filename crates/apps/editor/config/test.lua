-- Editor Test Configuration
-- This file demonstrates the editor's test configuration capabilities.
-- Run with: cargo run --bin editor -- --config crates/editor/config/test.lua

-- Number of frames to run before automatically exiting
-- Set to nil or remove to run indefinitely
debug_frames = 120

-- Mouse events to inject at specific frames
-- Each event has:
--   frame: Frame number when the event occurs (0-indexed)
--   type: "mouse_move" or "mouse_click"
--   x, y: Screen coordinates for mouse_move
--   button: "left", "right", or "middle" for mouse_click
--   pressed: true/false for mouse_click
events = {
    -- Initial mouse position (center of window at 1280x800)
    { frame = 5, type = "mouse_move", x = 640, y = 400 },

    -- Move to upper-left quadrant
    { frame = 20, type = "mouse_move", x = 400, y = 250 },

    -- Move to lower-right quadrant
    { frame = 40, type = "mouse_move", x = 880, y = 550 },

    -- Move to center-left
    { frame = 60, type = "mouse_move", x = 300, y = 400 },

    -- Simulate a click (shows cursor in Far mode - green wireframe)
    { frame = 75, type = "mouse_move", x = 640, y = 350 },
    { frame = 80, type = "mouse_click", button = "left", pressed = true },
    { frame = 82, type = "mouse_click", button = "left", pressed = false },

    -- Move to another position for final capture
    { frame = 100, type = "mouse_move", x = 500, y = 300 },
}

-- Frame captures to save as images
-- Each capture has:
--   frame: Frame number when to capture (after rendering)
--   path: Output file path (relative to config file directory or absolute)
captures = {
    -- Capture initial state
    { frame = 10, path = "output/frame_010_initial.png" },

    -- Capture with cursor in upper-left
    { frame = 25, path = "output/frame_025_upper_left.png" },

    -- Capture with cursor in lower-right
    { frame = 45, path = "output/frame_045_lower_right.png" },

    -- Capture during click interaction
    { frame = 81, path = "output/frame_081_click.png" },

    -- Final capture
    { frame = 110, path = "output/frame_110_final.png" },
}

-- Output directory for captures (relative to config file directory)
-- If not specified, paths in captures are used as-is
output_dir = "output"
