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
//!
//! # Display a review panel with inline message
//! editor --review "Review message here"
//!
//! # Display a review panel from file
//! editor --review-file doc/review/current.md
//! ```

use app::{cli::CommonArgs, run_app, AppConfig};
use clap::Parser;
use editor::EditorApp;

/// Crossworld Voxel Editor
///
/// A native OpenGL voxel editor for creating and modifying voxel models.
#[derive(Parser)]
#[command(name = "editor")]
#[command(about = "Voxel editor with automated testing support")]
struct Args {
    #[command(flatten)]
    common: CommonArgs,
}

fn main() {
    println!("=== Crossworld Voxel Editor ===\n");

    let args = Args::parse();

    // Log configuration
    if let Some(frames) = args.common.debug {
        println!("Debug mode: running {} frames\n", frames);
    }
    if let Some(ref path) = args.common.config {
        println!("Using config file: {}\n", path.display());
    }
    if let Some(ref note) = args.common.note {
        println!("Note message: {}\n", note);
    }
    if let Some(ref message) = args.common.review {
        println!("Review message: {}\n", message);
    }
    if let Some(ref path) = args.common.review_file {
        println!("Review document: {}\n", path.display());
    }

    // Create editor app
    let app = if let Some(path) = args.common.config_path() {
        EditorApp::from_config_file(path)
    } else if let Some(frames) = args.common.debug_frames() {
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

    // Apply common arguments to config
    let config = match args.common.apply_to(config) {
        Ok(cfg) => cfg,
        Err(e) => {
            eprintln!("Error: Failed to apply configuration: {}", e);
            return;
        }
    };

    run_app(app, config);
}
