//! Deserializable configuration schema and compatibility aliases.

use serde::{Deserialize, Deserializer};
use std::str::FromStr;

use crate::cli::{DesktopIconMode, MatchMode, PinnedOrderMode, RankingMode};
use crate::ui::{HorizontalPosition, InputPanelStyle, PanelPosition};

#[derive(Debug, Deserialize, Clone, Default)]
pub struct FselConfig {
    #[serde(default)]
    pub panels: crate::ui::PanelSettings,
    #[serde(flatten)]
    pub general: GeneralConfig,
    #[serde(flatten)]
    pub ui: UiConfig,
    #[serde(flatten)]
    pub layout: LayoutConfig,
    #[serde(default)]
    pub dmenu: DmenuConfig,
    #[serde(default)]
    pub cclip: CclipConfig,
    #[serde(default)]
    pub app_launcher: AppLauncherConfig,
}

/// Legacy `[app_launcher]` section for backward compatibility.
#[derive(Debug, Deserialize, Clone, Default)]
pub struct AppLauncherConfig {
    pub filter_desktop: Option<bool>,
    pub filter_actions: Option<bool>,
    pub auto_hide_duplicates: Option<bool>,
    pub list_executables_in_path: Option<bool>,
    pub hide_before_typing: Option<bool>,
    pub launch_prefix: Option<Vec<String>>,
    #[serde(default, deserialize_with = "deserialize_optional_parsed")]
    pub match_mode: Option<MatchMode>,
    #[serde(default, deserialize_with = "deserialize_optional_parsed")]
    pub ranking_mode: Option<RankingMode>,
    #[serde(default, deserialize_with = "deserialize_optional_parsed")]
    pub pinned_order: Option<PinnedOrderMode>,
    pub confirm_first_launch: Option<bool>,
    pub prefix_depth: Option<usize>,
    #[serde(default, deserialize_with = "deserialize_optional_parsed")]
    pub icon_mode: Option<DesktopIconMode>,
    #[serde(default, deserialize_with = "deserialize_optional_parsed")]
    pub icon_position: Option<HorizontalPosition>,
    pub icon_preview_width_percent: Option<u16>,
    pub icon_list_width: Option<u16>,
    pub icon_list_height: Option<u16>,
    pub icon_list_gap: Option<u16>,
    pub icon_list_vertical_align_percent: Option<i16>,
    pub icon_arrow_before: Option<bool>,
    pub icon_size: Option<u16>,
    pub icon_horizontal_align_percent: Option<u16>,
    pub icon_vertical_align_percent: Option<u16>,
    pub icon_theme: Option<String>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct GeneralConfig {
    #[serde(default = "super::defaults::default_terminal_launcher")]
    pub terminal_launcher: String,
    #[serde(default = "super::defaults::default_true")]
    pub filter_desktop: bool,
    #[serde(default)]
    pub list_executables_in_path: bool,
    #[serde(default)]
    pub hide_before_typing: bool,
    #[serde(
        default = "super::defaults::default_match_mode",
        deserialize_with = "deserialize_parsed_or_default"
    )]
    pub match_mode: MatchMode,
    #[serde(
        default = "super::defaults::default_ranking_mode",
        deserialize_with = "deserialize_parsed_or_default"
    )]
    pub ranking_mode: RankingMode,
    #[serde(
        default = "super::defaults::default_pinned_order",
        deserialize_with = "deserialize_parsed_or_default"
    )]
    pub pinned_order: PinnedOrderMode,
    #[serde(default)]
    pub systemd_run: bool,
    #[serde(default)]
    pub uwsm: bool,
    #[serde(default)]
    pub detach: bool,
    #[serde(default)]
    pub no_exec: bool,
    #[serde(default)]
    pub confirm_first_launch: bool,
    #[serde(default = "super::defaults::default_prefix_depth")]
    pub prefix_depth: usize,
}

