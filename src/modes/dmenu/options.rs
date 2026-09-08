//! Effective dmenu options after shared and mode-specific overrides are merged.

use crossterm::event::KeyCode;
use ratatui::layout::Rect;
use ratatui::style::Color;

use crate::cli::{Opts, PanelPosition};
use crate::ui::{GraphicsAdapter, InputPanelStyle, Keybinds, effective_content_height};

pub(super) struct DmenuOptions {
    pub(super) panels: crate::ui::PanelSettings,
    pub(super) custom_panels: Vec<super::panels::DmenuPanel>,
    pub(super) disable_mouse: bool,
    pub(super) prompt_only: bool,
    pub(super) hide_before_typing: bool,
    pub(super) password_mode: bool,
    pub(super) password_character: String,
    pub(super) auto_select: bool,
    pub(super) only_match: bool,
    pub(super) index_mode: bool,
    pub(super) index_original_mode: bool,
    pub(super) accept_nth: Option<Vec<usize>>,
    pub(super) hard_stop: bool,
    pub(super) highlight_color: Color,
    pub(super) main_border_color: Color,
    pub(super) items_border_color: Color,
    pub(super) input_border_color: Color,
    pub(super) main_text_color: Color,
    pub(super) items_text_color: Color,
    pub(super) input_text_color: Color,
    pub(super) header_title_color: Color,
    pub(super) rounded_borders: bool,
    pub(super) show_main_border: bool,
    pub(super) show_items_border: bool,
    pub(super) show_input_border: bool,
    pub(super) show_panel_titles: bool,
    pub(super) show_input_count: bool,
    pub(super) show_input_prompt: bool,
    pub(super) show_selection_marker: bool,
    pub(super) selection_marker: String,
    pub(super) input_panel_style: InputPanelStyle,
    pub(super) main_background_color: Color,
    pub(super) items_background_color: Color,
    pub(super) items_selection_background_color: Color,
    pub(super) items_selection_rounded: bool,
    pub(super) input_background_color: Color,
    pub(super) content_panel_height_percent: u16,
    pub(super) input_panel_height: u16,
    pub(super) content_panel_position: PanelPosition,
    pub(super) cursor: String,
    pub(super) term_is_foot: bool,
    pub(super) graphics_adapter: GraphicsAdapter,
    pub(super) preview_command: Option<String>,
    pub(super) keybinds: Keybinds,
}

