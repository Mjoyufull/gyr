use eyre::Result;
use ratatui::Frame;
use ratatui::layout::{Alignment, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{
    Block, BorderType, Borders, Clear, List, ListItem, ListState, Paragraph, Wrap,
};

use crate::ui::DmenuUI;

use super::options::DmenuOptions;
use super::preview::PreviewRuntime;

pub(super) fn draw_frame(
    frame: &mut Frame,
    ui: &mut DmenuUI,
    list_state: &mut ListState,
    options: &DmenuOptions,
    preview: &mut PreviewRuntime,
) -> Result<()> {
    let layout = options.split_layout(frame.area());
    let chunks = layout.chunks;
    let content_panel_index = layout.content_panel_index;
    let items_panel_index = layout.items_panel_index;
    let input_panel_index = layout.input_panel_index;
    let show_content_panel = options.content_height(frame.area().height) > 0;

    let border_type = if options.rounded_borders {
        BorderType::Rounded
    } else {
        BorderType::Plain
    };

    let content_block = Block::default()
        .borders(Borders::ALL)
        .title(Span::styled(
            preview.title(),
            Style::default()
                .add_modifier(Modifier::BOLD)
                .fg(options.header_title_color),
        ))
        .border_type(border_type)
        .border_style(Style::default().fg(options.main_border_color));

    let default_content_lines = if preview.is_enabled() {
        None
    } else {
        ui.info_with_image_support(
            options.highlight_color,
            false,
            false,
            chunks[content_panel_index].width,
            chunks[content_panel_index].height.saturating_sub(2),
        );
        Some(ui.text.clone())
    };

    let items_panel_height = chunks[items_panel_index].height;
    let max_visible = items_panel_height.saturating_sub(2) as usize;
    let visible_items = ui
        .shown
        .iter()
        .skip(ui.scroll_offset)
        .take(max_visible)
        .map(ListItem::from)
        .collect::<Vec<ListItem>>();

    let items_list = List::new(visible_items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(Span::styled(
                    " Items ",
                    Style::default()
                        .add_modifier(Modifier::BOLD)
                        .fg(options.header_title_color),
                ))
                .border_type(border_type)
                .border_style(Style::default().fg(options.items_border_color)),
        )
        .style(Style::default().fg(options.items_text_color))
        .highlight_style(
            Style::default()
                .fg(options.highlight_color)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("> ");

    let visible_selection = ui.selected.and_then(|selected| {
        if selected >= ui.scroll_offset && selected < ui.scroll_offset + max_visible {
            Some(selected - ui.scroll_offset)
        } else {
            None
        }
    });
    list_state.select(visible_selection);

    let input_line = Line::from(vec![
        Span::styled("(", Style::default().fg(options.input_text_color)),
        Span::styled(
            (ui.selected.map_or(0, |index| index + 1)).to_string(),
            Style::default().fg(options.highlight_color),
        ),
        Span::styled("/", Style::default().fg(options.input_text_color)),
        Span::styled(
            ui.shown.len().to_string(),
            Style::default().fg(options.input_text_color),
        ),
        Span::styled(") ", Style::default().fg(options.input_text_color)),
        Span::styled(">", Style::default().fg(options.highlight_color)),
        Span::styled("> ", Style::default().fg(options.input_text_color)),
        Span::styled(
            options.display_query(&ui.query),
            Style::default().fg(options.input_text_color),
        ),
        Span::styled(
            &options.cursor,
            Style::default().fg(options.highlight_color),
        ),
    ]);

    let text_len = input_line.width();
    let available_width = chunks[input_panel_index].width.saturating_sub(2) as usize;
    let scroll_x = if text_len > available_width {
        (text_len - available_width) as u16
    } else {
        0
    };

    let input_paragraph = Paragraph::new(input_line)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(Span::styled(
                    options.input_title(),
                    Style::default()
                        .add_modifier(Modifier::BOLD)
                        .fg(options.header_title_color),
                ))
                .border_type(border_type)
                .border_style(Style::default().fg(options.input_border_color)),
        )
        .style(Style::default().fg(options.input_text_color))
        .alignment(Alignment::Left)
        .scroll((0, scroll_x));

    if matches!(options.graphics_adapter, crate::ui::GraphicsAdapter::Kitty) {
        if show_content_panel && (!options.hide_before_typing || !ui.query.is_empty()) {
            frame.render_widget(Clear, chunks[content_panel_index]);
        }
        if !options.prompt_only && (!options.hide_before_typing || !ui.query.is_empty()) {
            frame.render_widget(Clear, chunks[items_panel_index]);
        }
        frame.render_widget(Clear, chunks[input_panel_index]);
    }

    if show_content_panel && (!options.hide_before_typing || !ui.query.is_empty()) {
        let content_area = chunks[content_panel_index];
        let image_area = Rect {
            x: content_area.x.saturating_add(1),
            y: content_area.y.saturating_add(1),
            width: content_area.width.saturating_sub(2),
            height: content_area.height.saturating_sub(2),
        };
        if preview.render_image(frame, image_area)? {
            frame.render_widget(content_block, content_area);
        } else {
            let content_lines = if preview.is_enabled() {
                preview.text_lines().unwrap_or_default()
            } else {
                default_content_lines.unwrap_or_default()
            };
            let content_paragraph = Paragraph::new(content_lines)
                .block(content_block)
                .style(Style::default().fg(options.main_text_color))
                .wrap(Wrap { trim: false })
                .alignment(Alignment::Left);
            frame.render_widget(content_paragraph, content_area);
        }
    }
    if !options.prompt_only && (!options.hide_before_typing || !ui.query.is_empty()) {
        frame.render_stateful_widget(items_list, chunks[items_panel_index], list_state);
    }
    frame.render_widget(input_paragraph, chunks[input_panel_index]);
    Ok(())
}
