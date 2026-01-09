-- CubeCoord Selection Debug Configuration
-- Tests the far/near mode selection for CubeCoord
-- Run with: cargo run --bin editor -- --config crates/editor/config/cubecoord_debug.lua

debug_frames = 80

events = {
    -- Move to screen center to hit the cube
    { frame = 5, type = "mouse_move", x = 640, y = 400 },

    -- After some frames, toggle mode with space key (simulated)
    -- Note: We can't inject key events via lua yet, so we'll observe the mode toggling
    -- by looking at the debug output before/after frame 20

    -- Move around to test different hit positions
    { frame = 30, type = "mouse_move", x = 660, y = 380 },
    { frame = 50, type = "mouse_move", x = 620, y = 420 },

    -- Click to place a voxel
    { frame = 60, type = "mouse_click", button = "left", pressed = true },
    { frame = 62, type = "mouse_click", button = "left", pressed = false },

    -- Move to see the new voxel
    { frame = 70, type = "mouse_move", x = 640, y = 400 },
}

captures = {
    { frame = 10, path = "output/cubecoord_initial.png" },
    { frame = 65, path = "output/cubecoord_after_place.png" },
    { frame = 75, path = "output/cubecoord_final.png" },
}

output_dir = "output"
