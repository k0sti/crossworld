//! egui UI panels for the voxel editor
//!
//! Provides UI panels for palette selection, status display, and editor controls.

use egui::{Color32, Rect, Response, Sense, Ui, Vec2};
use std::path::PathBuf;

use crate::cursor::{CubeCursor, FocusMode};
use crate::editing::EditorState;
use crate::palette::{ColorPalette, MaterialPalette, ModelPalette};

// ============================================================================
// File State and Operations
// ============================================================================

/// Tracks the current file state for the editor
#[derive(Debug, Clone)]
pub struct FileState {
    /// Path to the currently open file (None if new/unsaved)
    pub current_file: Option<PathBuf>,
    /// Whether the file has unsaved changes
    pub dirty: bool,
}

impl Default for FileState {
    fn default() -> Self {
        Self::new()
    }
}

impl FileState {
    /// Create a new file state (no file, not dirty)
    pub fn new() -> Self {
        Self {
            current_file: None,
            dirty: false,
        }
    }

    /// Mark the file as having unsaved changes
    pub fn mark_dirty(&mut self) {
        self.dirty = true;
    }

    /// Mark the file as clean (just saved)
    pub fn mark_clean(&mut self) {
        self.dirty = false;
    }

    /// Set the current file path
    pub fn set_file(&mut self, path: PathBuf) {
        self.current_file = Some(path);
        self.dirty = false;
    }

    /// Clear the current file (new file)
    pub fn clear(&mut self) {
        self.current_file = None;
        self.dirty = false;
    }

    /// Get display name for the title bar
    pub fn display_name(&self) -> String {
        let name = self
            .current_file
            .as_ref()
            .and_then(|p| p.file_name())
            .and_then(|n| n.to_str())
            .unwrap_or("Untitled");

        if self.dirty {
            format!("{}*", name)
        } else {
            name.to_string()
        }
    }
}

/// File operations that can be triggered from the UI
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileOperation {
    /// Create a new empty cube
    New,
    /// Open an existing CSM file
    Open,
    /// Save to current file (or Save As if no file)
    Save,
    /// Save to a new file
    SaveAs,
    /// Import a VOX file into model palette
    ImportVox,
}

/// Show the file menu in the top menu bar
///
/// Returns the file operation if one was triggered, or None
///
/// # Arguments
/// * `ui` - The egui UI context
/// * `file_state` - Current file state (for enabling/disabling menu items)
///
/// # Returns
/// Option<FileOperation> - the operation to perform, if any
pub fn show_file_menu(ui: &mut Ui, _file_state: &FileState) -> Option<FileOperation> {
    let mut operation = None;

    ui.menu_button("File", |ui| {
        // New (Ctrl+N)
        if ui
            .add(egui::Button::new("New").shortcut_text("Ctrl+N"))
            .clicked()
        {
            operation = Some(FileOperation::New);
            ui.close();
        }

        ui.separator();

        // Open (Ctrl+O)
        if ui
            .add(egui::Button::new("Open...").shortcut_text("Ctrl+O"))
            .clicked()
        {
            operation = Some(FileOperation::Open);
            ui.close();
        }

        ui.separator();

        // Save (Ctrl+S)
        if ui
            .add(egui::Button::new("Save").shortcut_text("Ctrl+S"))
            .clicked()
        {
            operation = Some(FileOperation::Save);
            ui.close();
        }

        // Save As (Ctrl+Shift+S)
        if ui
            .add(egui::Button::new("Save As...").shortcut_text("Ctrl+Shift+S"))
            .clicked()
        {
            operation = Some(FileOperation::SaveAs);
            ui.close();
        }

        ui.separator();

        // Import VOX
        if ui.button("Import VOX...").clicked() {
            operation = Some(FileOperation::ImportVox);
            ui.close();
        }
    });

    // Handle keyboard shortcuts
    let ctx = ui.ctx();
    let modifiers = ctx.input(|i| i.modifiers);

    if modifiers.ctrl && !modifiers.shift {
        if ctx.input(|i| i.key_pressed(egui::Key::N)) {
            operation = Some(FileOperation::New);
        } else if ctx.input(|i| i.key_pressed(egui::Key::O)) {
            operation = Some(FileOperation::Open);
        } else if ctx.input(|i| i.key_pressed(egui::Key::S)) {
            operation = Some(FileOperation::Save);
        }
    } else if modifiers.ctrl && modifiers.shift {
        if ctx.input(|i| i.key_pressed(egui::Key::S)) {
            operation = Some(FileOperation::SaveAs);
        }
    }

    operation
}

