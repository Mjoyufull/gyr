use crossterm::event::KeyCode;
use ratatui::layout::Rect;
use ratatui::style::Color;

use crate::cli::{Opts, PanelPosition};
use crate::ui::{
    GraphicsAdapter, Keybinds, effective_content_height, items_panel_bounds, items_panel_height,
};

pub(super) struct DmenuOptions {
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
                .unwrap_or(cli.apps_border_color),
            input_border_color: cli
                .dmenu_input_border_color
                .unwrap_or(cli.input_border_color),
            main_text_color: cli.dmenu_main_text_color.unwrap_or(cli.main_text_color),
            items_text_color: cli.dmenu_items_text_color.unwrap_or(cli.apps_text_color),
            input_text_color: cli.dmenu_input_text_color.unwrap_or(cli.input_text_color),
            header_title_color: cli
                .dmenu_header_title_color
                .unwrap_or(cli.header_title_color),
            rounded_borders: cli.dmenu_rounded_borders.unwrap_or(cli.rounded_borders),
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

    pub(super) fn items_panel_height(&self, total_height: u16) -> u16 {
        items_panel_height(
            total_height,
            self.content_height(total_height),
            self.input_panel_height,
        )
    }

    pub(super) fn max_visible_items(&self, total_height: u16) -> usize {
        self.items_panel_height(total_height).saturating_sub(2) as usize
    }

    pub(super) fn items_panel_bounds(&self, total_height: u16) -> (u16, u16) {
        items_panel_bounds(
            total_height,
            self.content_height(total_height),
            self.input_panel_height,
            self.content_panel_position,
        )
    }

    pub(super) fn split_layout(&self, area: Rect) -> crate::ui::PanelLayout {
        crate::ui::split_content_panels(
            area,
            self.content_height(area.height),
            self.input_panel_height,
            self.content_panel_position,
        )
    }
}
