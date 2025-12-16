use std::error::Error;
use clap::Parser;
use winit::event_loop::EventLoop;
use proto_gl::ProtoGlApp;

#[derive(Parser, Debug)]
#[command(name = "proto-gl")]
#[command(about = "Proto-GL Physics Viewer")]
struct Args {
    /// Debug mode: run only a single frame then exit
    #[arg(long)]
    debug: bool,
}

fn main() -> Result<(), Box<dyn Error>> {
    let args = Args::parse();

    let event_loop = EventLoop::new()?;
    let mut app = ProtoGlApp::new(args.debug);

    event_loop.run_app(&mut app)?;
    Ok(())
}