/// Size of each color cell in the palette grid (pixels)
const COLOR_CELL_SIZE: f32 = 16.0;

/// Spacing between color cells (pixels)
const COLOR_CELL_SPACING: f32 = 1.0;

/// Show the R2G3B2 color palette panel
///
/// Displays a grid of 128 colors organized by hue (8 rows of 16 columns).
/// Clicking a color selects it for voxel placement.
///
/// # Arguments
/// * `ui` - The egui UI context
/// * `color_palette` - The color palette state (for selection tracking)
/// * `editor_state` - The editor state (to update selected material)
///
/// # Returns
/// True if a color was selected this frame
pub fn show_color_palette(
    ui: &mut Ui,
    color_palette: &mut ColorPalette,
    editor_state: &mut EditorState,
) -> bool {
    let mut color_selected = false;

    ui.label("Colors (R2G3B2)");
    ui.add_space(4.0);

    // Get colors organized by hue (8 rows × 16 columns)
    let rows = color_palette.rows_by_hue();

    // Draw the palette grid
    for (row_idx, row) in rows.iter().enumerate() {
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing = Vec2::splat(COLOR_CELL_SPACING);
            for (col_idx, palette_color) in row.iter().enumerate() {
                let palette_idx = row_idx * 16 + col_idx;
                let is_selected = palette_idx == color_palette.selected_index();

                // Convert palette color to egui Color32
                let color = Color32::from_rgb(
                    (palette_color.color.x * 255.0) as u8,
                    (palette_color.color.y * 255.0) as u8,
                    (palette_color.color.z * 255.0) as u8,
                );

                // Draw color button
                if color_button(ui, color, is_selected).clicked() {
                    color_palette.select(palette_idx);
                    editor_state.set_material(palette_color.index);
                    color_selected = true;
                }
            }
        });
    }

    ui.add_space(4.0);

    // Show selected color info
    let selected = color_palette.selected();
    ui.horizontal(|ui| {
        let sel_color = Color32::from_rgb(
            (selected.color.x * 255.0) as u8,
            (selected.color.y * 255.0) as u8,
            (selected.color.z * 255.0) as u8,
        );

        // Preview of selected color
        let (rect, _) = ui.allocate_exact_size(Vec2::splat(24.0), Sense::hover());
        ui.painter().rect_filled(rect, 2.0, sel_color);

        ui.vertical(|ui| {
            ui.label(format!("Index: {}", selected.index));
            ui.label(format!(
                "R{} G{} B{}",
                selected.r_bits, selected.g_bits, selected.b_bits
            ));
        });
    });

    color_selected
}

/// Draw a single color button in the palette grid
///
/// # Arguments
/// * `ui` - The egui UI context
/// * `color` - The button color
/// * `is_selected` - Whether this color is currently selected
///
/// # Returns
/// The button response for click detection
fn color_button(ui: &mut Ui, color: Color32, is_selected: bool) -> Response {
    let size = Vec2::splat(COLOR_CELL_SIZE);
    let (rect, response) = ui.allocate_exact_size(size, Sense::click());

    if ui.is_rect_visible(rect) {
        let painter = ui.painter();

        // Draw color fill
        painter.rect_filled(rect, 0.0, color);

        // Draw selection border
        if is_selected {
            // White border for selected color
            painter.rect_stroke(
                rect.expand(1.0),
                0.0,
                egui::Stroke::new(2.0, Color32::WHITE),
                egui::StrokeKind::Inside,
            );
        } else if response.hovered() {
            // Subtle border on hover
            painter.rect_stroke(
                rect,
                0.0,
                egui::Stroke::new(1.0, Color32::from_gray(200)),
                egui::StrokeKind::Inside,
            );
        }
    }

    response
}

