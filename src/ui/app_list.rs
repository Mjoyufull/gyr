use super::app_ui::AppIcons;
use crate::cli::Opts;
use crate::core::state::State;
use eyre::Result;
use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{
    Block, BorderType, Borders, HighlightSpacing, List, ListItem, ListState, Paragraph,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ListAreas {
    text: Rect,
    icon: Option<Rect>,
    selection: Option<Rect>,
}

pub(crate) fn launcher_visible_rows(total_height: u16, cli: &Opts) -> usize {
    let (_, _, apps_area) =
        super::app_ui::launcher_panel_areas(Rect::new(0, 0, 1, total_height), cli);
    visible_rows(apps_area.height.saturating_sub(2), cli)
}

fn visible_rows(content_height: u16, cli: &Opts) -> usize {
    usize::from(content_height / app_row_height(cli))
}

pub(crate) fn app_row_height(cli: &Opts) -> u16 {
    if cli.desktop_icon_mode.shows_list() {
        cli.desktop_icon_list_height.max(1)
    } else {
        1
    }
}

pub(crate) fn launcher_list_icon_area(size: Rect, cli: &Opts) -> Rect {
    let (_, _, apps_area) = super::app_ui::launcher_panel_areas(size, cli);
    let inner = apps_area.inner(ratatui::layout::Margin {
        horizontal: 1,
        vertical: 1,
    });
    let Some(icon_strip) = list_areas(inner, cli).icon else {
        return Rect::default();
    };
    Rect::new(0, 0, icon_strip.width, app_row_height(cli))
}

pub(super) fn render(
    frame: &mut Frame,
    state: &State,
    cli: &Opts,
    area: Rect,
    app_icons: Option<&mut AppIcons<'_>>,
) -> Result<bool> {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(cli.apps_border_color))
        .title(Span::styled(
            " Apps ",
            Style::default().fg(cli.header_title_color),
        ))
        .border_type(if cli.rounded_borders {
            BorderType::Rounded
        } else {
            BorderType::Plain
        });
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let row_height = app_row_height(cli);
    let max_visible = visible_rows(inner.height, cli);
    let visible_apps = state
        .shown
        .iter()
        .skip(state.scroll_offset)
        .take(max_visible)
        .collect::<Vec<_>>();
    let areas = list_areas(inner, cli);

    let items = visible_apps
        .iter()
        .map(|app| {
            let mut spans = Vec::new();
            if app.pinned {
                spans.push(Span::styled(
                    &cli.pin_icon,
                    Style::default().fg(cli.pin_color),
                ));
                spans.push(Span::raw(" "));
            }
            spans.push(Span::styled(
                &app.name,
                Style::default().fg(cli.apps_text_color),
            ));

            let mut lines = vec![Line::from(spans)];
            lines.resize_with(usize::from(row_height), Line::default);
            ListItem::new(lines)
        })
        .collect::<Vec<_>>();
    let highlight_style = Style::default()
        .fg(cli.highlight_color)
        .add_modifier(Modifier::BOLD);
    let arrow_before_icon = areas.selection.is_some();
    let list = List::new(items)
        .highlight_style(highlight_style)
        .highlight_symbol(if arrow_before_icon { "" } else { "> " })
        .highlight_spacing(if arrow_before_icon {
            HighlightSpacing::Never
        } else {
            HighlightSpacing::Always
        });
    let mut list_state = ListState::default();
    if let Some(selected) = state.selected
        && selected >= state.scroll_offset
        && selected < state.scroll_offset + max_visible
    {
        list_state.select(Some(selected - state.scroll_offset));
    }
    frame.render_stateful_widget(list, areas.text, &mut list_state);

    if let (Some(selected), Some(selection_area)) = (list_state.selected(), areas.selection) {
        let y = selection_area.y + selected as u16 * row_height;
        frame.render_widget(
            Paragraph::new("> ").style(highlight_style),
            Rect::new(selection_area.x, y, selection_area.width, 1),
        );
    }

    let mut render_failed = false;
    if let (Some(icons), Some(icon_strip)) = (app_icons, areas.icon) {
        for (row, app) in visible_apps.iter().enumerate() {
            let Some(icon) = app.icon.as_ref() else {
                continue;
            };
            if icons.failed_list_icons.contains(icon) {
                continue;
            }
            let Some(key) = icons.list_keys.get(icon) else {
                continue;
            };
            if !icons.image_manager.is_cached(key) {
                continue;
            }
            let icon_area = Rect::new(
                icon_strip.x,
                icon_strip.y + row as u16 * row_height,
                icon_strip.width,
                row_height,
            );
            if !icons.image_manager.render_cached(frame, key, icon_area)? {
                icons.failed_list_icons.insert(icon.clone());
                render_failed = true;
            }
        }
    }

    Ok(render_failed)
}

