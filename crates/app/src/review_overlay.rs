//! Review overlay rendering with egui
//!
//! Provides a transparent, resizable, scrollable panel for displaying review documents
//! with a text input field for user comments. On exit, the comment is written to stdout.

use egui::{Color32, Context as EguiContext, CornerRadius, Frame, Margin, Stroke};

use crate::runner::ReviewConfig;

/// Exit action from review overlay
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReviewAction {
    /// Continue reviewing
    Continue,
    /// Exit and send comment to stdout
    ExitWithComment,
    /// Exit without sending comment (ESC pressed)
    ExitWithoutComment,
}

/// Render a review overlay panel
///
/// Displays a semi-transparent, resizable, scrollable panel on the right side with
/// markdown-formatted content and a text input field at the bottom for comments.
///
/// Returns the action the user wants to take.
pub fn render_review_overlay(egui_ctx: &EguiContext, review: &mut ReviewConfig) -> ReviewAction {
    let mut action = ReviewAction::Continue;

    // Create a resizable window positioned on the right side
    egui::Window::new("Review")
        .anchor(egui::Align2::RIGHT_TOP, egui::vec2(-16.0, 16.0))
        .default_width(500.0)
        .collapsible(false)
        .resizable(true)
        .title_bar(true)
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
            // Display file name
            let file_name = review
                .file_path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("Unknown");
            ui.heading(file_name);
            ui.separator();

            // Calculate space needed for bottom UI (comment input + button + spacing)
            let bottom_ui_height = 120.0;
            let available_height = ui.available_height() - bottom_ui_height;

            // Scrollable content area
            egui::ScrollArea::vertical()
                .auto_shrink([false; 2])
                .max_height(available_height)
                .show(ui, |ui| {
                    render_markdown(ui, &review.content);
                });

            ui.separator();

            // Comment input area at the bottom
            ui.label(
                egui::RichText::new("Review Comment:").color(Color32::from_rgb(200, 200, 220)),
            );

            let text_edit_response = egui::TextEdit::multiline(&mut review.comment)
                .desired_width(f32::INFINITY)
                .desired_rows(3)
                .hint_text("Enter your review comment here...")
                .show(ui);

            ui.add_space(8.0);

            // Send & Exit button
            if ui
                .button(egui::RichText::new("Send & Exit").size(14.0))
                .clicked()
            {
                action = ReviewAction::ExitWithComment;
            }

            ui.add_space(4.0);

            ui.label(
                egui::RichText::new("Ctrl+Enter to send - ESC to cancel")
                    .small()
                    .color(Color32::from_rgb(150, 150, 160)),
            );

            // Handle Ctrl+Enter to submit
            if text_edit_response.response.has_focus()
                && ui.input(|i| i.key_pressed(egui::Key::Enter) && i.modifiers.ctrl)
            {
                action = ReviewAction::ExitWithComment;
            }

            // Handle ESC to exit without comment
            if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                action = ReviewAction::ExitWithoutComment;
            }
        });

    action
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
                ui.label(RichText::new("  •").text_style(TextStyle::Body));
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
