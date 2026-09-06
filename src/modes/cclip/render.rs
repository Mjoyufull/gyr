//! Clipboard history, content preview, image, and input rendering.

mod input;
mod panels;

use super::TagMetadataFormatter;
use super::image::ImageRuntime;
use super::state::CclipOptions;
use crate::ui::DmenuUI;
use eyre::Result;
use ratatui::layout::{Alignment, Rect};
use ratatui::widgets::{Clear, List, ListItem, ListState, Paragraph, Wrap};
use ratatui::{Terminal, backend::CrosstermBackend};
use std::io;
use std::io::Write;

pub(super) fn draw(
    terminal: &mut Terminal<CrosstermBackend<io::Stderr>>,
    ui: &mut DmenuUI<'_>,
    options: &CclipOptions,
    tag_metadata_formatter: &TagMetadataFormatter,
    list_state: &mut ListState,
    image_runtime: &mut ImageRuntime,
) -> Result<usize> {
    set_synchronized_output(options.term_is_foot, true);

    let mut max_visible = 0usize;
    let mut render_error = Ok(());
    let needs_sixel_clear = image_runtime.needs_terminal_clear();
    let force_buffer_sync = image_runtime.consume_buffer_sync();

    let draw_result = terminal.draw(|frame| {
        let content_height = options.content_height(frame.area().height);
        let show_content_panel = content_height > 0;
        let layout = options.split_layout(frame.area());
        let chunks = layout.chunks;
        let content_panel_index = layout.content_panel_index;
        let items_panel_index = layout.items_panel_index;
        let input_panel_index = layout.input_panel_index;
        let content_panel_width = chunks[content_panel_index].width;
        let content_panel_height = chunks[content_panel_index].height.saturating_sub(2);

        let preview_enabled = image_runtime.preview_enabled();
        match &ui.tag_mode {
            crate::ui::TagMode::Normal => ui.info_with_image_support(
                options.highlight_color,
                preview_enabled,
                options.hide_image_message,
                content_panel_width,
                content_panel_height,
            ),
            _ => ui.info_with_image_support(
                options.highlight_color,
                false,
                options.hide_image_message,
                content_panel_width,
                content_panel_height,
            ),
        }

        let border_type = panels::border_type(options.rounded_borders);
        let content_paragraph = Paragraph::new(ui.text.clone())
            .block(panels::panel_block(
                " Clipboard Preview ",
                border_type,
                options.header_title_color,
                options.main_border_color,
            ))
            .style(ratatui::style::Style::default().fg(options.main_text_color))
            .alignment(Alignment::Left)
            .wrap(Wrap { trim: false })
            .scroll((0, 0));

        max_visible = chunks[items_panel_index].height.saturating_sub(2) as usize;
        let visible_items = ui
            .shown
            .iter()
            .skip(ui.scroll_offset)
            .take(max_visible)
            .map(|item| item.to_list_item(Some(tag_metadata_formatter)))
            .collect::<Vec<ListItem>>();
        let items_list = List::new(visible_items)
            .block(panels::panel_block(
                " Clipboard History ",
                border_type,
                options.header_title_color,
                options.items_border_color,
            ))
            .style(ratatui::style::Style::default().fg(options.items_text_color))
            .highlight_style(panels::highlight_style(
                ui,
                tag_metadata_formatter,
                options.highlight_color,
            ))
            .highlight_symbol("> ");
        let visible_selection = ui.selected.and_then(|selected| {
            if selected >= ui.scroll_offset && selected < ui.scroll_offset + max_visible {
                Some(selected - ui.scroll_offset)
            } else {
                None
            }
        });
        list_state.select(visible_selection);

        let (input_line, input_title) = input::input_line_and_title(ui, options);
        let text_len = input_line.width();
        let available_width = chunks[input_panel_index].width.saturating_sub(2) as usize;
        let scroll_x = text_len.saturating_sub(available_width) as u16;
        let input_paragraph = Paragraph::new(input_line)
            .block(panels::panel_block(
                input_title,
                border_type,
                options.header_title_color,
                options.input_border_color,
            ))
            .style(ratatui::style::Style::default().fg(options.input_text_color))
            .alignment(Alignment::Left)
            .scroll((0, scroll_x));

        let is_kitty = matches!(options.graphics_adapter, crate::ui::GraphicsAdapter::Kitty);
        if is_kitty || needs_sixel_clear || force_buffer_sync {
            frame.render_widget(Clear, chunks[content_panel_index]);
            frame.render_widget(Clear, chunks[items_panel_index]);
            frame.render_widget(Clear, chunks[input_panel_index]);
        }

        let image_rendered =
            if show_content_panel && preview_enabled && image_runtime.current_is_image() {
                render_inline_image(
                    frame,
                    image_runtime,
                    chunks[content_panel_index],
                    &mut render_error,
                )
            } else {
                false
            };

        if show_content_panel {
            if image_rendered {
                frame.render_widget(
                    panels::panel_block(
                        " Clipboard Preview ",
                        border_type,
                        options.header_title_color,
                        options.main_border_color,
                    ),
                    chunks[content_panel_index],
                );
                render_image_diagnostics(frame, ui, chunks[content_panel_index], options);
            } else {
                frame.render_widget(content_paragraph, chunks[content_panel_index]);
            }
        }

        frame.render_stateful_widget(items_list, chunks[items_panel_index], list_state);
        frame.render_widget(input_paragraph, chunks[input_panel_index]);
    });
    set_synchronized_output(options.term_is_foot, false);
    image_runtime.finish_draw();

    draw_result?;
    render_error?;
    Ok(max_visible)
}

fn render_image_diagnostics(
    frame: &mut ratatui::Frame,
    ui: &DmenuUI<'_>,
    area: Rect,
    options: &CclipOptions,
) {
    let Some(diagnostics) = ui
        .selected
        .and_then(|selected| ui.shown.get(selected))
        .and_then(|item| ui.get_cclip_diagnostics(item))
    else {
        return;
    };
    let inner = area.inner(ratatui::layout::Margin {
        horizontal: 1,
        vertical: 1,
    });
    if inner.height == 0 {
        return;
    }
    frame.render_widget(
        Paragraph::new(diagnostics)
            .style(ratatui::style::Style::default().fg(options.main_text_color)),
        Rect::new(inner.x, inner.y + inner.height - 1, inner.width, 1),
    );
}

fn set_synchronized_output(enabled: bool, enter: bool) {
    if !enabled {
        return;
    }
    let mut stderr = std::io::stderr();
    let sequence = if enter {
        b"\x1b[?2026h"
    } else {
        b"\x1b[?2026l"
    };
    let _ = stderr.write_all(sequence);
    let _ = stderr.flush();
}

fn render_inline_image(
    frame: &mut ratatui::Frame,
    image_runtime: &mut ImageRuntime,
    content_chunk: Rect,
    render_error: &mut Result<()>,
) -> bool {
    let image_area = Rect {
        x: content_chunk.x + 1,
        y: content_chunk.y + 1,
        width: content_chunk.width.saturating_sub(2),
        height: content_chunk.height.saturating_sub(2),
    };
    match image_runtime.render_inline_image(frame, image_area) {
        Ok(rendered) => rendered,
        Err(error) => {
            *render_error = Err(error);
            false
        }
    }
}
