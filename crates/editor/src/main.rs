//! Crossworld Voxel Editor
//!
//! A native OpenGL voxel editor using the app framework with glow/egui/winit.
//!
//! # Usage
//!
//! ```bash
//! # Normal interactive mode
//! editor
//!
//! # Run with test configuration (automated mouse events, frame capture)
//! editor --config crates/editor/config/test.lua
//!
//! # Debug mode: run N frames then exit
//! editor --debug 100
//! ```

use app::{run_app, AppConfig};
use editor::EditorApp;
use std::path::PathBuf;

fn main() {
    println!("=== Crossworld Voxel Editor ===\n");

    // Parse command line arguments
    let args: Vec<String> = std::env::args().collect();
    let mut config_path: Option<PathBuf> = None;
    let mut debug_frames: Option<u64> = None;

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--help" | "-h" => {
                println!("Usage: editor [OPTIONS]");
                println!();
                println!("Options:");
                println!("  --config PATH   Load test configuration from Lua file");
                println!("  --debug N       Run only N frames then exit");
                println!("  --help          Show this help message");
                println!();
                println!("Test Configuration:");
                println!("  The Lua config file can define:");
                println!("  - debug_frames: Number of frames to run");
                println!("  - events: Mouse events to inject at specific frames");
                println!("  - captures: Frame captures to save");
                println!();
                println!("Example Lua config:");
                println!("  debug_frames = 60");
                println!("  events = {{");
                println!("    {{ frame = 10, type = \"mouse_move\", x = 400, y = 300 }},");
                println!("    {{ frame = 20, type = \"mouse_click\", button = \"left\", pressed = true }},");
                println!("  }}");
                println!("  captures = {{");
                println!("    {{ frame = 30, path = \"output/frame_030.png\" }},");
                println!("  }}");
                return;
            }
            "--config" | "-c" => {
                if i + 1 < args.len() {
                    config_path = Some(PathBuf::from(&args[i + 1]));
                    println!("Using config file: {}\n", args[i + 1]);
                    i += 1;
                } else {
                    eprintln!("Error: --config requires a path");
                    return;
                }
            }
            "--debug" => {
                if i + 1 < args.len() {
                    match args[i + 1].parse::<u64>() {
                        Ok(n) => {
                            debug_frames = Some(n);
                            println!("Debug mode: running {} frames\n", n);
                        }
                        Err(_) => {
                            eprintln!("Error: --debug requires a number of frames");
                            return;
                        }
                    }
                    i += 1;
                } else {
                    eprintln!("Error: --debug requires a number of frames");
                    return;
                }
            }
            _ => {
                eprintln!("Unknown argument: {}", args[i]);
                eprintln!("Use --help for usage information");
                return;
            }
        }
        i += 1;
    }

    // Create editor app
    let app = if let Some(path) = config_path {
        EditorApp::from_config_file(&path)
    } else if let Some(frames) = debug_frames {
        // Create a minimal config with just debug_frames
        let test_config = editor::lua_config::EditorTestConfig {
            debug_frames: Some(frames),
            ..Default::default()
        };
        EditorApp::new().with_test_config(test_config)
    } else {
        EditorApp::new()
    };

    let config = AppConfig::new("Crossworld Voxel Editor").with_size(1280, 800);

    run_app(app, config);
}