impl DmenuOptions {
    pub(super) fn from_cli(cli: &Opts) -> Self {
        Self {
            panels: cli.panels.clone(),
            custom_panels: cli.dmenu_panels.clone(),
            disable_mouse: cli.dmenu_disable_mouse.unwrap_or(cli.disable_mouse),
            prompt_only: cli.dmenu_prompt_only,
            hide_before_typing: cli.dmenu_hide_before_typing,
            password_mode: cli.dmenu_password_mode,
            password_character: cli.dmenu_password_character.clone(),
            auto_select: cli.dmenu_auto_select,
            only_match: cli.dmenu_only_match,
            index_mode: cli.dmenu_index_mode,
            index_original_mode: cli.dmenu_index_original_mode,
            accept_nth: cli.dmenu_accept_nth.clone(),
            hard_stop: cli.dmenu_hard_stop.unwrap_or(cli.hard_stop),
            highlight_color: cli.dmenu_highlight_color.unwrap_or(cli.highlight_color),
            main_border_color: cli.dmenu_main_border_color.unwrap_or(cli.main_border_color),
            items_border_color: cli
                .dmenu_items_border_color
                .unwrap_or(cli.items_border_color),
            input_border_color: cli
                .dmenu_input_border_color
                .unwrap_or(cli.input_border_color),
            main_text_color: cli.dmenu_main_text_color.unwrap_or(cli.main_text_color),
            items_text_color: cli.dmenu_items_text_color.unwrap_or(cli.items_text_color),
            input_text_color: cli.dmenu_input_text_color.unwrap_or(cli.input_text_color),
            header_title_color: cli
                .dmenu_header_title_color
                .unwrap_or(cli.header_title_color),
            rounded_borders: cli.dmenu_rounded_borders.unwrap_or(cli.rounded_borders),
            show_main_border: cli.show_main_border,
            show_items_border: cli.show_items_border,
            show_input_border: cli.show_input_border,
            show_panel_titles: cli.show_panel_titles,
            show_input_count: cli.show_input_count,
            show_input_prompt: cli.show_input_prompt,
            show_selection_marker: cli.show_selection_marker,
            selection_marker: cli.selection_marker.clone(),
            input_panel_style: cli.input_panel_style,
            main_background_color: cli.main_background_color,
            items_background_color: cli.items_background_color,
            items_selection_background_color: cli.items_selection_background_color,
            items_selection_rounded: cli.items_selection_rounded,
            input_background_color: cli.input_background_color,
            content_panel_height_percent: cli
                .dmenu_title_panel_height_percent
                .unwrap_or(cli.title_panel_height_percent),
            input_panel_height: cli
                .dmenu_input_panel_height
                .unwrap_or(cli.input_panel_height),
            content_panel_position: cli
                .dmenu_title_panel_position
                .unwrap_or(cli.title_panel_position.unwrap_or(PanelPosition::Top)),
            cursor: cli
                .dmenu_cursor
                .clone()
                .unwrap_or_else(|| cli.cursor.clone()),
            term_is_foot: std::env::var("TERM")
                .unwrap_or_default()
                .starts_with("foot"),
            graphics_adapter: GraphicsAdapter::detect(None),
            preview_command: cli.dmenu_preview.clone(),
            keybinds: cli.keybinds.clone(),
        }
    }

    pub(super) fn input_config(&self) -> crate::ui::InputConfig {
        crate::ui::InputConfig {
            disable_mouse: self.disable_mouse,
            exit_key: KeyCode::Null,
            render_rate: None,
            ..crate::ui::InputConfig::default()
        }
    }

    pub(super) fn input_title(&self) -> &'static str {
        if self.prompt_only {
            " Input "
        } else {
            " Filter "
        }
    }

    pub(super) fn display_query(&self, query: &str) -> String {
        if self.password_mode {
            self.password_character.repeat(query.chars().count())
        } else {
            query.to_string()
        }
    }

    pub(super) fn content_height(&self, total_height: u16) -> u16 {
        effective_content_height(total_height, self.content_panel_height_percent)
    }

    pub(super) fn max_visible_items(&self, area: Rect) -> usize {
        self.result_layout(area).capacity()
    }

    pub(super) fn result_layout(&self, area: Rect) -> crate::ui::result_layout::ResultLayout {
        let layout = self.split_layout(area);
        let block = crate::ui::panel_block(
            " Items ",
            crate::ui::PanelTheme {
                show_border: self.show_items_border,
                show_title: self.show_panel_titles,
                bold_title: true,
                rounded_border: self.rounded_borders,
                border_color: self.items_border_color,
                background_color: self.items_background_color,
                title_color: self.header_title_color,
            },
        );
        crate::ui::result_layout::ResultLayout::new(
            block.inner(layout.chunks[layout.items_panel_index]),
            1,
            &self.panels,
        )
    }

    pub(super) fn split_layout(&self, area: Rect) -> crate::ui::PanelLayout {
        self.split_all(area).0
    }

    pub(super) fn split_all(&self, area: Rect) -> (crate::ui::PanelLayout, Vec<Rect>) {
        let mut layout = if self.panels.enabled() {
            let (info, input, items) = self.panels.split(
                area,
                self.content_panel_height_percent,
                self.input_panel_height,
                self.content_panel_position,
            );
            crate::ui::PanelLayout {
                chunks: [info, items, input],
                content_panel_index: 0,
                items_panel_index: 1,
                input_panel_index: 2,
            }
        } else {
            crate::ui::split_content_panels(
                area,
                self.content_height(area.height),
                self.input_panel_height,
                self.content_panel_position,
            )
        };
        let mut custom = Vec::with_capacity(self.custom_panels.len());
        let mut remaining = layout.chunks[layout.items_panel_index];
        for panel in &self.custom_panels {
            let side = panel.position.rotated(self.panels.rotation);
            let total = if side.horizontal() {
                remaining.width
            } else {
                remaining.height
            };
            let cells = (u32::from(total) * u32::from(panel.size) / 100) as u16;
            let (rect, rest) = crate::ui::panels::dock(remaining, side, cells);
            custom.push(rect);
            remaining = rest;
        }
        layout.chunks[layout.items_panel_index] = remaining;
        (layout, custom)
    }
}

