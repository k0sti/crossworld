//! Egui integration for OpenGL applications
//!
//! Provides a simple wrapper around egui_glow and egui_winit for
//! consistent egui setup across applications.

use egui::Context as EguiContext;
use egui_glow::Painter;
use egui_winit::State as EguiState;
use glow::Context;
use std::sync::Arc;
use winit::window::Window;

/// Egui integration wrapper for OpenGL applications
///
/// Handles initialization, event processing, and rendering of egui UI.
pub struct EguiIntegration {
    /// Egui context for running UI logic
    pub ctx: EguiContext,
    /// Egui-winit state for event handling
    state: EguiState,
    /// Egui-glow painter for rendering
    painter: Painter,
}

impl EguiIntegration {
    /// Create a new egui integration for the given window and GL context
    ///
    /// # Safety
    /// The GL context must be current when this is called.
    pub unsafe fn new(window: &Window, gl: Arc<Context>) -> Self {
        let ctx = EguiContext::default();
        let state = EguiState::new(
            ctx.clone(),
            egui::ViewportId::ROOT,
            window,
            None,
            None,
            None,
        );
        let painter = Painter::new(gl, "", None, false).expect("Failed to create egui painter");

        Self {
            ctx,
            state,
            painter,
        }
    }

    /// Get a reference to the egui context
    pub fn context(&self) -> &EguiContext {
        &self.ctx
    }

    /// Handle a window event
    ///
    /// Returns true if egui wants exclusive use of this event.
    pub fn on_window_event(&mut self, window: &Window, event: &winit::event::WindowEvent) -> bool {
        let response = self.state.on_window_event(window, event);
        response.consumed
    }

    /// Begin a new frame for egui rendering
    ///
    /// Returns the raw input for the egui context.
    pub fn begin_frame(&mut self, window: &Window) -> egui::RawInput {
        self.state.take_egui_input(window)
    }

    /// End the frame and render egui
    ///
    /// Call this after running your egui code with the FullOutput from egui.
    pub fn end_frame(&mut self, window: &Window, full_output: egui::FullOutput, size: [u32; 2]) {
        self.state
            .handle_platform_output(window, full_output.platform_output);

        let clipped_primitives = self
            .ctx
            .tessellate(full_output.shapes, full_output.pixels_per_point);
        self.painter.paint_and_update_textures(
            size,
            full_output.pixels_per_point,
            &clipped_primitives,
            &full_output.textures_delta,
        );
    }

    /// Run egui for one frame with the given UI function
    ///
    /// This is a convenience method that handles begin_frame, running UI, and end_frame.
    pub fn run(
        &mut self,
        window: &Window,
        size: [u32; 2],
        run_ui: impl FnMut(&EguiContext),
    ) {
        let raw_input = self.begin_frame(window);
        let full_output = self.ctx.run(raw_input, run_ui);
        self.end_frame(window, full_output, size);
    }

    /// Run egui with full access to the output
    ///
    /// Use this when you need to inspect the egui output before rendering.
    /// The run_ui closure is called, then you get access to the FullOutput.
    pub fn run_with_output(
        &mut self,
        window: &Window,
        run_ui: impl FnMut(&EguiContext),
    ) -> egui::FullOutput {
        let raw_input = self.begin_frame(window);
        self.ctx.run(raw_input, run_ui)
    }

    /// Get pixels per point for the current scale
    pub fn pixels_per_point(&self) -> f32 {
        self.ctx.pixels_per_point()
    }
}

impl Drop for EguiIntegration {
    fn drop(&mut self) {
        self.painter.destroy();
    }
}