/// Show the color palette panel in a side panel
///
/// # Arguments
/// * `ctx` - The egui context
/// * `color_palette` - The color palette state
/// * `editor_state` - The editor state
///
/// # Returns
/// True if a color was selected this frame
pub fn show_color_palette_panel(
    ctx: &egui::Context,
    color_palette: &mut ColorPalette,
    editor_state: &mut EditorState,
) -> bool {
    let mut color_selected = false;

    egui::SidePanel::right("color_palette_panel")
        .resizable(false)
        .min_width(280.0)
        .show(ctx, |ui| {
            ui.heading("Color Palette");
            ui.separator();
            color_selected = show_color_palette(ui, color_palette, editor_state);
        });

    color_selected
}

/// Size of the material color swatch in the list (pixels)
const MATERIAL_SWATCH_SIZE: f32 = 16.0;

/// Show the material palette panel
///
/// Displays a scrollable list of materials from the MATERIAL_REGISTRY (indices 0-127).
/// Each material shows its color swatch and name. Clicking a material selects it.
///
/// # Arguments
/// * `ui` - The egui UI context
/// * `material_palette` - The material palette state (for selection tracking)
/// * `editor_state` - The editor state (to update selected material)
///
/// # Returns
/// True if a material was selected this frame
pub fn show_material_palette(
    ui: &mut Ui,
    material_palette: &mut MaterialPalette,
    editor_state: &mut EditorState,
) -> bool {
    let mut material_selected = false;

    // Show selected material info at the top
    let selected = material_palette.selected();
    ui.horizontal(|ui| {
        let sel_color = Color32::from_rgb(
            (selected.color.x * 255.0) as u8,
            (selected.color.y * 255.0) as u8,
            (selected.color.z * 255.0) as u8,
        );

        // Preview of selected material color
        let (rect, _) = ui.allocate_exact_size(Vec2::splat(24.0), Sense::hover());
        ui.painter().rect_filled(rect, 2.0, sel_color);

        ui.vertical(|ui| {
            ui.label(format!("Selected: {}", format_material_name(selected.id)));
            ui.label(format!("Index: {}", selected.index));
        });
    });

    ui.separator();

    // Scrollable list of materials grouped by category
    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            for category in MaterialPalette::categories() {
                ui.collapsing(format_category_name(category), |ui| {
                    let materials = material_palette.get_category(category);
                    for material in materials {
                        let is_selected = material.index == material_palette.selected_material_index();

                        if material_list_item(ui, material, is_selected).clicked() {
                            material_palette.select_by_material(material.index);
                            editor_state.set_material(material.index);
                            material_selected = true;
                        }
                    }
                });
            }
        });

    material_selected
}

/// Draw a single material item in the list
///
/// Shows a color swatch and the material name.
///
/// # Arguments
/// * `ui` - The egui UI context
/// * `material` - The material to display
/// * `is_selected` - Whether this material is currently selected
///
/// # Returns
/// The response for click detection
fn material_list_item(
    ui: &mut Ui,
    material: &cube::material::Material,
    is_selected: bool,
) -> Response {
    let desired_size = Vec2::new(ui.available_width(), MATERIAL_SWATCH_SIZE + 4.0);
    let (rect, response) = ui.allocate_exact_size(desired_size, Sense::click());

    if ui.is_rect_visible(rect) {
        let painter = ui.painter();
        let visuals = ui.visuals();

        // Background for selected/hovered state
        let bg_color = if is_selected {
            visuals.selection.bg_fill
        } else if response.hovered() {
            visuals.widgets.hovered.bg_fill
        } else {
            Color32::TRANSPARENT
        };

        if bg_color != Color32::TRANSPARENT {
            painter.rect_filled(rect, 2.0, bg_color);
        }

        // Color swatch
        let swatch_rect = Rect::from_min_size(
            rect.min + Vec2::new(4.0, 2.0),
            Vec2::splat(MATERIAL_SWATCH_SIZE),
        );
        let color = Color32::from_rgb(
            (material.color.x * 255.0) as u8,
            (material.color.y * 255.0) as u8,
            (material.color.z * 255.0) as u8,
        );
        painter.rect_filled(swatch_rect, 2.0, color);
        painter.rect_stroke(swatch_rect, 2.0, egui::Stroke::new(1.0, Color32::from_gray(80)), egui::StrokeKind::Inside);

        // Material name
        let text_pos = rect.min + Vec2::new(MATERIAL_SWATCH_SIZE + 12.0, 2.0);
        let text_color = if is_selected {
            visuals.selection.stroke.color
        } else {
            visuals.text_color()
        };
        painter.text(
            text_pos,
            egui::Align2::LEFT_TOP,
            format_material_name(material.id),
            egui::FontId::default(),
            text_color,
        );
    }

    response
}

