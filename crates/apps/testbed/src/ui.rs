//! UI rendering module for the physics testbed
//!
//! Contains all egui-based UI rendering code including:
//! - Viewport title labels
//! - Status overlays
//! - Quadrant layout calculations

use crate::{ColliderType, PhysicsScene, PhysicsState};

/// UI constants
pub const TOP_BAR_HEIGHT: f32 = 40.0;

/// Calculate quadrant rectangles for the 2x2 viewport layout
///
/// Returns 4 rectangles in egui coordinates (Y increases downward):
/// - Index 0: Top-left (Cuboid)
/// - Index 1: Top-right (Mesh)
/// - Index 2: Bottom-left (Terrain)
/// - Index 3: Bottom-right (Empty)
pub fn calculate_quadrant_rects(width: f32, height: f32) -> [egui::Rect; 4] {
    let render_height = height - TOP_BAR_HEIGHT;
    let half_width = width / 2.0;
    let half_height = render_height / 2.0;

    [
        // Top-left (Cuboid)
        egui::Rect::from_min_size(
            egui::pos2(0.0, TOP_BAR_HEIGHT),
            egui::vec2(half_width, half_height),
        ),
        // Top-right (Mesh)
        egui::Rect::from_min_size(
            egui::pos2(half_width, TOP_BAR_HEIGHT),
            egui::vec2(half_width, half_height),
        ),
        // Bottom-left (Terrain)
        egui::Rect::from_min_size(
            egui::pos2(0.0, TOP_BAR_HEIGHT + half_height),
            egui::vec2(half_width, half_height),
        ),
        // Bottom-right (Empty)
        egui::Rect::from_min_size(
            egui::pos2(half_width, TOP_BAR_HEIGHT + half_height),
            egui::vec2(half_width, half_height),
        ),
    ]
}

/// Render the viewport title label at the top of a quadrant
pub fn render_viewport_title(
    painter: &egui::Painter,
    collider_type: ColliderType,
    rect: egui::Rect,
) {
    let label = collider_type.label();
    let color = collider_type.color();

    // Position at top-left of the viewport rect with some padding
    let pos = egui::pos2(rect.min.x + 8.0, rect.min.y + 8.0);

    // Draw background for readability
    let text_galley =
        painter.layout_no_wrap(label.to_string(), egui::FontId::proportional(14.0), color);
    let bg_rect = egui::Rect::from_min_size(
        egui::pos2(pos.x - 4.0, pos.y - 2.0),
        egui::vec2(text_galley.size().x + 8.0, text_galley.size().y + 4.0),
    );
    painter.rect_filled(bg_rect, 3.0, egui::Color32::from_black_alpha(200));

    // Draw text
    painter.galley(pos, text_galley, color);
}

/// Format status summary for a scene
pub fn format_status_summary(states: &[PhysicsState]) -> String {
    if states.is_empty() {
        return "No objects".to_string();
    }

    let grounded_count = states.iter().filter(|s| s.is_on_ground).count();

    // Get average Y position
    let avg_y: f32 = states.iter().map(|s| s.falling_position.y).sum::<f32>() / states.len() as f32;

    // Get average velocity
    let avg_vel: f32 =
        states.iter().map(|s| s.falling_velocity.y).sum::<f32>() / states.len() as f32;

    format!(
        "{} obj | {} grounded | avg Y: {:.1} | avg vel: {:.1}",
        states.len(),
        grounded_count,
        avg_y,
        avg_vel
    )
}

/// Render status overlay for a scene at the bottom of its viewport
pub fn render_status_overlay(painter: &egui::Painter, scene: &PhysicsScene, rect: egui::Rect) {
    let status_text = format_status_summary(&scene.states);
    let color = scene.collider_type.color();

    // Position at bottom-left of the viewport rect with some padding
    let pos = egui::pos2(rect.min.x + 8.0, rect.max.y - 22.0);

    // Draw background for readability
    let text_galley = painter.layout_no_wrap(status_text, egui::FontId::monospace(11.0), color);
    let bg_rect = egui::Rect::from_min_size(
        egui::pos2(pos.x - 4.0, pos.y - 2.0),
        egui::vec2(text_galley.size().x + 8.0, text_galley.size().y + 4.0),
    );
    painter.rect_filled(bg_rect, 2.0, egui::Color32::from_black_alpha(180));

    // Draw text
    painter.galley(pos, text_galley, color);
}

/// Render all UI overlays for all scenes
pub fn render_scene_overlays(
    painter: &egui::Painter,
    scenes: &[Option<PhysicsScene>; 4],
    quadrant_rects: &[egui::Rect; 4],
) {
    for (i, scene_opt) in scenes.iter().enumerate() {
        if let Some(scene) = scene_opt {
            // Render title at top
            render_viewport_title(painter, scene.collider_type, quadrant_rects[i]);
            // Render status at bottom
            render_status_overlay(painter, scene, quadrant_rects[i]);
        }
    }
}