#[derive(Debug, Deserialize, Clone)]
pub struct UiConfig {
    #[serde(default = "super::defaults::default_highlight_color")]
    pub highlight_color: String,
    #[serde(default = "super::defaults::default_cursor")]
    pub cursor: String,
    #[serde(default)]
    pub hard_stop: bool,
    #[serde(default = "super::defaults::default_true")]
    pub rounded_borders: bool,
    #[serde(default = "super::defaults::default_true")]
    pub show_main_border: bool,
    #[serde(default = "super::defaults::default_true")]
    #[serde(alias = "show_apps_border")]
    pub show_items_border: bool,
    #[serde(default = "super::defaults::default_true")]
    pub show_input_border: bool,
    #[serde(default = "super::defaults::default_true")]
    pub show_panel_titles: bool,
    #[serde(default = "super::defaults::default_true")]
    pub show_input_count: bool,
    #[serde(default = "super::defaults::default_true")]
    pub show_input_prompt: bool,
    #[serde(default = "super::defaults::default_true")]
    pub show_selection_marker: bool,
    #[serde(default = "super::defaults::default_selection_marker")]
    pub selection_marker: String,
    #[serde(default = "super::defaults::default_true")]
    pub show_pin_icons: bool,
    #[serde(default, deserialize_with = "deserialize_parsed_or_default")]
    pub input_panel_style: InputPanelStyle,
    #[serde(default)]
    pub disable_mouse: bool,
    #[serde(default = "super::defaults::default_white")]
    pub main_border_color: String,
    #[serde(default = "super::defaults::default_reset")]
    pub main_background_color: String,
    #[serde(default = "super::defaults::default_white")]
    #[serde(alias = "apps_border_color")]
    pub items_border_color: String,
    #[serde(default = "super::defaults::default_reset")]
    #[serde(alias = "apps_background_color")]
    pub items_background_color: String,
    #[serde(default = "super::defaults::default_reset")]
    #[serde(alias = "apps_selection_background_color")]
    pub items_selection_background_color: String,
    #[serde(default)]
    #[serde(alias = "apps_selection_rounded")]
    pub items_selection_rounded: bool,
    #[serde(default = "super::defaults::default_white")]
    pub input_border_color: String,
    #[serde(default = "super::defaults::default_reset")]
    pub input_background_color: String,
    #[serde(default = "super::defaults::default_white")]
    pub main_text_color: String,
    #[serde(default = "super::defaults::default_white")]
    #[serde(alias = "apps_text_color")]
    pub items_text_color: String,
    #[serde(default = "super::defaults::default_white")]
    pub input_text_color: String,
    #[serde(default)]
    pub fancy_mode: bool,
    #[serde(default = "super::defaults::default_white")]
    pub header_title_color: String,
    #[serde(default = "super::defaults::default_pin_color")]
    pub pin_color: String,
    #[serde(default = "super::defaults::default_pin_icon")]
    pub pin_icon: String,
    #[serde(default)]
    pub keybinds: crate::ui::Keybinds,
}

#[derive(Debug, Deserialize, Clone)]
pub struct LayoutConfig {
    #[serde(default = "super::defaults::default_title_panel_height")]
    pub title_panel_height_percent: u16,
    #[serde(default = "super::defaults::default_input_panel_height")]
    pub input_panel_height: u16,
    #[serde(
        default = "super::defaults::default_title_panel_position",
        deserialize_with = "deserialize_parsed_or_default"
    )]
    pub title_panel_position: PanelPosition,
}

#[derive(Debug, Deserialize, Clone, Default)]
pub struct DmenuConfig {
    pub delimiter: Option<String>,
    pub preview: Option<String>,
    pub password_character: Option<String>,
    pub show_line_numbers: Option<bool>,
    pub wrap_long_lines: Option<bool>,
    pub exit_if_empty: Option<bool>,
    pub disable_mouse: Option<bool>,
    pub hard_stop: Option<bool>,
    pub rounded_borders: Option<bool>,
    pub cursor: Option<String>,
    pub highlight_color: Option<String>,
    pub main_border_color: Option<String>,
    pub items_border_color: Option<String>,
    pub input_border_color: Option<String>,
    pub main_text_color: Option<String>,
    pub items_text_color: Option<String>,
    pub input_text_color: Option<String>,
    pub header_title_color: Option<String>,
    pub title_panel_height_percent: Option<u16>,
    pub input_panel_height: Option<u16>,
    #[serde(default, deserialize_with = "deserialize_optional_parsed")]
    pub title_panel_position: Option<PanelPosition>,
}

#[derive(Debug, Deserialize, Clone, Default)]
pub struct CclipConfig {
    pub image_preview: Option<bool>,
    pub hide_inline_image_message: Option<bool>,
    pub show_tag_color_names: Option<bool>,
    pub show_line_numbers: Option<bool>,
    pub wrap_long_lines: Option<bool>,
    pub disable_mouse: Option<bool>,
    pub hard_stop: Option<bool>,
    pub rounded_borders: Option<bool>,
    pub cursor: Option<String>,
    pub highlight_color: Option<String>,
    pub main_border_color: Option<String>,
    pub items_border_color: Option<String>,
    pub input_border_color: Option<String>,
    pub main_text_color: Option<String>,
    pub items_text_color: Option<String>,
    pub input_text_color: Option<String>,
    pub header_title_color: Option<String>,
    pub title_panel_height_percent: Option<u16>,
    pub input_panel_height: Option<u16>,
    #[serde(default, deserialize_with = "deserialize_optional_parsed")]
    pub title_panel_position: Option<PanelPosition>,
}

fn deserialize_parsed_or_default<'de, D, T>(deserializer: D) -> Result<T, D::Error>
where
    D: Deserializer<'de>,
    T: Default + FromStr,
{
    let value = String::deserialize(deserializer)?;
    Ok(value.parse().unwrap_or_default())
}

fn deserialize_optional_parsed<'de, D, T>(deserializer: D) -> Result<Option<T>, D::Error>
where
    D: Deserializer<'de>,
    T: FromStr,
{
    let value = Option::<String>::deserialize(deserializer)?;
    Ok(value.and_then(|entry| entry.parse().ok()))
}