/// Format a material ID for display
///
/// Converts snake_case IDs to Title Case with spaces.
fn format_material_name(id: &str) -> String {
    id.split('_')
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                None => String::new(),
                Some(first) => first.to_uppercase().chain(chars).collect(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// Format a category name for display
///
/// Capitalizes the first letter.
fn format_category_name(category: &str) -> String {
    let mut chars = category.chars();
    match chars.next() {
        None => String::new(),
        Some(first) => first.to_uppercase().chain(chars).collect(),
    }
}

/// Show the material palette panel in a side panel
///
/// # Arguments
/// * `ctx` - The egui context
/// * `material_palette` - The material palette state
/// * `editor_state` - The editor state
///
/// # Returns
/// True if a material was selected this frame
pub fn show_material_palette_panel(
    ctx: &egui::Context,
    material_palette: &mut MaterialPalette,
    editor_state: &mut EditorState,
) -> bool {
    let mut material_selected = false;

    egui::SidePanel::left("material_palette_panel")
        .resizable(true)
        .min_width(180.0)
        .default_width(200.0)
        .show(ctx, |ui| {
            ui.heading("Materials");
            ui.separator();
            material_selected = show_material_palette(ui, material_palette, editor_state);
        });

    material_selected
}

/// Render the current material color preview in a given rect
pub fn draw_material_preview(ui: &mut Ui, material_index: u8, rect: Rect) {
    use crate::palette::{ColorPalette, MaterialPalette};

    let color = if ColorPalette::is_palette_color(material_index) {
        // R2G3B2 color
        let palette_color = crate::palette::PaletteColor::from_index(material_index);
        Color32::from_rgb(
            (palette_color.color.x * 255.0) as u8,
            (palette_color.color.y * 255.0) as u8,
            (palette_color.color.z * 255.0) as u8,
        )
    } else if MaterialPalette::is_material(material_index) {
        // Material from registry - get color from material
        use cube::material::MATERIAL_REGISTRY;
        let material = &MATERIAL_REGISTRY[material_index as usize];
        Color32::from_rgb(
            (material.color.x * 255.0) as u8,
            (material.color.y * 255.0) as u8,
            (material.color.z * 255.0) as u8,
        )
    } else {
        // Fallback
        Color32::GRAY
    };

    ui.painter().rect_filled(rect, 2.0, color);
}

/// Size of the model thumbnail in the list (pixels)
const MODEL_THUMBNAIL_SIZE: f32 = 32.0;

/// Show the model palette panel
///
/// Displays a scrollable list of loaded VOX models.
/// Each model shows its name and dimensions. Clicking a model selects it.
///
/// # Arguments
/// * `ui` - The egui UI context
/// * `model_palette` - The model palette state (for selection tracking)
///
/// # Returns
/// True if a model was selected this frame
pub fn show_model_palette(ui: &mut Ui, model_palette: &mut ModelPalette) -> bool {
    let mut model_selected = false;

    // Show status info at the top
    ui.horizontal(|ui| {
        ui.label(format!("Models: {}", model_palette.len()));
        if model_palette.is_empty() {
            ui.label("(none loaded)");
        }
    });

    // Show selected model info if one is selected
    if let Some(selected) = model_palette.selected() {
        ui.separator();
        ui.horizontal(|ui| {
            // Model icon placeholder (colored square)
            let (rect, _) = ui.allocate_exact_size(Vec2::splat(24.0), Sense::hover());
            ui.painter().rect_filled(rect, 2.0, Color32::from_rgb(100, 149, 237));

            ui.vertical(|ui| {
                ui.label(format!("Selected: {}", selected.name));
                let size = selected.size();
                ui.label(format!("Size: {}x{}x{}", size.x, size.y, size.z));
            });
        });
    }

    ui.separator();

    // Scrollable list of models
    if model_palette.is_empty() {
        ui.vertical_centered(|ui| {
            ui.add_space(20.0);
            ui.label("No models loaded");
            ui.add_space(8.0);
            ui.label("Use File > Import VOX");
            ui.label("to load models");
            ui.add_space(20.0);
        });
    } else {
        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                let selected_idx = model_palette.selected_index();
                let model_count = model_palette.len();

                // Track which model was clicked
                let mut clicked_idx = None;

                for idx in 0..model_count {
                    if let Some(model) = model_palette.get(idx) {
                        let is_selected = selected_idx == Some(idx);

                        if model_list_item(ui, model, is_selected).clicked() {
                            clicked_idx = Some(idx);
                        }
                    }
                }

                // Apply selection after iteration
                if let Some(idx) = clicked_idx {
                    model_palette.select(idx);
                    model_selected = true;
                }
            });
    }

    model_selected
}

