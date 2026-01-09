//! Physics Component Testbed
//!
//! Compares physics behavior between cuboid and terrain colliders.
//!
//! Scene configuration can be loaded from Lua files.
//! Use `--config <path>` to specify a config file.

use app::{run_app, AppConfig};
use testbed::PhysicsTestbed;
use std::path::PathBuf;

fn main() {
    println!("=== Physics Component Testbed ===");
    println!("Comparing cuboid vs terrain collider physics behavior\n");

    // Parse command line arguments
    let args: Vec<String> = std::env::args().collect();
    let mut debug_frames: Option<u64> = None;
    let mut config_path: Option<PathBuf> = None;

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
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
            "--help" | "-h" => {
                println!("Usage: testbed [OPTIONS]");
                println!();
                println!("Options:");
                println!("  --debug N       Run only N frames with debug output");
                println!("  --config PATH   Load scene configuration from Lua (.lua) file");
                println!("  --help          Show this help message");
                return;
            }
            _ => {
                eprintln!("Unknown argument: {}", args[i]);
                eprintln!("Use --help for usage information");
                return;
            }
        }
        i += 1;
    }

    // Create testbed, optionally from config file
    let app = if let Some(path) = config_path {
        PhysicsTestbed::from_config_file(&path)
    } else {
        // Try default config location
        let default_config = PathBuf::from("crates/testbed/config/scene.lua");
        if default_config.exists() {
            println!("Loading default config: {:?}", default_config);
            PhysicsTestbed::from_config_file(&default_config)
        } else {
            PhysicsTestbed::new()
        }
    };

    let mut config = AppConfig::new("Physics Testbed - Left: Cuboid | Right: Terrain Collider")
        .with_size(1200, 700);

    // Apply debug mode if requested
    if let Some(frames) = debug_frames {
        config = config.with_debug_mode(frames);
    }

    run_app(app, config);
}
