//! Launcher result-list layout, backgrounds, markers, and terminal images.

use super::app_ui::AppIcons;
use crate::cli::Opts;
use crate::core::state::State;
use eyre::Result;
use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Paragraph};
use unicode_width::UnicodeWidthStr;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ListAreas {
    text: Rect,
    icon: Option<Rect>,
    selection: Option<Rect>,
}

pub(crate) fn launcher_visible_rows(size: Rect, cli: &Opts) -> usize {
    launcher_result_layout(size, cli).capacity()
}

pub(crate) fn launcher_result_layout(size: Rect, cli: &Opts) -> super::result_layout::ResultLayout {
    result_layout(launcher_list_content_area(size, cli), cli)
}

fn result_layout(area: Rect, cli: &Opts) -> super::result_layout::ResultLayout {
    if cli.app_grid_columns > 0 {
        return super::result_layout::ResultLayout::grid(
            area,
            app_row_height(cli),
            cli.app_grid_columns,
            &cli.panels,
        );
    }
    super::result_layout::ResultLayout::new(area, app_row_height(cli), &cli.panels)
}

pub(crate) fn launcher_list_content_area(size: Rect, cli: &Opts) -> Rect {
    let (_, _, items_area) = super::app_ui::launcher_panel_areas(size, cli);
    apps_block(cli).inner(items_area)
}

pub(crate) fn app_row_height(cli: &Opts) -> u16 {
    if cli.app_grid_columns > 0 {
        cli.app_grid_row_height.max(2)
    } else if cli.desktop_icon_mode.shows_list() {
        cli.desktop_icon_list_height.max(1)
    } else {
        1
    }
}

pub(crate) fn launcher_list_icon_area(size: Rect, cli: &Opts) -> Rect {
    let inner = launcher_list_content_area(size, cli);
    let slot = result_layout(inner, cli).slot(0);
    let content = list_content_area(slot, cli);
    let Some(icon_strip) = list_areas(content, cli).icon else {
        return Rect::default();
    };
    Rect::new(0, 0, icon_strip.width, icon_strip.height)
}

pub(super) fn render(
    frame: &mut Frame,
    state: &State,
    cli: &Opts,
    area: Rect,
    mut app_icons: Option<&mut AppIcons<'_>>,
) -> Result<bool> {
    let block = apps_block(cli);
    let inner = block.inner(area);
    frame.render_widget(block, area);
    let layout = result_layout(inner, cli);
    let mut render_failed = false;
    for (index, app) in state
        .shown
        .iter()
        .skip(state.scroll_offset)
        .take(layout.capacity())
        .enumerate()
    {
        let slot = layout.slot(index);
        let selected = state.selected == Some(state.scroll_offset + index);
        let (style, background, selection_background) = row_style(cli, app.pinned, selected);
        frame.render_widget(
            Block::default().style(Style::default().bg(background)),
            slot,
        );
        if selected {
            super::render_selection_background(
                frame,
                slot,
                background,
                selection_background,
                cli.items_selection_rounded,
            );
        }
        let areas = list_areas(list_content_area(slot, cli), cli);
        let mut spans = Vec::new();
        if areas.selection.is_none() {
            spans.extend(selection_marker_spans(cli, selected));
        }
        if app.pinned && cli.show_pin_icons {
            spans.push(Span::styled(
                &cli.pin_icon,
                Style::default().fg(cli.pin_color),
            ));
            spans.push(Span::raw(" "));
        }
        spans.push(Span::raw(&app.name));
        frame.render_widget(Paragraph::new(Line::from(spans)).style(style), areas.text);
        if selected && let Some(marker) = areas.selection {
            frame.render_widget(
                Paragraph::new(format!("{} ", cli.selection_marker)).style(style),
                marker,
            );
        }
        if let (Some(icons), Some(icon_area), Some(icon)) =
            (app_icons.as_deref_mut(), areas.icon, app.icon.as_ref())
        {
            if icons.failed_list_icons.contains(icon) {
                continue;
            }
            let Some(placement) = icons.list_icons.get(icon) else {
                continue;
            };
            if !icons.image_manager.is_cached(&placement.key) {
                continue;
            }
            if !icons.image_manager.render_cached(
                frame,
                &placement.key,
                overflow_icon_area(icon_area, placement.top_overflow_rows),
            )? {
                icons.failed_list_icons.insert(icon.clone());
                render_failed = true;
            }
        }
    }
    Ok(render_failed)
}

fn row_style(
    cli: &Opts,
    pinned: bool,
    selected: bool,
) -> (Style, ratatui::style::Color, ratatui::style::Color) {
    let text = if selected {
        if pinned {
            cli.pinned_highlight_color.unwrap_or(cli.highlight_color)
        } else {
            cli.highlight_color
        }
    } else if pinned {
        cli.pinned_text_color.unwrap_or(cli.items_text_color)
    } else {
        cli.items_text_color
    };
    let background = if pinned {
        cli.pinned_background_color
            .unwrap_or(cli.items_background_color)
    } else {
        cli.items_background_color
    };
    let selection = if pinned {
        cli.pinned_selection_background_color
            .unwrap_or(cli.items_selection_background_color)
    } else {
        cli.items_selection_background_color
    };
    let style = Style::default().fg(text);
    (
        if selected {
            style.add_modifier(Modifier::BOLD)
        } else {
            style
        },
        background,
        selection,
    )
}