/// Draw a single model item in the list
///
/// Shows a thumbnail placeholder and the model name with dimensions.
///
/// # Arguments
/// * `ui` - The egui UI context
/// * `model` - The model to display
/// * `is_selected` - Whether this model is currently selected
///
/// # Returns
/// The response for click detection
fn model_list_item(
    ui: &mut Ui,
    model: &crate::palette::PaletteModel,
    is_selected: bool,
) -> Response {
    let desired_size = Vec2::new(ui.available_width(), MODEL_THUMBNAIL_SIZE + 8.0);
    let (rect, response) = ui.allocate_exact_size(desired_size, Sense::click());

    if ui.is_rect_visible(rect) {
        let painter = ui.painter();
        let visuals = ui.visuals();

        // Background for selected/hovered state
        let bg_color = if is_selected {
            visuals.selection.bg_fill
        } else if response.hovered() {
            visuals.widgets.hovered.bg_fill
        } else {
            Color32::TRANSPARENT
        };

        if bg_color != Color32::TRANSPARENT {
            painter.rect_filled(rect, 2.0, bg_color);
        }

        // Thumbnail placeholder (colored box representing the model)
        let thumbnail_rect = Rect::from_min_size(
            rect.min + Vec2::new(4.0, 4.0),
            Vec2::splat(MODEL_THUMBNAIL_SIZE),
        );
        // Use a color based on model ID to differentiate models
        let hue = (model.id as f32 * 0.15) % 1.0;
        let (r, g, b) = hsv_to_rgb(hue, 0.6, 0.8);
        painter.rect_filled(
            thumbnail_rect,
            4.0,
            Color32::from_rgb((r * 255.0) as u8, (g * 255.0) as u8, (b * 255.0) as u8),
        );
        painter.rect_stroke(
            thumbnail_rect,
            4.0,
            egui::Stroke::new(1.0, Color32::from_gray(80)),
            egui::StrokeKind::Inside,
        );

        // Model name and dimensions
        let text_x = rect.min.x + MODEL_THUMBNAIL_SIZE + 12.0;
        let text_color = if is_selected {
            visuals.selection.stroke.color
        } else {
            visuals.text_color()
        };

        // Model name
        painter.text(
            egui::Pos2::new(text_x, rect.min.y + 6.0),
            egui::Align2::LEFT_TOP,
            &model.name,
            egui::FontId::default(),
            text_color,
        );

        // Model dimensions (smaller text)
        let size = model.size();
        painter.text(
            egui::Pos2::new(text_x, rect.min.y + 22.0),
            egui::Align2::LEFT_TOP,
            format!("{}x{}x{}", size.x, size.y, size.z),
            egui::FontId::proportional(11.0),
            Color32::from_gray(150),
        );
    }

    response
}