#[cfg(test)]
mod tests {
    use super::DmenuOptions;
    use crate::cli::Opts;
    use crate::ui::InputPanelStyle;
    use ratatui::layout::Rect;
    use ratatui::style::Color;

    #[test]
    fn dmenu_inherits_shared_visual_options() {
        let options = DmenuOptions::from_cli(&Opts {
            show_main_border: false,
            show_items_border: false,
            show_input_border: false,
            show_panel_titles: false,
            show_input_count: false,
            show_input_prompt: false,
            show_selection_marker: false,
            selection_marker: "█".to_string(),
            input_panel_style: InputPanelStyle::Command,
            main_background_color: Color::Black,
            items_background_color: Color::Blue,
            items_selection_background_color: Color::Yellow,
            items_selection_rounded: true,
            input_background_color: Color::Red,
            ..Opts::default()
        });

        assert!(!options.show_main_border);
        assert!(!options.show_items_border);
        assert!(!options.show_input_border);
        assert!(!options.show_panel_titles);
        assert!(!options.show_input_count);
        assert!(!options.show_input_prompt);
        assert!(!options.show_selection_marker);
        assert_eq!(options.selection_marker, "█");
        assert_eq!(options.input_panel_style, InputPanelStyle::Command);
        assert_eq!(options.main_background_color, Color::Black);
        assert_eq!(options.items_background_color, Color::Blue);
        assert_eq!(options.items_selection_background_color, Color::Yellow);
        assert!(options.items_selection_rounded);
        assert_eq!(options.input_background_color, Color::Red);
    }

    #[test]
    fn custom_panels_partition_space_without_overlap_at_every_rotation() {
        for rotation in [0, 90, 180, 270] {
            let mut cli = Opts::default();
            cli.panels.rotation = rotation;
            cli.dmenu_panels = vec![
                super::super::panels::DmenuPanel::parse("details:left:30:printf details")
                    .expect("valid panel"),
                super::super::panels::DmenuPanel::parse("extra:bottom:20:printf extra")
                    .expect("valid panel"),
            ];
            let options = DmenuOptions::from_cli(&cli);
            for (width, height) in [(0, 0), (1, 1), (2, 3), (80, 40)] {
                let area = Rect::new(3, 5, width, height);
                let (layout, custom) = options.split_all(area);
                let rectangles: Vec<_> = layout.chunks.into_iter().chain(custom).collect();
                assert_eq!(
                    rectangles
                        .iter()
                        .map(|r| u32::from(r.width) * u32::from(r.height))
                        .sum::<u32>(),
                    u32::from(width) * u32::from(height)
                );
                for (index, rectangle) in rectangles.iter().enumerate() {
                    for other in &rectangles[index + 1..] {
                        assert!(rectangle.intersection(*other).is_empty());
                    }
                }
            }
        }
    }

    #[test]
    fn borderless_dmenu_mouse_area_uses_released_rows() {
        let bordered = DmenuOptions::from_cli(&Opts::default());
        let borderless = DmenuOptions::from_cli(&Opts {
            show_items_border: false,
            show_panel_titles: false,
            ..Opts::default()
        });

        assert_eq!(
            bordered.result_layout(Rect::new(0, 0, 80, 40)).slot(0).y,
            13
        );
        assert_eq!(
            borderless.result_layout(Rect::new(0, 0, 80, 40)).slot(0).y,
            12
        );
    }
}