fn overflow_icon_area(item_area: Rect, top_overflow_rows: u16) -> Rect {
    Rect::new(
        item_area.x,
        item_area.y.saturating_sub(top_overflow_rows),
        item_area.width,
        item_area.height.saturating_add(top_overflow_rows),
    )
}

fn selection_marker_spans(cli: &Opts, selected: bool) -> Vec<Span<'_>> {
    let width = marker_gutter_width(cli);
    if width == 0 {
        return Vec::new();
    }
    if selected {
        vec![Span::raw(format!("{} ", cli.selection_marker))]
    } else {
        vec![Span::raw(" ".repeat(usize::from(width)))]
    }
}

fn marker_gutter_width(cli: &Opts) -> u16 {
    if !cli.show_selection_marker || cli.selection_marker.is_empty() {
        return 0;
    }
    let width =
        UnicodeWidthStr::width(cli.selection_marker.as_str()).min(usize::from(u16::MAX - 1));
    width as u16 + 1
}

#[cfg(test)]
fn selection_marker_area(area: Rect, selected: usize, row_height: u16) -> Rect {
    Rect::new(area.x, area.y + selected as u16 * row_height, area.width, 1)
}

fn apps_block(cli: &Opts) -> Block<'static> {
    super::panel_block(
        " Apps ",
        super::PanelTheme {
            show_border: cli.show_items_border,
            show_title: cli.show_panel_titles,
            bold_title: false,
            rounded_border: cli.rounded_borders,
            border_color: cli.items_border_color,
            background_color: cli.items_background_color,
            title_color: cli.header_title_color,
        },
    )
}

fn list_areas(area: Rect, cli: &Opts) -> ListAreas {
    if cli.app_grid_columns > 0 && !area.is_empty() {
        let icon_width = cli.desktop_icon_list_width.min(area.width);
        return ListAreas {
            text: Rect::new(area.x, area.bottom() - 1, area.width, 1),
            icon: cli.desktop_icon_mode.shows_list().then_some(Rect::new(
                area.x + (area.width - icon_width) / 2,
                area.y,
                icon_width,
                area.height - 1,
            )),
            selection: None,
        };
    }
    if !cli.desktop_icon_mode.shows_list() || area.width < 4 {
        return ListAreas {
            text: area,
            icon: None,
            selection: None,
        };
    }

    let marker_width = marker_gutter_width(cli);
    let fixed_width = marker_width + 1;
    if area.width <= fixed_width {
        return ListAreas {
            text: area,
            icon: None,
            selection: None,
        };
    }
    let icon_width = cli.desktop_icon_list_width.min(area.width - fixed_width);
    let gap = cli
        .desktop_icon_list_gap
        .min(area.width.saturating_sub(icon_width + fixed_width));
    match cli.desktop_icon_position {
        super::HorizontalPosition::Left if cli.desktop_icon_arrow_before && marker_width > 0 => {
            ListAreas {
                selection: Some(Rect::new(area.x, area.y, marker_width, area.height)),
                icon: Some(Rect::new(
                    area.x + marker_width,
                    area.y,
                    icon_width,
                    area.height,
                )),
                text: Rect::new(
                    area.x + marker_width + icon_width + gap,
                    area.y,
                    area.width - marker_width - icon_width - gap,
                    area.height,
                ),
            }
        }
        super::HorizontalPosition::Left => ListAreas {
            text: Rect::new(
                area.x + icon_width + gap,
                area.y,
                area.width - icon_width - gap,
                area.height,
            ),
            icon: Some(Rect::new(area.x, area.y, icon_width, area.height)),
            selection: None,
        },
        super::HorizontalPosition::Right => ListAreas {
            text: Rect::new(area.x, area.y, area.width - icon_width - gap, area.height),
            icon: Some(Rect::new(
                area.x + area.width - icon_width,
                area.y,
                icon_width,
                area.height,
            )),
            selection: None,
        },
        // Center is a title-preview placement. Keep list icons on the default side.
        super::HorizontalPosition::Center => ListAreas {
            text: Rect::new(area.x, area.y, area.width - icon_width - gap, area.height),
            icon: Some(Rect::new(
                area.x + area.width - icon_width,
                area.y,
                icon_width,
                area.height,
            )),
            selection: None,
        },
    }
}

fn list_content_area(area: Rect, cli: &Opts) -> Rect {
    super::selection_content_area(area, cli.items_selection_rounded)
}

#[cfg(test)]
mod tests;
