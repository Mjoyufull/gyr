//! Dmenu rendering for shared panels, text previews, and image previews.

use eyre::Result;
use ratatui::Frame;
use ratatui::layout::Alignment;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Clear, List, ListItem, ListState, Paragraph, Wrap};

use crate::ui::DmenuUI;

use super::options::DmenuOptions;
use super::panels::PreviewPanels;

pub(super) fn draw_frame(
    frame: &mut Frame,
    ui: &mut DmenuUI,
    list_state: &mut ListState,
    options: &DmenuOptions,
    previews: &mut PreviewPanels,
) -> Result<()> {
    let preview = &mut previews.primary;
    let (layout, custom_areas) = options.split_all(frame.area());
    let chunks = layout.chunks;
    let content_panel_index = layout.content_panel_index;
    let items_panel_index = layout.items_panel_index;
    let input_panel_index = layout.input_panel_index;
    let show_content_panel = !chunks[content_panel_index].is_empty();

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
    let content_block = crate::ui::panel_block(preview.title(), content_theme);
    let items_block = crate::ui::panel_block(" Items ", items_theme);

    let default_content_lines = if preview.is_enabled() {
        None
    } else {
        ui.info_with_image_support(
            options.highlight_color,
            false,
            false,
            content_block.inner(chunks[content_panel_index]).width,
            content_block.inner(chunks[content_panel_index]).height,
        );
        Some(ui.text.clone())
    };

    let result_layout = options.result_layout(frame.area());
    result_layout.keep_visible(ui.selected, &mut ui.scroll_offset);
    let max_visible = result_layout.capacity();
    let visible_items = ui
        .shown
        .iter()
        .skip(ui.scroll_offset)
        .take(max_visible)
        .map(ListItem::from)
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
        let image_area = crate::ui::panel_block(preview.title(), content_theme).inner(content_area);
        frame.render_widget(
            crate::ui::panel_block(preview.title(), content_theme),
            content_area,
        );
        if !preview.render_image(frame, image_area)? {
            let content_lines = if preview.is_enabled() {
                preview.text_lines().unwrap_or_default()
            } else {
                default_content_lines.unwrap_or_default()
            };
            let content_paragraph = Paragraph::new(content_lines)
                .style(Style::default().fg(options.main_text_color))
                .style(Style::default().bg(options.main_background_color))
                .wrap(Wrap { trim: false })
                .alignment(Alignment::Left);
            frame.render_widget(content_paragraph, image_area);
        }
    }
    if !options.prompt_only && (!options.hide_before_typing || !ui.query.is_empty()) {
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
            let list = List::new([item])
                .style(Style::default().fg(options.items_text_color))
                .highlight_style(
                    Style::default()
                        .fg(options.highlight_color)
                        .add_modifier(Modifier::BOLD),
                )
                .highlight_symbol(marker.as_str())
                .highlight_spacing(ratatui::widgets::HighlightSpacing::Always);
            let mut item_state = ListState::default();
            item_state.select(selected.then_some(0));
            frame.render_stateful_widget(
                list,
                crate::ui::selection_content_area(slot, options.items_selection_rounded),
                &mut item_state,
            );
        }
    }
    let selected_name = ui
        .selected
        .and_then(|selected| ui.shown.get(selected))
        .map_or("fsel", |item| item.display_text.as_str());
    crate::ui::render_input_panel(
        frame,
        chunks[input_panel_index],
        crate::ui::InputPanelData {
            title: options.input_title(),
            classic_line: input_line(ui, options, true),
            command_line: input_line(ui, options, false),
            selected_name,
            selected_count: ui.selected.map_or(0, |index| index + 1),
            total_count: ui.shown.len(),
            primary_action: "select",
            exit_action: "close",
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
    if !options.hide_before_typing || !ui.query.is_empty() {
        for ((spec, runtime), area) in options
            .custom_panels
            .iter()
            .zip(&mut previews.custom)
            .zip(custom_areas)
        {
            if area.is_empty() {
                continue;
            }
            let title = format!(" {} ", spec.name);
            let block = crate::ui::panel_block(&title, content_theme);
            let inner = block.inner(area);
            frame.render_widget(block, area);
            if !runtime.render_image(frame, inner)? {
                frame.render_widget(
                    Paragraph::new(runtime.text_lines().unwrap_or_default())
                        .style(
                            Style::default()
                                .fg(options.main_text_color)
                                .bg(options.main_background_color),
                        )
                        .wrap(Wrap { trim: false }),
                    inner,
                );
            }
        }
    }
    Ok(())
}

fn input_line(ui: &DmenuUI<'_>, options: &DmenuOptions, include_count: bool) -> Line<'static> {
    let mut spans = Vec::new();
    if include_count && options.show_input_count {
        spans.extend([
            Span::styled("(", Style::default().fg(options.input_text_color)),
            Span::styled(
                ui.selected.map_or(0, |index| index + 1).to_string(),
                Style::default().fg(options.highlight_color),
            ),
            Span::styled("/", Style::default().fg(options.input_text_color)),
            Span::styled(
                ui.shown.len().to_string(),
                Style::default().fg(options.input_text_color),
            ),
            Span::styled(") ", Style::default().fg(options.input_text_color)),
        ]);
    }
    if options.show_input_prompt {
        spans.extend([
            Span::styled(">", Style::default().fg(options.highlight_color)),
            Span::styled("> ", Style::default().fg(options.input_text_color)),
        ]);
    }
    spans.extend([
        Span::styled(
            options.display_query(&ui.query),
            Style::default().fg(options.input_text_color),
        ),
        Span::styled(
            options.cursor.clone(),
            Style::default().fg(options.highlight_color),
        ),
    ]);
    Line::from(spans)
}
