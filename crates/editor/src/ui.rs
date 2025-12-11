use bevy::prelude::*;
use bevy_egui::{egui, EguiContexts};
use crate::cursor::{EditorState, FocusMode};
use crate::file_io::FileState;

/// Resource to track frames elapsed (for skipping initial frames)
#[derive(Resource, Default)]
pub(crate) struct UiFrameCounter(u32);

/// System that renders the UI panels
pub fn render_ui(
    mut contexts: EguiContexts,
    editor_state: Res<EditorState>,
    file_state: Res<FileState>,
    mut frame_counter: ResMut<UiFrameCounter>,
) {
    // Skip first 2 frames to let egui fully initialize
    frame_counter.0 += 1;
    if frame_counter.0 < 3 {
        return;
    }

    // Get the primary window's context
    let Ok(ctx) = contexts.ctx_mut() else {
        warn!("UI: egui context not available");
        return;
    };

    // Render panels with error handling
    let status_result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        egui::TopBottomPanel::bottom("status_bar")
            .default_height(60.0)
            .show(ctx, |ui| {
            ui.vertical(|ui| {
                ui.heading("Crossworld Voxel Editor");

                ui.horizontal(|ui| {
                    // File status
                    let filename = file_state.get_filename();
                    let dirty_marker = if file_state.dirty { "*" } else { "" };
                    ui.label(format!("File: {}{}", filename, dirty_marker));

                    ui.separator();

                    // Cursor info
                    if editor_state.cursor.valid {
                        ui.label(format!(
                            "Cursor: ({:.0}, {:.0}, {:.0})",
                            editor_state.cursor.position.x,
                            editor_state.cursor.position.y,
                            editor_state.cursor.position.z
                        ));
                        ui.label(format!("Size: {}", editor_state.cursor.size));
                    } else {
                        ui.label("Cursor: (no target)");
                    }

                    ui.separator();

                    // Focus mode
                    let focus_mode_text = match editor_state.focus_mode {
                        FocusMode::Near => "Remove Mode (Red)",
                        FocusMode::Far => "Place Mode (Green)",
                    };
                    ui.label(format!("Mode: {}", focus_mode_text));

                    ui.separator();

                    // Selected material
                    ui.label(format!("Material: {}", editor_state.selected_material));
                });
            });
        });
    }));

    if let Err(e) = status_result {
        error!("Status bar rendering panicked: {:?}. Egui context may not be ready.", e);
        return;
    }

    // Help panel on the right side
    let help_result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        egui::SidePanel::right("help_panel")
            .default_width(300.0)
            .show(ctx, |ui| {
            ui.heading("Keyboard Controls");

            ui.separator();

            ui.label("FILE OPERATIONS");
            ui.label("  Ctrl+N: New scene");
            ui.label("  Ctrl+O: Open CSM file");
            ui.label("  Ctrl+S: Save");
            ui.label("  Ctrl+Shift+S: Save As");

            ui.add_space(10.0);

            ui.label("CAMERA CONTROLS");
            ui.label("  Right-click + drag: Rotate camera");
            ui.label("  Scroll wheel: Zoom in/out");
            ui.label("  F: Frame scene (fit view)");
            ui.label("  Home: Reset camera");
            ui.label("  Numpad 1: Front view");
            ui.label("  Numpad 3: Side view");
            ui.label("  Numpad 7: Top view");

            ui.add_space(10.0);

            ui.label("EDITING CONTROLS");
            ui.label("  Left-click: Place voxel(s)");
            ui.label("  Shift+Left-click: Remove voxel(s)");
            ui.label("  Delete: Remove voxel(s)");
            ui.label("  Tab: Toggle Near/Far mode");
            ui.label("  [ / ]: Decrease/Increase cursor size");
            ui.label("  Shift+Scroll: Adjust cursor size");

            ui.add_space(10.0);

            ui.label("MATERIAL SELECTION");
            ui.label("  0-9: Select material 0-9");
            ui.label("    0 = Air (transparent)");
            ui.label("    1-9 = Various materials");

            ui.add_space(10.0);

            ui.label("CURRENT STATUS");
            ui.label(format!("  Focus Mode: {:?}", editor_state.focus_mode));
            ui.label(format!("  Cursor Size: {}", editor_state.cursor.size));
            ui.label(format!("  Material: {}", editor_state.selected_material));
            if let Some(pos) = editor_state.last_paint_position {
                ui.label(format!("  Last Paint: ({}, {}, {})", pos.x, pos.y, pos.z));
            }
        });
    }));

    if let Err(e) = help_result {
        error!("Help panel rendering panicked: {:?}. Egui context may not be ready.", e);
    }
}

/// Plugin for UI systems
pub struct UiPlugin;

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        app
            .init_resource::<UiFrameCounter>()
            // Run UI rendering in Update schedule
            // bevy_egui handles the proper ordering internally
            .add_systems(Update, render_ui);
    }
}