fn list_areas(area: Rect, cli: &Opts) -> ListAreas {
    if !cli.desktop_icon_mode.shows_list() || area.width < 4 {
        return ListAreas {
            text: area,
            icon: None,
            selection: None,
        };
    }

    // Preserve two columns for the selection marker and one for visible label text.
    let icon_width = cli.desktop_icon_list_width.min(area.width - 3);
    match cli.desktop_icon_position {
        super::HorizontalPosition::Left if cli.desktop_icon_arrow_before => ListAreas {
            selection: Some(Rect::new(area.x, area.y, 2, area.height)),
            icon: Some(Rect::new(area.x + 2, area.y, icon_width, area.height)),
            text: Rect::new(
                area.x + 2 + icon_width,
                area.y,
                area.width - 2 - icon_width,
                area.height,
            ),
        },
        super::HorizontalPosition::Left => ListAreas {
            text: Rect::new(
                area.x + icon_width,
                area.y,
                area.width - icon_width,
                area.height,
            ),
            icon: Some(Rect::new(area.x, area.y, icon_width, area.height)),
            selection: None,
        },
        super::HorizontalPosition::Right => ListAreas {
            text: Rect::new(area.x, area.y, area.width - icon_width, area.height),
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

#[cfg(test)]
mod tests {
    use super::{launcher_list_icon_area, launcher_visible_rows, list_areas};
    use crate::cli::{DesktopIconMode, Opts};
    use crate::ui::{HorizontalPosition, PanelPosition};
    use ratatui::layout::Rect;

    #[test]
    fn list_icons_reduce_visible_apps_by_configured_row_height() {
        let cli = Opts {
            desktop_icon_mode: DesktopIconMode::List,
            desktop_icon_list_height: 2,
            title_panel_height_percent: 25,
            input_panel_height: 3,
            ..Opts::default()
        };

        assert_eq!(launcher_visible_rows(40, &cli), 12);
    }

    #[test]
    fn visible_rows_saturates_when_panel_sizes_overflow() {
        let cli = Opts {
            title_panel_height_percent: u16::MAX,
            input_panel_height: u16::MAX,
            ..Opts::default()
        };

        assert_eq!(launcher_visible_rows(u16::MAX, &cli), 0);
    }

    #[test]
    fn middle_title_position_uses_the_actual_apps_pane_height() {
        let cli = Opts {
            desktop_icon_mode: DesktopIconMode::List,
            desktop_icon_list_height: 2,
            title_panel_position: Some(PanelPosition::Middle),
            title_panel_height_percent: 25,
            input_panel_height: 3,
            ..Opts::default()
        };

        assert_eq!(launcher_visible_rows(40, &cli), 6);
    }

    #[test]
    fn list_icons_can_reserve_the_right_side() {
        let cli = Opts {
            desktop_icon_mode: DesktopIconMode::Both,
            desktop_icon_position: HorizontalPosition::Right,
            desktop_icon_list_width: 4,
            ..Opts::default()
        };

        let areas = list_areas(Rect::new(10, 3, 30, 8), &cli);

        assert_eq!(areas.text, Rect::new(10, 3, 26, 8));
        assert_eq!(areas.icon, Some(Rect::new(36, 3, 4, 8)));
        assert_eq!(areas.selection, None);
    }

    #[test]
    fn list_icons_can_reserve_the_left_side() {
        let cli = Opts {
            desktop_icon_mode: DesktopIconMode::List,
            desktop_icon_position: HorizontalPosition::Left,
            desktop_icon_list_width: 5,
            ..Opts::default()
        };

        let areas = list_areas(Rect::new(2, 4, 20, 6), &cli);

        assert_eq!(areas.text, Rect::new(7, 4, 15, 6));
        assert_eq!(areas.icon, Some(Rect::new(2, 4, 5, 6)));
        assert_eq!(areas.selection, None);
    }

    #[test]
    fn selection_arrow_can_be_reserved_before_a_left_icon() {
        let cli = Opts {
            desktop_icon_mode: DesktopIconMode::List,
            desktop_icon_position: HorizontalPosition::Left,
            desktop_icon_list_width: 5,
            desktop_icon_arrow_before: true,
            ..Opts::default()
        };

        let areas = list_areas(Rect::new(2, 4, 20, 6), &cli);

        assert_eq!(areas.selection, Some(Rect::new(2, 4, 2, 6)));
        assert_eq!(areas.icon, Some(Rect::new(4, 4, 5, 6)));
        assert_eq!(areas.text, Rect::new(9, 4, 13, 6));
    }

    #[test]
    fn narrow_list_keeps_selection_and_label_space() {
        let cli = Opts {
            desktop_icon_mode: DesktopIconMode::List,
            desktop_icon_position: HorizontalPosition::Left,
            desktop_icon_list_width: 16,
            ..Opts::default()
        };

        let areas = list_areas(Rect::new(2, 4, 4, 6), &cli);

        assert_eq!(areas.icon, Some(Rect::new(2, 4, 1, 6)));
        assert_eq!(areas.text, Rect::new(3, 4, 3, 6));
    }

    #[test]
    fn list_worker_area_matches_each_rendered_icon_slot() {
        let cli = Opts {
            desktop_icon_mode: DesktopIconMode::List,
            desktop_icon_list_width: 5,
            desktop_icon_list_height: 2,
            ..Opts::default()
        };

        assert_eq!(
            launcher_list_icon_area(Rect::new(0, 0, 100, 40), &cli),
            Rect::new(0, 0, 5, 2)
        );
    }
}