/// Show the model palette panel in a side panel
///
/// # Arguments
/// * `ctx` - The egui context
/// * `model_palette` - The model palette state
///
/// # Returns
/// True if a model was selected this frame
pub fn show_model_palette_panel(ctx: &egui::Context, model_palette: &mut ModelPalette) -> bool {
    let mut model_selected = false;

    egui::SidePanel::left("model_palette_panel")
        .resizable(true)
        .min_width(160.0)
        .default_width(180.0)
        .show(ctx, |ui| {
            ui.heading("Models");
            ui.separator();
            model_selected = show_model_palette(ui, model_palette);
        });

    model_selected
}

/// Status bar information
pub struct StatusBarInfo {
    /// Current FPS
    pub fps: f32,
    /// Whether cursor is valid (has raycast hit)
    pub cursor_valid: bool,
    /// Cursor position (if valid)
    pub cursor_position: Option<glam::Vec3>,
    /// Current cursor mode
    pub cursor_mode: FocusMode,
    /// Currently selected material index
    pub selected_material: u8,
}

impl StatusBarInfo {
    /// Create status bar info from cursor and editor state
    pub fn from_state(cursor: &CubeCursor, editor_state: &EditorState, delta_time: f32) -> Self {
        Self {
            fps: if delta_time > 0.0 {
                1.0 / delta_time
            } else {
                0.0
            },
            cursor_valid: cursor.valid,
            cursor_position: if cursor.valid {
                Some(cursor.position)
            } else {
                None
            },
            cursor_mode: cursor.focus_mode,
            selected_material: editor_state.material(),
        }
    }
}

/// Show the status bar at the bottom of the screen
///
/// Displays cursor state, position, mode, and FPS information.
///
/// # Arguments
/// * `ctx` - The egui context
/// * `info` - Status bar information
pub fn show_status_bar(ctx: &egui::Context, info: &StatusBarInfo) {
    egui::TopBottomPanel::bottom("status_bar")
        .exact_height(24.0)
        .show(ctx, |ui| {
            ui.horizontal_centered(|ui| {
                // Cursor status
                let cursor_status = if info.cursor_valid {
                    "Cursor: Active"
                } else {
                    "Cursor: Inactive"
                };
                let cursor_color = if info.cursor_valid {
                    Color32::from_rgb(100, 200, 100)
                } else {
                    Color32::from_gray(150)
                };
                ui.colored_label(cursor_color, cursor_status);

                ui.separator();

                // Cursor position (if valid)
                if let Some(pos) = info.cursor_position {
                    ui.label(format!("Pos: ({:.0}, {:.0}, {:.0})", pos.x, pos.y, pos.z));
                    ui.separator();
                }

                // Cursor mode
                let mode_text = match info.cursor_mode {
                    FocusMode::Near => "Mode: Near (Remove)",
                    FocusMode::Far => "Mode: Far (Place)",
                };
                let mode_color = match info.cursor_mode {
                    FocusMode::Near => Color32::from_rgb(255, 100, 100),
                    FocusMode::Far => Color32::from_rgb(100, 255, 100),
                };
                ui.colored_label(mode_color, mode_text);

                ui.separator();

                // Selected material
                ui.label(format!("Material: {}", info.selected_material));

                // FPS on the right side
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let fps_color = if info.fps >= 55.0 {
                        Color32::from_rgb(100, 200, 100) // Green for good FPS
                    } else if info.fps >= 30.0 {
                        Color32::from_rgb(200, 200, 100) // Yellow for okay FPS
                    } else {
                        Color32::from_rgb(200, 100, 100) // Red for low FPS
                    };
                    ui.colored_label(fps_color, format!("FPS: {:.0}", info.fps));
                });
            });
        });
}

