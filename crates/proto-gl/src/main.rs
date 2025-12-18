use std::error::Error;
use clap::Parser;
use winit::event_loop::EventLoop;
use proto_gl::ProtoGlApp;

#[derive(Parser, Debug)]
#[command(name = "proto-gl")]
#[command(about = "Proto-GL Physics Viewer")]
struct Args {
    /// Debug mode: run N physics iterations (default: 100), log data, and exit
    /// Use 0 for normal windowed mode with debug output
    #[arg(long)]
    debug: Option<Option<u32>>,

    /// Capture a single frame and save to file (e.g., --capture-frame output.png)
    /// Renders one frame, saves to the specified path, then exits
    #[arg(long, value_name = "PATH")]
    capture_frame: Option<String>,
}

fn main() -> Result<(), Box<dyn Error>> {
    let args = Args::parse();

    // Check if --debug was passed (with or without a value)
    if let Some(iterations_opt) = args.debug {
        let iterations = iterations_opt.unwrap_or(100);

        if iterations > 0 {
            // Run in debug mode - no window, just physics simulation
            println!("=== PHYSICS DEBUG MODE ===");
            println!("Running {} physics iterations...\n", iterations);

            proto_gl::run_physics_debug(iterations)?;

            return Ok(());
        }

        // iterations == 0 means run windowed with debug flag
        let event_loop = EventLoop::new()?;
        let mut app = ProtoGlApp::new(true, None);
        event_loop.run_app(&mut app)?;
        return Ok(());
    }

    // Check if --capture-frame was passed
    if let Some(path) = args.capture_frame {
        println!("=== CAPTURE FRAME MODE ===");
        println!("Will render single frame and save to: {}\n", path);

        let event_loop = EventLoop::new()?;
        let mut app = ProtoGlApp::new(true, Some(path));
        event_loop.run_app(&mut app)?;
        return Ok(());
    }

    // Normal windowed mode
    let event_loop = EventLoop::new()?;
    let mut app = ProtoGlApp::new(false, None);

    event_loop.run_app(&mut app)?;
    Ok(())
}
