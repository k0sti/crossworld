//! Physics Component Testbed
//!
//! Compares physics behavior between cuboid and terrain colliders.

use app::{run_app, AppConfig};
use testbed::PhysicsTestbed;

fn main() {
    println!("=== Physics Component Testbed ===");
    println!("Comparing cuboid vs terrain collider physics behavior\n");

    // Parse command line arguments
    let args: Vec<String> = std::env::args().collect();
    let mut debug_frames: Option<u64> = None;

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
            "--help" | "-h" => {
                println!("Usage: testbed [OPTIONS]");
                println!();
                println!("Options:");
                println!("  --debug N    Run only N frames with debug output");
                println!("  --help       Show this help message");
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

    let app = PhysicsTestbed::new().with_debug_frames(debug_frames);
    let config = AppConfig::new("Physics Testbed - Left: Cuboid | Right: Terrain Collider")
        .with_size(1200, 700);

    run_app(app, config);
}
