//! Note overlay rendering with egui
//!
//! Provides a transparent overlay panel that displays markdown-formatted text.
//! Used for displaying annotation notes on top of the application.

use egui::{Color32, Context as EguiContext, CornerRadius, Frame, Margin, Stroke};

/// Render a note overlay with the given text
///
/// Displays a semi-transparent panel in the bottom-left corner with
/// markdown-formatted text.
pub fn render_note_overlay(egui_ctx: &EguiContext, note: &str) {
    // Create a window positioned at bottom-left with transparent background
    egui::Window::new("Note")
        .anchor(egui::Align2::LEFT_BOTTOM, egui::vec2(16.0, -16.0))
        .collapsible(false)
        .resizable(false)
        .title_bar(false)
        .frame(
            Frame::default()
                .fill(Color32::from_rgba_unmultiplied(20, 20, 30, 220))
                .stroke(Stroke::new(
                    1.0,
                    Color32::from_rgba_unmultiplied(100, 100, 120, 180),
                ))
                .corner_radius(CornerRadius::same(8))
                .inner_margin(Margin::same(16)),
        )
        .show(egui_ctx, |ui| {
            // Set max width for the note panel
            ui.set_max_width(400.0);

            // Render markdown-style text
            render_markdown(ui, note);
        });
}

/// Render markdown-formatted text in the UI
///
/// Supports basic markdown:
/// - **bold** text
/// - *italic* text
/// - `code` inline
/// - # Headings (1-3 levels)
/// - - bullet lists
/// - newlines preserved
fn render_markdown(ui: &mut egui::Ui, text: &str) {
    use egui::{FontId, RichText, TextStyle};

    let lines: Vec<&str> = text.lines().collect();

    for line in lines {
        let trimmed = line.trim();

        if trimmed.is_empty() {
            ui.add_space(8.0);
            continue;
        }

        // Handle headings
        if let Some(heading) = trimmed.strip_prefix("### ") {
            ui.label(
                RichText::new(heading)
                    .font(FontId::proportional(14.0))
                    .strong()
                    .color(Color32::from_rgb(180, 180, 200)),
            );
            ui.add_space(4.0);
            continue;
        }
        if let Some(heading) = trimmed.strip_prefix("## ") {
            ui.label(
                RichText::new(heading)
                    .font(FontId::proportional(16.0))
                    .strong()
                    .color(Color32::from_rgb(200, 200, 220)),
            );
            ui.add_space(6.0);
            continue;
        }
        if let Some(heading) = trimmed.strip_prefix("# ") {
            ui.label(
                RichText::new(heading)
                    .font(FontId::proportional(18.0))
                    .strong()
                    .color(Color32::from_rgb(220, 220, 240)),
            );
            ui.add_space(8.0);
            continue;
        }

        // Handle bullet points
        if let Some(bullet_text) = trimmed.strip_prefix("- ") {
            ui.horizontal(|ui| {
                ui.label(RichText::new("  ").text_style(TextStyle::Body));
                render_inline_markdown(ui, bullet_text);
            });
            continue;
        }

        // Regular text with inline formatting
        render_inline_markdown(ui, trimmed);
    }
}

/// Render inline markdown formatting (bold, italic, code)
fn render_inline_markdown(ui: &mut egui::Ui, text: &str) {
    use egui::Color32;

    let mut job = egui::text::LayoutJob::default();
    let mut chars = text.chars().peekable();
    let mut current_text = String::new();

    let default_format = egui::TextFormat {
        color: Color32::from_rgb(220, 220, 230),
        ..Default::default()
    };

    let bold_format = egui::TextFormat {
        color: Color32::from_rgb(255, 255, 255),
        ..Default::default()
    };

    let italic_format = egui::TextFormat {
        color: Color32::from_rgb(200, 200, 220),
        italics: true,
        ..Default::default()
    };

    let code_format = egui::TextFormat {
        color: Color32::from_rgb(180, 220, 180),
        background: Color32::from_rgba_unmultiplied(60, 60, 80, 150),
        ..Default::default()
    };

    while let Some(c) = chars.next() {
        match c {
            '*' => {
                // Check for bold (**) vs italic (*)
                if chars.peek() == Some(&'*') {
                    chars.next(); // consume second *
                                  // Flush current text
                    if !current_text.is_empty() {
                        job.append(&current_text, 0.0, default_format.clone());
                        current_text.clear();
                    }
                    // Collect bold text
                    let mut bold_text = String::new();
                    while let Some(&next_c) = chars.peek() {
                        if next_c == '*' {
                            chars.next();
                            if chars.peek() == Some(&'*') {
                                chars.next();
                                break;
                            } else {
                                bold_text.push('*');
                            }
                        } else {
                            bold_text.push(chars.next().unwrap());
                        }
                    }
                    job.append(&bold_text, 0.0, bold_format.clone());
                } else {
                    // Italic
                    if !current_text.is_empty() {
                        job.append(&current_text, 0.0, default_format.clone());
                        current_text.clear();
                    }
                    let mut italic_text = String::new();
                    while let Some(&next_c) = chars.peek() {
                        if next_c == '*' {
                            chars.next();
                            break;
                        } else {
                            italic_text.push(chars.next().unwrap());
                        }
                    }
                    job.append(&italic_text, 0.0, italic_format.clone());
                }
            }
            '`' => {
                // Inline code
                if !current_text.is_empty() {
                    job.append(&current_text, 0.0, default_format.clone());
                    current_text.clear();
                }
                let mut code_text = String::new();
                while let Some(&next_c) = chars.peek() {
                    if next_c == '`' {
                        chars.next();
                        break;
                    } else {
                        code_text.push(chars.next().unwrap());
                    }
                }
                job.append(&code_text, 0.0, code_format.clone());
            }
            _ => {
                current_text.push(c);
            }
        }
    }

    // Flush remaining text
    if !current_text.is_empty() {
        job.append(&current_text, 0.0, default_format);
    }

    ui.label(job);
}
