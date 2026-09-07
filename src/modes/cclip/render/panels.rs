//! Cclip selection highlighting with optional tag colors.

use super::super::TagMetadataFormatter;
use crate::ui::DmenuUI;
use ratatui::style::{Color, Modifier, Style};

pub(super) fn highlight_style(
    ui: &DmenuUI<'_>,
    formatter: &TagMetadataFormatter,
    highlight_color: Color,
) -> Style {
    let tag_color = ui.selected.and_then(|selected| {
        if selected < ui.shown.len() {
            ui.shown[selected]
                .tags
                .as_ref()
                .and_then(|tags| tags.first())
                .and_then(|tag| formatter.get_color(tag))
        } else {
            None
        }
    });

    Style::default()
        .fg(tag_color.unwrap_or(highlight_color))
        .add_modifier(Modifier::BOLD)
}
