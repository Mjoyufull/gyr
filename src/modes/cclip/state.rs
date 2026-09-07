//! Effective cclip rendering and interaction options.

use crossterm::event::KeyCode;
use ratatui::layout::Rect;
use ratatui::style::Color;

use crate::cli::{Opts, PanelPosition};
use crate::ui::{GraphicsAdapter, InputConfig, InputPanelStyle, Keybinds};

pub(super) struct CclipOptions {
    pub(super) panels: crate::ui::PanelSettings,
    pub(super) disable_mouse: bool,
    pub(super) hard_stop: bool,
    pub(super) wrap_long_lines: bool,
    pub(super) show_line_numbers: bool,
    pub(super) show_tag_color_names: bool,
    pub(super) hide_image_message: bool,
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
    pub(super) keybinds: Keybinds,
    pub(super) content_panel_height_percent: u16,
    pub(super) input_panel_height: u16,
    pub(super) content_panel_position: PanelPosition,
    pub(super) cursor: String,
    pub(super) term_is_foot: bool,
    pub(super) graphics_adapter: GraphicsAdapter,
    pub(super) explicit_image_preview: Option<bool>,
}

impl CclipOptions {
    pub(super) fn from_cli(cli: &Opts) -> Self {
        Self {
            panels: cli.panels.clone(),
            disable_mouse: cli
                .cclip_disable_mouse
                .or(cli.dmenu_disable_mouse)
                .unwrap_or(cli.disable_mouse),
            hard_stop: cli
                .cclip_hard_stop
                .or(cli.dmenu_hard_stop)
                .unwrap_or(cli.hard_stop),
            wrap_long_lines: cli.cclip_wrap_long_lines.unwrap_or(true),
            show_line_numbers: super::items::show_line_numbers(cli),
            show_tag_color_names: cli.cclip_show_tag_color_names.unwrap_or(false),
            hide_image_message: cli.cclip_hide_inline_image_message.unwrap_or(false),
            highlight_color: cli
                .cclip_highlight_color
                .or(cli.dmenu_highlight_color)
                .unwrap_or(cli.highlight_color),
            main_border_color: cli
                .cclip_main_border_color
                .or(cli.dmenu_main_border_color)
                .unwrap_or(cli.main_border_color),
            items_border_color: cli
                .cclip_items_border_color
                .or(cli.dmenu_items_border_color)
                .unwrap_or(cli.items_border_color),
            input_border_color: cli
                .cclip_input_border_color
                .or(cli.dmenu_input_border_color)
                .unwrap_or(cli.input_border_color),
            main_text_color: cli
                .cclip_main_text_color
                .or(cli.dmenu_main_text_color)
                .unwrap_or(cli.main_text_color),
            items_text_color: cli
                .cclip_items_text_color
                .or(cli.dmenu_items_text_color)
                .unwrap_or(cli.items_text_color),
            input_text_color: cli
                .cclip_input_text_color
                .or(cli.dmenu_input_text_color)
                .unwrap_or(cli.input_text_color),
            header_title_color: cli
                .cclip_header_title_color
                .or(cli.dmenu_header_title_color)
                .unwrap_or(cli.header_title_color),
            rounded_borders: cli
                .cclip_rounded_borders
                .or(cli.dmenu_rounded_borders)
                .unwrap_or(cli.rounded_borders),
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
            keybinds: cli.keybinds.clone(),
            content_panel_height_percent: cli
                .cclip_title_panel_height_percent
                .or(cli.dmenu_title_panel_height_percent)
                .unwrap_or(cli.title_panel_height_percent),
            input_panel_height: cli
                .cclip_input_panel_height
                .or(cli.dmenu_input_panel_height)
                .unwrap_or(cli.input_panel_height),
            content_panel_position: cli
                .cclip_title_panel_position
                .or(cli.dmenu_title_panel_position)
                .unwrap_or(cli.title_panel_position.unwrap_or(PanelPosition::Top)),
            cursor: cli
                .cclip_cursor
                .clone()
                .or(cli.dmenu_cursor.clone())
                .unwrap_or_else(|| cli.cursor.clone()),
            term_is_foot: std::env::var("TERM")
                .unwrap_or_default()
                .starts_with("foot"),
            graphics_adapter: GraphicsAdapter::detect(None),
            explicit_image_preview: cli.cclip_image_preview,
        }
    }

    pub(super) fn set_graphics_adapter(&mut self, adapter: GraphicsAdapter) {
        self.graphics_adapter = adapter;
    }

    pub(super) fn input_config(&self) -> InputConfig {
        InputConfig {
            exit_key: KeyCode::Null,
            disable_mouse: self.disable_mouse,
            render_rate: None,
            ..InputConfig::default()
        }
    }

    pub(super) fn content_height(&self, total_height: u16) -> u16 {
        crate::ui::effective_content_height(total_height, self.content_panel_height_percent)
    }

    pub(super) fn split_layout(&self, area: Rect) -> crate::ui::PanelLayout {
        if self.panels.enabled() {
            let (info, input, items) = self.panels.split(
                area,
                self.content_panel_height_percent,
                self.input_panel_height,
                self.content_panel_position,
            );
            return crate::ui::PanelLayout {
                chunks: [info, items, input],
                content_panel_index: 0,
                items_panel_index: 1,
                input_panel_index: 2,
            };
        }
        crate::ui::split_content_panels(
            area,
            self.content_height(area.height),
            self.input_panel_height,
            self.content_panel_position,
        )
    }

    pub(super) fn max_visible_items(&self, area: Rect) -> usize {
        self.result_layout(area).capacity()
    }

    pub(super) fn result_layout(&self, area: Rect) -> crate::ui::result_layout::ResultLayout {
        let layout = self.split_layout(area);
        let block = crate::ui::panel_block(
            " Clipboard History ",
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

    pub(super) fn image_preview_enabled(&self, supports_graphics: bool) -> bool {
        self.explicit_image_preview.unwrap_or(supports_graphics)
    }
}

#[cfg(test)]
mod tests {
    use super::CclipOptions;
    use crate::cli::Opts;
    use crate::ui::InputPanelStyle;
    use ratatui::style::Color;

    #[test]
    fn cclip_inherits_shared_visual_options() {
        let options = CclipOptions::from_cli(&Opts {
            show_items_border: false,
            show_panel_titles: false,
            show_input_count: false,
            show_input_prompt: false,
            show_selection_marker: false,
            selection_marker: "█".to_string(),
            input_panel_style: InputPanelStyle::Command,
            items_background_color: Color::Blue,
            items_selection_background_color: Color::Yellow,
            items_selection_rounded: true,
            ..Opts::default()
        });

        assert!(!options.show_items_border);
        assert!(!options.show_panel_titles);
        assert!(!options.show_input_count);
        assert!(!options.show_input_prompt);
        assert!(!options.show_selection_marker);
        assert_eq!(options.selection_marker, "█");
        assert_eq!(options.input_panel_style, InputPanelStyle::Command);
        assert_eq!(options.items_background_color, Color::Blue);
        assert_eq!(options.items_selection_background_color, Color::Yellow);
        assert!(options.items_selection_rounded);
    }
}