/// Convert HSV to RGB
///
/// # Arguments
/// * `h` - Hue (0.0-1.0)
/// * `s` - Saturation (0.0-1.0)
/// * `v` - Value (0.0-1.0)
///
/// # Returns
/// (r, g, b) tuple in 0.0-1.0 range
fn hsv_to_rgb(h: f32, s: f32, v: f32) -> (f32, f32, f32) {
    let h = h * 6.0;
    let i = h.floor() as i32;
    let f = h - i as f32;
    let p = v * (1.0 - s);
    let q = v * (1.0 - s * f);
    let t = v * (1.0 - s * (1.0 - f));

    match i % 6 {
        0 => (v, t, p),
        1 => (q, v, p),
        2 => (p, v, t),
        3 => (p, q, v),
        4 => (t, p, v),
        _ => (v, p, q),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_color_cell_constants() {
        // Ensure reasonable values for palette display
        assert!(COLOR_CELL_SIZE > 0.0);
        assert!(COLOR_CELL_SIZE <= 32.0);
        assert!(COLOR_CELL_SPACING >= 0.0);
        assert!(COLOR_CELL_SPACING <= 4.0);
    }

    #[test]
    fn test_material_swatch_constant() {
        // Ensure reasonable value for material swatch size
        assert!(MATERIAL_SWATCH_SIZE > 0.0);
        assert!(MATERIAL_SWATCH_SIZE <= 32.0);
    }

    #[test]
    fn test_format_material_name() {
        assert_eq!(format_material_name("stone"), "Stone");
        assert_eq!(format_material_name("dark_oak"), "Dark Oak");
        assert_eq!(format_material_name("stained_glass_red"), "Stained Glass Red");
        assert_eq!(format_material_name(""), "");
    }

    #[test]
    fn test_format_category_name() {
        assert_eq!(format_category_name("terrain"), "Terrain");
        assert_eq!(format_category_name("wood"), "Wood");
        assert_eq!(format_category_name("gems"), "Gems");
        assert_eq!(format_category_name(""), "");
    }

    #[test]
    fn test_model_thumbnail_constant() {
        // Ensure reasonable value for model thumbnail size
        assert!(MODEL_THUMBNAIL_SIZE > 0.0);
        assert!(MODEL_THUMBNAIL_SIZE <= 64.0);
    }

    #[test]
    fn test_hsv_to_rgb() {
        // Test red (h=0)
        let (r, g, b) = hsv_to_rgb(0.0, 1.0, 1.0);
        assert!((r - 1.0).abs() < 0.01);
        assert!(g < 0.01);
        assert!(b < 0.01);

        // Test green (h=1/3)
        let (r, g, b) = hsv_to_rgb(1.0 / 3.0, 1.0, 1.0);
        assert!(r < 0.01);
        assert!((g - 1.0).abs() < 0.01);
        assert!(b < 0.01);

        // Test blue (h=2/3)
        let (r, g, b) = hsv_to_rgb(2.0 / 3.0, 1.0, 1.0);
        assert!(r < 0.01);
        assert!(g < 0.01);
        assert!((b - 1.0).abs() < 0.01);

        // Test white (s=0, v=1)
        let (r, g, b) = hsv_to_rgb(0.0, 0.0, 1.0);
        assert!((r - 1.0).abs() < 0.01);
        assert!((g - 1.0).abs() < 0.01);
        assert!((b - 1.0).abs() < 0.01);

        // Test black (v=0)
        let (r, g, b) = hsv_to_rgb(0.0, 1.0, 0.0);
        assert!(r < 0.01);
        assert!(g < 0.01);
        assert!(b < 0.01);
    }

    #[test]
    fn test_status_bar_info_default() {
        use glam::Vec3;

        // Create a status bar info with known values
        let info = StatusBarInfo {
            fps: 60.0,
            cursor_valid: true,
            cursor_position: Some(Vec3::new(1.0, 2.0, 3.0)),
            cursor_mode: FocusMode::Far,
            selected_material: 156,
        };

        assert_eq!(info.fps, 60.0);
        assert!(info.cursor_valid);
        assert!(info.cursor_position.is_some());
        assert_eq!(info.cursor_mode, FocusMode::Far);
        assert_eq!(info.selected_material, 156);
    }

    #[test]
    fn test_status_bar_info_from_state() {
        use crate::cursor::CubeCursor;
        use crate::editing::EditorState;

        let cursor = CubeCursor::new();
        let editor_state = EditorState::new();
        let delta_time = 1.0 / 60.0; // 60 FPS

        let info = StatusBarInfo::from_state(&cursor, &editor_state, delta_time);

        // FPS should be approximately 60
        assert!((info.fps - 60.0).abs() < 1.0);
        // Cursor starts invalid
        assert!(!info.cursor_valid);
        assert!(info.cursor_position.is_none());
        // Default mode is Far
        assert_eq!(info.cursor_mode, FocusMode::Far);
    }

    #[test]
    fn test_status_bar_info_zero_delta() {
        use crate::cursor::CubeCursor;
        use crate::editing::EditorState;

        let cursor = CubeCursor::new();
        let editor_state = EditorState::new();
        let delta_time = 0.0; // Edge case

        let info = StatusBarInfo::from_state(&cursor, &editor_state, delta_time);

        // Should handle zero delta gracefully
        assert_eq!(info.fps, 0.0);
    }

    // ========================================================================
    // FileState tests
    // ========================================================================

    #[test]
    fn test_file_state_new() {
        let state = FileState::new();
        assert!(state.current_file.is_none());
        assert!(!state.dirty);
    }

    #[test]
    fn test_file_state_default() {
        let state = FileState::default();
        assert!(state.current_file.is_none());
        assert!(!state.dirty);
    }

    #[test]
    fn test_file_state_mark_dirty() {
        let mut state = FileState::new();
        assert!(!state.dirty);

        state.mark_dirty();
        assert!(state.dirty);
    }

    #[test]
    fn test_file_state_mark_clean() {
        let mut state = FileState::new();
        state.mark_dirty();
        assert!(state.dirty);

        state.mark_clean();
        assert!(!state.dirty);
    }

    #[test]
    fn test_file_state_set_file() {
        let mut state = FileState::new();
        state.mark_dirty();

        let path = PathBuf::from("/tmp/test.csm");
        state.set_file(path.clone());

        assert_eq!(state.current_file, Some(path));
        assert!(!state.dirty); // set_file clears dirty flag
    }

    #[test]
    fn test_file_state_clear() {
        let mut state = FileState::new();
        state.set_file(PathBuf::from("/tmp/test.csm"));
        state.mark_dirty();

        state.clear();

        assert!(state.current_file.is_none());
        assert!(!state.dirty);
    }

    #[test]
    fn test_file_state_display_name_untitled() {
        let state = FileState::new();
        assert_eq!(state.display_name(), "Untitled");
    }

    #[test]
    fn test_file_state_display_name_with_file() {
        let mut state = FileState::new();
        state.set_file(PathBuf::from("/path/to/my_model.csm"));
        assert_eq!(state.display_name(), "my_model.csm");
    }

    #[test]
    fn test_file_state_display_name_dirty() {
        let mut state = FileState::new();
        state.mark_dirty();
        assert_eq!(state.display_name(), "Untitled*");

        state.set_file(PathBuf::from("/path/to/test.csm"));
        state.mark_dirty();
        assert_eq!(state.display_name(), "test.csm*");
    }

    #[test]
    fn test_file_operation_variants() {
        // Ensure all variants are distinct
        assert_ne!(FileOperation::New, FileOperation::Open);
        assert_ne!(FileOperation::Save, FileOperation::SaveAs);
        assert_ne!(FileOperation::SaveAs, FileOperation::ImportVox);

        // Test equality
        assert_eq!(FileOperation::New, FileOperation::New);
        assert_eq!(FileOperation::Open, FileOperation::Open);
        assert_eq!(FileOperation::Save, FileOperation::Save);
        assert_eq!(FileOperation::SaveAs, FileOperation::SaveAs);
        assert_eq!(FileOperation::ImportVox, FileOperation::ImportVox);
    }
}
