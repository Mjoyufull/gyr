//! Content-panel rendering for dmenu and cclip selections.

mod cclip;
mod lines;

use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};

use super::{DmenuUI, tag_mode::tag_mode_lines};

impl<'a> DmenuUI<'a> {
    /// Update `self.text` to show content for current selection.
    pub fn info(&mut self, color: Color) {
        let panel_width = crossterm::terminal::size()
            .map(|(width, _)| width)
            .unwrap_or(0);
        self.info_with_image_support(color, false, false, panel_width, 0);
    }

    /// Update `self.text` to show content with optional image preview support.
    pub fn info_with_image_support(
        &mut self,
        highlight_color: Color,
        enable_images: bool,
        hide_image_message: bool,
        panel_width: u16,
        panel_height: u16,
    ) {
        self.drain_cclip_content_results();
        if let Some(text) =
            tag_mode_lines(&self.tag_mode, self.temp_message_text(), highlight_color)
        {
            self.text = text;
            return;
        }

        let Some(selected) = self.selected else {
            self.text.clear();
            return;
        };
        if selected >= self.shown.len() {
            self.text.clear();
            return;
        }

        let item = self.shown[selected].clone();
        if enable_images && self.is_cclip_image_item(&item) {
            self.text = if hide_image_message {
                vec![Line::from(Span::raw(String::new()))]
            } else {
                vec![
                    Line::from(vec![
                        Span::styled(
                            "󰋩 IMAGE PREVIEW ",
                            Style::default().add_modifier(Modifier::BOLD),
                        ),
                        image_status_span(),
                    ]),
                    Line::from(""),
                    Line::from("  󱇛 Press 'Alt+i' for Fullscreen View"),
                    Line::from("  󰆏 Press 'Enter' to Copy to Clipboard"),
                    Line::from(""),
                    Line::from(self.get_image_info(&item)),
                ]
            };
            if let Some(diagnostics) = self.get_cclip_diagnostics(&item) {
                self.text.push(Line::from(diagnostics));
            }
            return;
        }

        let content = if self.is_cclip_item(&item) {
            self.get_cclip_content_for_display(&item)
        } else {
            item.get_content_display()
        };
        let display_content =
            lines::normalize_display_content(content, self.show_line_numbers, item.line_number);
        let mut content_lines =
            lines::build_content_lines(&display_content, self.wrap_long_lines, panel_width);
        if content_lines.is_empty() {
            content_lines.push(Line::from(Span::raw("[No content]")));
        }
        lines::pad_lines_to_height(&mut content_lines, panel_width, panel_height);
        self.text = content_lines;
    }
}

fn image_status_span() -> Span<'static> {
    let mut status_span = Span::styled("- Loading...", Style::default().fg(Color::Yellow));
    if let Ok(state) = crate::ui::DISPLAY_STATE.try_lock() {
        match &*state {
            crate::ui::DisplayState::Failed(message) => {
                status_span = Span::styled(
                    format!("- Failed: {}", message),
                    Style::default().fg(Color::Red),
                );
            }
            crate::ui::DisplayState::Image(_) => {
                status_span = Span::styled("- Ready", Style::default().fg(Color::Green));
            }
            _ => {}
        }
    }
    status_span
}
