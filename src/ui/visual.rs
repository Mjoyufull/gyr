//! Shared visual primitives for the launcher, dmenu, and clipboard selector.

use ratatui::Frame;
use ratatui::layout::{Margin, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::Span;
use ratatui::widgets::{Block, BorderType, Borders, Paragraph};

#[derive(Clone, Copy)]
pub(crate) struct PanelTheme {
    pub(crate) show_border: bool,
    pub(crate) show_title: bool,
    pub(crate) bold_title: bool,
    pub(crate) rounded_border: bool,
    pub(crate) border_color: Color,
    pub(crate) background_color: Color,
    pub(crate) title_color: Color,
}

pub(crate) fn panel_block(title: &str, theme: PanelTheme) -> Block<'_> {
    let mut block = Block::default()
        .borders(if theme.show_border {
            Borders::ALL
        } else {
            Borders::NONE
        })
        .style(Style::default().bg(theme.background_color))
        .border_style(Style::default().fg(theme.border_color))
        .border_type(if theme.rounded_border {
            BorderType::Rounded
        } else {
            BorderType::Plain
        });
    if theme.show_title {
        let mut style = Style::default().fg(theme.title_color);
        if theme.bold_title {
            style = style.add_modifier(Modifier::BOLD);
        }
        block = block.title(Span::styled(title, style));
    }
    block
}

pub(crate) fn selection_content_area(area: Rect, rounded: bool) -> Rect {
    if rounded && area.width > 2 {
        area.inner(Margin {
            horizontal: 1,
            vertical: 0,
        })
    } else {
        area
    }
}

pub(crate) fn render_selection_background(
    frame: &mut Frame,
    area: Rect,
    panel_color: Color,
    selection_color: Color,
    rounded: bool,
) {
    if !rounded || area.width < 3 {
        frame.render_widget(
            Block::default().style(Style::default().bg(selection_color)),
            area,
        );
        return;
    }

    frame.render_widget(
        Block::default().style(Style::default().bg(selection_color)),
        Rect::new(area.x + 1, area.y, area.width - 2, area.height),
    );
    let cap_style = Style::default().fg(selection_color).bg(panel_color);
    for y in area.y..area.y + area.height {
        frame.render_widget(
            Paragraph::new("▐").style(cap_style),
            Rect::new(area.x, y, 1, 1),
        );
        frame.render_widget(
            Paragraph::new("▌").style(cap_style),
            Rect::new(area.x + area.width - 1, y, 1, 1),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::selection_content_area;
    use ratatui::layout::Rect;

    #[test]
    fn rounded_selection_reserves_its_caps() {
        assert_eq!(
            selection_content_area(Rect::new(2, 4, 20, 3), true),
            Rect::new(3, 4, 18, 3)
        );
        assert_eq!(
            selection_content_area(Rect::new(2, 4, 2, 3), true),
            Rect::new(2, 4, 2, 3)
        );
    }
}
