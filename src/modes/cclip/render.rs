//! Clipboard selector rendering and protocol-specific image placement.

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
        let layout = options.split_layout(frame.area());
        let chunks = layout.chunks;
        let content_panel_index = layout.content_panel_index;
        let show_content_panel = !chunks[content_panel_index].is_empty();
        let items_panel_index = layout.items_panel_index;
        let input_panel_index = layout.input_panel_index;
        let content_theme = crate::ui::PanelTheme {
            show_border: options.show_main_border,
            show_title: options.show_panel_titles,
            bold_title: true,
            rounded_border: options.rounded_borders,
            border_color: options.main_border_color,
            background_color: options.main_background_color,
            title_color: options.header_title_color,
        };
        let items_theme = crate::ui::PanelTheme {
            show_border: options.show_items_border,
            show_title: options.show_panel_titles,
            bold_title: true,
            rounded_border: options.rounded_borders,
            border_color: options.items_border_color,
            background_color: options.items_background_color,
            title_color: options.header_title_color,
        };
        let content_block = crate::ui::panel_block(" Clipboard Preview ", content_theme);
        let items_block = crate::ui::panel_block(" Clipboard History ", items_theme);
        let content_inner = content_block.inner(chunks[content_panel_index]);

        let preview_enabled = image_runtime.preview_enabled();
        match &ui.tag_mode {
            crate::ui::TagMode::Normal => ui.info_with_image_support(
                options.highlight_color,
                preview_enabled,
                options.hide_image_message,
                content_inner.width,
                content_inner.height,
            ),
            _ => ui.info_with_image_support(
                options.highlight_color,
                false,
                options.hide_image_message,
                content_inner.width,
                content_inner.height,
            ),
        }

        let content_paragraph = Paragraph::new(ui.text.clone())
            .style(
                ratatui::style::Style::default()
                    .fg(options.main_text_color)
                    .bg(options.main_background_color),
            )
            .alignment(Alignment::Left)
            .wrap(Wrap { trim: false })
            .scroll((0, 0));

        let result_layout = options.result_layout(frame.area());
        result_layout.keep_visible(ui.selected, &mut ui.scroll_offset);
        max_visible = result_layout.capacity();
        let visible_items = ui
            .shown
            .iter()
            .skip(ui.scroll_offset)
            .take(max_visible)
            .map(|item| item.to_list_item(Some(tag_metadata_formatter)))
            .collect::<Vec<ListItem>>();

        let marker = if options.show_selection_marker && !options.selection_marker.is_empty() {
            format!("{} ", options.selection_marker)
        } else {
            String::new()
        };
        let visible_selection = ui.selected.and_then(|selected| {
            if selected >= ui.scroll_offset && selected < ui.scroll_offset + max_visible {
                Some(selected - ui.scroll_offset)
            } else {
                None
            }
        });
        list_state.select(visible_selection);

        let is_kitty = matches!(options.graphics_adapter, crate::ui::GraphicsAdapter::Kitty);
        if is_kitty || needs_sixel_clear || force_buffer_sync {
            frame.render_widget(Clear, chunks[content_panel_index]);
            frame.render_widget(Clear, chunks[items_panel_index]);
            frame.render_widget(Clear, chunks[input_panel_index]);
        }

        if show_content_panel {
            frame.render_widget(content_block, chunks[content_panel_index]);
        }
        let image_rendered =
            if show_content_panel && preview_enabled && image_runtime.current_is_image() {
                render_inline_image(frame, image_runtime, content_inner, &mut render_error)
            } else {
                false
            };

        if show_content_panel && !image_rendered {
            frame.render_widget(content_paragraph, content_inner);
        }

        frame.render_widget(items_block, chunks[items_panel_index]);
        for (index, item) in visible_items.into_iter().enumerate() {
            let slot = result_layout.slot(index);
            let selected = visible_selection == Some(index);
            if selected {
                crate::ui::render_selection_background(
                    frame,
                    slot,
                    options.items_background_color,
                    options.items_selection_background_color,
                    options.items_selection_rounded,
                );
            }
            let item_area =
                crate::ui::selection_content_area(slot, options.items_selection_rounded);
            let list = List::new([item])
                .style(ratatui::style::Style::default().fg(options.items_text_color))
                .highlight_style(panels::highlight_style(
                    ui,
                    tag_metadata_formatter,
                    options.highlight_color,
                ))
                .highlight_symbol(marker.as_str())
                .highlight_spacing(ratatui::widgets::HighlightSpacing::Always);
            let mut item_state = ListState::default();
            item_state.select(selected.then_some(0));
            frame.render_stateful_widget(list, item_area, &mut item_state);
        }

        let input_lines = input::input_lines(ui, options);
        let selected_name = ui
            .selected
            .and_then(|selected| ui.shown.get(selected))
            .map_or("fsel", |item| item.display_text.as_str());
        crate::ui::render_input_panel(
            frame,
            chunks[input_panel_index],
            crate::ui::InputPanelData {
                title: input_lines.title,
                classic_line: input_lines.classic,
                command_line: input_lines.command,
                selected_name,
                selected_count: ui.selected.map_or(0, |selected| selected + 1),
                total_count: ui.shown.len(),
                primary_action: input_lines.primary_action,
                exit_action: input_lines.exit_action,
            },
            crate::ui::InputPanelTheme {
                style: options.input_panel_style,
                show_border: options.show_input_border,
                show_title: options.show_panel_titles,
                bold_title: true,
                show_count: options.show_input_count,
                rounded_border: options.rounded_borders,
                border_color: options.input_border_color,
                background_color: options.input_background_color,
                text_color: options.input_text_color,
                highlight_color: options.highlight_color,
                title_color: options.header_title_color,
                keybinds: &options.keybinds,
            },
        );
    });
    set_synchronized_output(options.term_is_foot, false);
    image_runtime.finish_draw();

    draw_result?;
    render_error?;
    Ok(max_visible)
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
    image_area: Rect,
    render_error: &mut Result<()>,
) -> bool {
    match image_runtime.render_inline_image(frame, image_area) {
        Ok(rendered) => rendered,
        Err(error) => {
            *render_error = Err(error);
            false
        }
    }
}
