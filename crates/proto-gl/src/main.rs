use std::error::Error;
use winit::event_loop::EventLoop;
use proto_gl::ProtoGlApp;

fn main() -> Result<(), Box<dyn Error>> {
    let event_loop = EventLoop::new()?;
    let mut app = ProtoGlApp::default();

    event_loop.run_app(&mut app)?;
    Ok(())
}
