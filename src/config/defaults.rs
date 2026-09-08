//! Stable configuration defaults shared with runtime option defaults.

use super::schema::{GeneralConfig, LayoutConfig, UiConfig};
use crate::cli::{MatchMode, PinnedOrderMode, RankingMode};
use crate::ui::PanelPosition;

pub(super) fn default_terminal_launcher() -> String {
    "alacritty -e".to_string()
}

pub(super) fn default_true() -> bool {
    true
}

pub(super) fn default_match_mode() -> MatchMode {
    MatchMode::Fuzzy
}

pub(super) fn default_ranking_mode() -> RankingMode {
    RankingMode::Frecency
}

pub(super) fn default_pinned_order() -> PinnedOrderMode {
    PinnedOrderMode::Ranking
}

pub(super) fn default_highlight_color() -> String {
    "LightBlue".to_string()
}

pub(super) fn default_cursor() -> String {
    "█".to_string()
}

pub(super) fn default_selection_marker() -> String {
    ">".to_string()
}

pub(super) fn default_white() -> String {
    "White".to_string()
}

pub(super) fn default_reset() -> String {
    "Reset".to_string()
}

pub(super) fn default_pin_color() -> String {
    "rgb(255, 165, 0)".to_string()
}

pub(super) fn default_pin_icon() -> String {
    "📌".to_string()
}

pub(super) fn default_title_panel_height() -> u16 {
    30
}

pub(super) fn default_input_panel_height() -> u16 {
    3
}

pub(super) fn default_title_panel_position() -> PanelPosition {
    PanelPosition::Top
}

pub(super) fn default_prefix_depth() -> usize {
    3
}

impl Default for GeneralConfig {
    fn default() -> Self {
        Self {
            terminal_launcher: default_terminal_launcher(),
            filter_desktop: true,
            list_executables_in_path: false,
            hide_before_typing: false,
            match_mode: default_match_mode(),
            ranking_mode: default_ranking_mode(),
            pinned_order: default_pinned_order(),
            systemd_run: false,
            uwsm: false,
            detach: false,
            no_exec: false,
            confirm_first_launch: false,
            prefix_depth: default_prefix_depth(),
        }
    }
}

impl Default for UiConfig {
    fn default() -> Self {
        Self {
            pinned_text_color: None,
            pinned_background_color: None,
            pinned_highlight_color: None,
            pinned_selection_background_color: None,
            highlight_color: default_highlight_color(),
            cursor: default_cursor(),
            hard_stop: false,
            rounded_borders: true,
            show_main_border: true,
            show_items_border: true,
            show_input_border: true,
            show_panel_titles: true,
            show_input_count: true,
            show_input_prompt: true,
            show_selection_marker: true,
            selection_marker: default_selection_marker(),
            show_pin_icons: true,
            input_panel_style: crate::ui::InputPanelStyle::Classic,
            disable_mouse: false,
            main_border_color: default_white(),
            main_background_color: default_reset(),
            items_border_color: default_white(),
            items_background_color: default_reset(),
            items_selection_background_color: default_reset(),
            items_selection_rounded: false,
            input_border_color: default_white(),
            input_background_color: default_reset(),
            main_text_color: default_white(),
            items_text_color: default_white(),
            input_text_color: default_white(),
            fancy_mode: false,
            header_title_color: default_white(),
            pin_color: default_pin_color(),
            pin_icon: default_pin_icon(),
            keybinds: crate::ui::Keybinds::default(),
        }
    }
}

impl Default for LayoutConfig {
    fn default() -> Self {
        Self {
            title_panel_height_percent: default_title_panel_height(),
            input_panel_height: default_input_panel_height(),
            title_panel_position: default_title_panel_position(),
        }
    }
}
