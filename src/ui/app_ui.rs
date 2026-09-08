//! Launcher panel layout and selected-application preview rendering.

use eyre::Result;
use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout, Margin, Rect};
use ratatui::style::Style;
use ratatui::text::Line;
use ratatui::widgets::Paragraph;
use std::collections::{HashMap, HashSet};

pub(crate) fn effective_title_height(total_height: u16, title_panel_height_percent: u16) -> u16 {
    if title_panel_height_percent == 0 {
        0
    } else {
        (total_height as f32 * title_panel_height_percent as f32 / 100.0).round() as u16
    }
}

pub(crate) fn launcher_panel_areas(size: Rect, cli: &crate::cli::Opts) -> (Rect, Rect, Rect) {
    if cli.panels.enabled() {
        return cli.panels.split(
            size,
            cli.title_panel_height_percent,
            cli.input_panel_height,
            cli.title_panel_position.unwrap_or_default(),
        );
    }
    let title_height = effective_title_height(size.height, cli.title_panel_height_percent);
    let chunks = match cli.title_panel_position {
        Some(crate::ui::PanelPosition::Bottom) => Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Min(0),
                Constraint::Length(cli.input_panel_height),
                Constraint::Length(title_height),
            ])
            .split(size),
        Some(crate::ui::PanelPosition::Middle) => Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Min(0),
                Constraint::Length(title_height),
                Constraint::Length(cli.input_panel_height),
                Constraint::Min(0),
            ])
            .split(size),
        _ => Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(title_height),
                Constraint::Min(0),
                Constraint::Length(cli.input_panel_height),
            ])
            .split(size),
    };

    match cli.title_panel_position {
        Some(crate::ui::PanelPosition::Bottom) => (chunks[2], chunks[1], chunks[0]),
        Some(crate::ui::PanelPosition::Middle) => (chunks[1], chunks[2], chunks[0]),
        _ => (chunks[0], chunks[2], chunks[1]),
    }
}

fn split_icon_preview(
    area: Rect,
    position: crate::ui::HorizontalPosition,
    icon_width_percent: u16,
) -> (Rect, Option<Rect>) {
    if position == crate::ui::HorizontalPosition::Center {
        let icon_width = (u32::from(area.width) * u32::from(icon_width_percent) / 100)
            .min(u32::from(area.width)) as u16;
        let icon_x = area.x + area.width.saturating_sub(icon_width) / 2;
        return (Rect::new(icon_x, area.y, icon_width, area.height), None);
    }

    let text_width_percent = 100u16.saturating_sub(icon_width_percent);
    let constraints = match position {
        crate::ui::HorizontalPosition::Left => [
            Constraint::Percentage(icon_width_percent),
            Constraint::Percentage(text_width_percent),
        ],
        crate::ui::HorizontalPosition::Right => [
            Constraint::Percentage(text_width_percent),
            Constraint::Percentage(icon_width_percent),
        ],
        crate::ui::HorizontalPosition::Center => unreachable!("center was handled above"),
    };
    let content = Layout::horizontal(constraints).split(area);
    match position {
        crate::ui::HorizontalPosition::Left => (content[0], Some(content[1])),
        crate::ui::HorizontalPosition::Right => (content[1], Some(content[0])),
        crate::ui::HorizontalPosition::Center => unreachable!("center was handled above"),
    }
}

pub(crate) fn launcher_preview_icon_area(size: Rect, cli: &crate::cli::Opts) -> Rect {
    let (title_area, _, _) = launcher_panel_areas(size, cli);
    if title_area.is_empty() {
        return Rect::default();
    }
    let panel_inner = info_block("", cli).inner(title_area);
    let (icon_area, _) = split_icon_preview(
        panel_inner,
        cli.desktop_icon_position,
        cli.desktop_icon_preview_width_percent,
    );
    let icon_area = icon_area.inner(Margin {
        horizontal: 1,
        vertical: 0,
    });
    Rect::new(0, 0, icon_area.width, icon_area.height)
}

fn info_block<'a>(title: &'a str, cli: &crate::cli::Opts) -> ratatui::widgets::Block<'a> {
    super::panel_block(
        title,
        super::PanelTheme {
            show_border: cli.show_main_border,
            show_title: cli.show_panel_titles,
            bold_title: false,
            rounded_border: cli.rounded_borders,
            border_color: cli.main_border_color,
            background_color: cli.main_background_color,
            title_color: cli.header_title_color,
        },
    )
}

/// App filtering and sorting UI (Stateless Renderer)
pub struct UI;

/// Borrowed application icon state used by the launcher renderer.
pub struct AppIcons<'a> {
    pub(crate) image_manager: &'a mut crate::ui::ImageManager,
    pub(crate) preview_key: Option<&'a str>,
    pub(crate) list_icons: &'a HashMap<String, ListIconPlacement>,
    pub(crate) failed_list_icons: &'a mut HashSet<String>,
}

pub(crate) struct ListIconPlacement {
    pub(crate) key: String,
    pub(crate) top_overflow_rows: u16,
}

impl UI {
    /// Create new stateless UI renderer
    pub fn new() -> Self {
        Self
    }

