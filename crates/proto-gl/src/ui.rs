use glam::{Vec3, Quat};

/// UI state captured from app for rendering
pub struct UiState {
    pub fps: f32,
    pub frame_time: f32,
    pub world_depth: u32,
    pub gravity: f32,
    pub timestep: f32,
    pub camera_distance: f32,
    pub camera_yaw: f32,
    pub camera_pitch: f32,
    pub camera_pos: Vec3,
    pub camera_rot: Quat,
    pub object_count: usize,
    pub render_world: bool,
    pub render_objects: bool,
    pub show_debug_info: bool,
}

/// Render the egui debug panel
pub fn render_debug_panel(ctx: &egui::Context, state: &mut UiState) {
    egui::SidePanel::right("controls").show(ctx, |ui| {
        ui.heading("Proto-GL Viewer");

        ui.separator();
        ui.label(format!("FPS: {:.1}", state.fps));
        ui.label(format!("Frame time: {:.2} ms", state.frame_time * 1000.0));
        ui.label(format!("Objects: {}", state.object_count));

        ui.separator();
        ui.heading("Configuration");
        ui.label(format!("World depth: {}", state.world_depth));
        ui.label(format!("Gravity: {:.2}", state.gravity));
        ui.label(format!("Timestep: {:.4}", state.timestep));

        ui.separator();
        ui.heading("Camera");
        ui.label(format!("Distance: {:.1}", state.camera_distance));
        ui.label(format!("Yaw: {:.2}", state.camera_yaw));
        ui.label(format!("Pitch: {:.2}", state.camera_pitch));

        ui.separator();
        ui.heading("Rendering");
        ui.checkbox(&mut state.render_world, "Render World");
        ui.checkbox(&mut state.render_objects, "Render Objects");
        ui.checkbox(&mut state.show_debug_info, "Show Debug Info");

        if state.show_debug_info {
            ui.separator();
            ui.heading("Debug Info");
            ui.label(format!("Cam Pos: ({:.1}, {:.1}, {:.1})",
                state.camera_pos.x, state.camera_pos.y, state.camera_pos.z));
            ui.label(format!("Cam Rot: ({:.2}, {:.2}, {:.2}, {:.2})",
                state.camera_rot.x, state.camera_rot.y, state.camera_rot.z, state.camera_rot.w));
        }

        ui.separator();
        if ui.button("Reset Scene").clicked() {
            println!("Reset scene (not yet implemented)");
        }

        ui.separator();
        ui.label("Controls:");
        ui.label("• Right-click drag: Rotate camera");
        ui.label("• Mouse wheel: Zoom");
    });
}
