//! Crossworld Voxel Editor
//!
//! A native OpenGL voxel editor using the app framework with glow/egui/winit.

use app::{run_app, AppConfig};
use editor::EditorApp;

fn main() {
    println!("=== Crossworld Voxel Editor ===\n");

    // Parse command line arguments
    let args: Vec<String> = std::env::args().collect();

    if let Some(arg) = args.get(1) {
        match arg.as_str() {
            "--help" | "-h" => {
                println!("Usage: editor [OPTIONS]");
                println!();
                println!("Options:");
                println!("  --help       Show this help message");
                return;
            }
            _ => {
                eprintln!("Unknown argument: {}", arg);
                eprintln!("Use --help for usage information");
                return;
            }
        }
    }

    let app = EditorApp::new();
    let config = AppConfig::new("Crossworld Voxel Editor").with_size(1280, 800);

    run_app(app, config);
}
