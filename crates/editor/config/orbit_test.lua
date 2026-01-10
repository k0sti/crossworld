-- Orbit Camera Test Configuration
-- Tests orbit camera for roll issues
-- Run with: cargo run --bin editor -- --config crates/editor/config/orbit_test.lua

debug_frames = 200

-- Simulate right-click drag for orbit camera rotation
events = {
    -- Initial mouse position
    { frame = 5, type = "mouse_move", x = 640, y = 400 },

    -- Start right-click drag for orbit
    { frame = 10, type = "mouse_click", button = "right", pressed = true },

    -- Drag diagonally (should trigger roll warning if there's an issue)
    { frame = 15, type = "mouse_move", x = 700, y = 350 },
    { frame = 20, type = "mouse_move", x = 750, y = 300 },
    { frame = 25, type = "mouse_move", x = 800, y = 280 },
    { frame = 30, type = "mouse_move", x = 850, y = 300 },
    { frame = 35, type = "mouse_move", x = 880, y = 350 },
    { frame = 40, type = "mouse_move", x = 900, y = 400 },
    { frame = 45, type = "mouse_move", x = 880, y = 450 },
    { frame = 50, type = "mouse_move", x = 850, y = 500 },
    { frame = 55, type = "mouse_move", x = 800, y = 520 },
    { frame = 60, type = "mouse_move", x = 750, y = 500 },
    { frame = 65, type = "mouse_move", x = 700, y = 450 },
    { frame = 70, type = "mouse_move", x = 680, y = 400 },

    -- Continue circular motion
    { frame = 75, type = "mouse_move", x = 640, y = 350 },
    { frame = 80, type = "mouse_move", x = 580, y = 300 },
    { frame = 85, type = "mouse_move", x = 520, y = 280 },
    { frame = 90, type = "mouse_move", x = 460, y = 300 },
    { frame = 95, type = "mouse_move", x = 420, y = 350 },
    { frame = 100, type = "mouse_move", x = 400, y = 400 },
    { frame = 105, type = "mouse_move", x = 420, y = 450 },
    { frame = 110, type = "mouse_move", x = 460, y = 500 },
    { frame = 115, type = "mouse_move", x = 520, y = 520 },
    { frame = 120, type = "mouse_move", x = 580, y = 500 },
    { frame = 125, type = "mouse_move", x = 620, y = 450 },
    { frame = 130, type = "mouse_move", x = 640, y = 400 },

    -- Release right-click
    { frame = 135, type = "mouse_click", button = "right", pressed = false },
}

captures = {
    { frame = 50, path = "output/orbit_test_050.png" },
    { frame = 100, path = "output/orbit_test_100.png" },
    { frame = 130, path = "output/orbit_test_130.png" },
}

output_dir = "output"