    /// Render the UI using the centralized State
    pub fn render(
        &self,
        f: &mut Frame,
        state: &crate::core::state::State,
        cli: &crate::cli::Opts,
        mut app_icons: Option<AppIcons<'_>>,
    ) -> Result<(bool, bool)> {
        let size = f.area();
        let mut icon_render_failed = false;
        let (title_area, input_area, apps_area) = launcher_panel_areas(size, cli);
        let should_render_border = !title_area.is_empty();

        // Render Title/Info Panel
        if should_render_border {
            // Determine dynamic title
            let title = if cli.fancy_mode {
                if let Some(selected) = state.selected {
                    state
                        .shown
                        .get(selected)
                        .map(|a| a.name.clone())
                        .unwrap_or_else(|| "Fsel".to_string())
                } else {
                    "Fsel".to_string()
                }
            } else {
                "Fsel".to_string()
            };

            let title = format!(" {title} ");
            let info_block = info_block(&title, cli);

            // Text rendering from state.text which should be populated by state.update_info
            let info_text: Vec<Line> = state.text.lines().map(Line::from).collect();
            if app_icons
                .as_ref()
                .and_then(|icons| icons.preview_key)
                .is_some()
            {
                let inner = info_block.inner(title_area);
                let (icon_area, text_area) = split_icon_preview(
                    inner,
                    cli.desktop_icon_position,
                    cli.desktop_icon_preview_width_percent,
                );
                let icon_area = icon_area.inner(Margin {
                    horizontal: 1,
                    vertical: 0,
                });
                f.render_widget(info_block, title_area);
                let icon_rendered = if icon_area.width > 0 && icon_area.height > 0 {
                    let icons = app_icons
                        .as_mut()
                        .expect("preview key requires application icon state");
                    let key = icons
                        .preview_key
                        .expect("preview key was checked before rendering");
                    Some(icons.image_manager.render_cached(f, key, icon_area)?)
                } else {
                    None
                };
                if icon_rendered == Some(true) {
                    if let Some(text_area) = text_area {
                        f.render_widget(
                            Paragraph::new(info_text)
                                .style(Style::default().fg(cli.main_text_color)),
                            text_area,
                        );
                    }
                } else {
                    icon_render_failed = icon_rendered == Some(false);
                    f.render_widget(
                        Paragraph::new(info_text).style(Style::default().fg(cli.main_text_color)),
                        inner,
                    );
                }
            } else {
                let paragraph = Paragraph::new(info_text)
                    .block(info_block)
                    .style(Style::default().fg(cli.main_text_color));
                f.render_widget(paragraph, title_area);
            }
        }

        super::input_panel::render(f, state, cli, input_area);

        let list_render_failed =
            super::app_list::render(f, state, cli, apps_area, app_icons.as_mut())?;
        Ok((icon_render_failed, list_render_failed))
    }
}

#[cfg(test)]
mod tests {
    use super::{effective_title_height, launcher_preview_icon_area, split_icon_preview};
    use crate::cli::{DesktopIconMode, Opts};
    use crate::ui::HorizontalPosition;
    use ratatui::layout::Rect;

    #[test]
    fn effective_title_height_allows_zero() {
        assert_eq!(effective_title_height(40, 0), 0);
    }

    #[test]
    fn effective_title_height_matches_percentage_rounding() {
        assert_eq!(effective_title_height(21, 10), 2);
    }

    #[test]
    fn icon_preview_can_place_icon_on_the_right() {
        let (icon, text) =
            split_icon_preview(Rect::new(0, 0, 100, 10), HorizontalPosition::Right, 40);

        assert_eq!(text, Some(Rect::new(0, 0, 60, 10)));
        assert_eq!(icon, Rect::new(60, 0, 40, 10));
    }

    #[test]
    fn icon_preview_can_swap_to_the_left() {
        let (icon, text) =
            split_icon_preview(Rect::new(0, 0, 100, 10), HorizontalPosition::Left, 35);

        assert_eq!(icon, Rect::new(0, 0, 35, 10));
        assert_eq!(text, Some(Rect::new(35, 0, 65, 10)));
    }

    #[test]
    fn icon_preview_can_use_the_center_of_the_title_panel() {
        let (icon, text) =
            split_icon_preview(Rect::new(10, 3, 100, 10), HorizontalPosition::Center, 40);

        assert_eq!(icon, Rect::new(40, 3, 40, 10));
        assert_eq!(text, None);
    }

    #[test]
    fn centered_preview_percentage_does_not_saturate_on_wide_terminals() {
        let (icon, _) =
            split_icon_preview(Rect::new(0, 0, 2_000, 10), HorizontalPosition::Center, 40);

        assert_eq!(icon, Rect::new(600, 0, 800, 10));
    }

    #[test]
    fn preview_worker_area_matches_the_rendered_icon_slot() {
        let cli = Opts {
            desktop_icon_mode: DesktopIconMode::Preview,
            title_panel_height_percent: 25,
            desktop_icon_preview_width_percent: 40,
            ..Opts::default()
        };

        assert_eq!(
            launcher_preview_icon_area(Rect::new(0, 0, 100, 40), &cli),
            Rect::new(0, 0, 37, 8)
        );
    }

    #[test]
    fn borderless_preview_uses_the_released_panel_cells() {
        let cli = Opts {
            desktop_icon_mode: DesktopIconMode::Preview,
            title_panel_height_percent: 25,
            desktop_icon_preview_width_percent: 40,
            show_main_border: false,
            ..Opts::default()
        };

        assert_eq!(
            launcher_preview_icon_area(Rect::new(0, 0, 100, 40), &cli),
            Rect::new(0, 0, 38, 9)
        );

        let titleless = Opts {
            show_panel_titles: false,
            ..cli
        };
        assert_eq!(
            launcher_preview_icon_area(Rect::new(0, 0, 100, 40), &titleless),
            Rect::new(0, 0, 38, 10)
        );
    }
}
