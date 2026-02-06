//! Physics Component Testbed
//!
//! Compares physics behavior between cuboid and terrain colliders.
//!
//! Scene configuration can be loaded from Lua files.
//! Use `--config <path>` to specify a config file.

use app::{cli::CommonArgs, run_app, AppConfig};
use clap::Parser;
use std::path::PathBuf;
use testbed::PhysicsTestbed;

/// Physics Component Testbed
///
/// Compares physics behavior between cuboid and terrain colliders.
#[derive(Parser)]
#[command(name = "testbed")]
#[command(about = "Physics testing application with cuboid vs terrain collider comparison")]
struct Args {
    #[command(flatten)]
    common: CommonArgs,
}

fn main() {
    println!("=== Physics Component Testbed ===");
    println!("Comparing cuboid vs terrain collider physics behavior\n");

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

    // Create testbed, optionally from config file
    let app = if let Some(path) = args.common.config_path() {
        PhysicsTestbed::from_config_file(path)
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

    let config = AppConfig::new("Physics Testbed - Left: Cuboid | Right: Terrain Collider")
        .with_size(1200, 700);

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
