use super::color::string_to_color;
use super::launch::{set_launch_prefix, set_systemd_run, set_uwsm};
use super::types::Opts;
use crate::config::FselConfig;
use crate::ui::PanelPosition;

pub(super) fn apply_config_defaults(default: &mut Opts, fsel_config: &FselConfig) {
    apply_general_config(default, fsel_config);
    apply_app_launcher_overrides(default, fsel_config);
    apply_ui_config(default, fsel_config);
    apply_layout_config(default, fsel_config);
    apply_dmenu_config(default, fsel_config);
    apply_cclip_config(default, fsel_config);
}

fn apply_general_config(default: &mut Opts, fsel_config: &FselConfig) {
    default.terminal_launcher = fsel_config.general.terminal_launcher.clone();
    if default.terminal_launcher == "tty" {
        default.tty = true;
        default.terminal_launcher.clear();
    }
    default.filter_desktop = fsel_config.general.filter_desktop;
    default.list_executables_in_path = fsel_config.general.list_executables_in_path;
    default.hide_before_typing = fsel_config.general.hide_before_typing;
    default.match_mode = fsel_config.general.match_mode;
    default.ranking_mode = fsel_config.general.ranking_mode;
    default.pinned_order_mode = fsel_config.general.pinned_order;
    default.systemd_run = fsel_config.general.systemd_run;
    default.uwsm = fsel_config.general.uwsm;
    default.detach = fsel_config.general.detach;
    default.no_exec = fsel_config.general.no_exec;
    default.confirm_first_launch = fsel_config.general.confirm_first_launch;
    default.prefix_depth = fsel_config.general.prefix_depth;

    if default.systemd_run {
        set_systemd_run(default);
    }
    if default.uwsm {
        set_uwsm(default);
    }
}

fn apply_app_launcher_overrides(default: &mut Opts, fsel_config: &FselConfig) {
    if let Some(filter) = fsel_config.app_launcher.filter_desktop {
        default.filter_desktop = filter;
    }
    if let Some(filter) = fsel_config.app_launcher.filter_actions {
        default.filter_actions = filter;
    }
    if let Some(auto_hide_duplicates) = fsel_config.app_launcher.auto_hide_duplicates {
        default.auto_hide_duplicates = auto_hide_duplicates;
    }
    if let Some(list_exec) = fsel_config.app_launcher.list_executables_in_path {
        default.list_executables_in_path = list_exec;
    }
    if let Some(hide) = fsel_config.app_launcher.hide_before_typing {
        default.hide_before_typing = hide;
    }
    if let Some(prefix) = fsel_config.app_launcher.launch_prefix.clone() {
        set_launch_prefix(default, prefix);
    }
    if let Some(mode) = fsel_config.app_launcher.match_mode {
        default.match_mode = mode;
    }
    if let Some(confirm) = fsel_config.app_launcher.confirm_first_launch {
        default.confirm_first_launch = confirm;
    }
    if let Some(depth) = fsel_config.app_launcher.prefix_depth {
        default.prefix_depth = depth;
    }
    if let Some(ranking_mode) = fsel_config.app_launcher.ranking_mode {
        default.ranking_mode = ranking_mode;
    }
    if let Some(pinned_order_mode) = fsel_config.app_launcher.pinned_order {
        default.pinned_order_mode = pinned_order_mode;
    }
}

fn apply_ui_config(default: &mut Opts, fsel_config: &FselConfig) {
    if let Ok(color) = string_to_color(&fsel_config.ui.highlight_color) {
        default.highlight_color = color;
    }
    default.cursor = fsel_config.ui.cursor.clone();
    default.hard_stop = fsel_config.ui.hard_stop;
    default.rounded_borders = fsel_config.ui.rounded_borders;
    default.disable_mouse = fsel_config.ui.disable_mouse;
    if let Ok(color) = string_to_color(&fsel_config.ui.main_border_color) {
        default.main_border_color = color;
    }
    if let Ok(color) = string_to_color(&fsel_config.ui.apps_border_color) {
        default.apps_border_color = color;
    }
    if let Ok(color) = string_to_color(&fsel_config.ui.input_border_color) {
        default.input_border_color = color;
    }
    if let Ok(color) = string_to_color(&fsel_config.ui.main_text_color) {
        default.main_text_color = color;
    }
    if let Ok(color) = string_to_color(&fsel_config.ui.apps_text_color) {
        default.apps_text_color = color;
    }
    if let Ok(color) = string_to_color(&fsel_config.ui.input_text_color) {
        default.input_text_color = color;
    }
    default.fancy_mode = fsel_config.ui.fancy_mode;
    if let Ok(color) = string_to_color(&fsel_config.ui.header_title_color) {
        default.header_title_color = color;
    }
    if let Ok(color) = string_to_color(&fsel_config.ui.pin_color) {
        default.pin_color = color;
    }
    default.pin_icon = fsel_config.ui.pin_icon.clone();
    default.keybinds = fsel_config.ui.keybinds.clone();
}

fn apply_layout_config(default: &mut Opts, fsel_config: &FselConfig) {
    default.title_panel_height_percent = fsel_config.layout.title_panel_height_percent;
    default.input_panel_height = fsel_config.layout.input_panel_height;
    default.title_panel_position = Some(fsel_config.layout.title_panel_position);
}

fn apply_dmenu_config(default: &mut Opts, fsel_config: &FselConfig) {
    if let Some(delimiter) = fsel_config.dmenu.delimiter.as_deref() {
        default.dmenu_delimiter = delimiter.to_string();
    }
    if let Some(character) = fsel_config.dmenu.password_character.as_deref() {
        default.dmenu_password_character = character.to_string();
    }
    default.dmenu_preview = fsel_config.dmenu.preview.clone();
    if let Some(show_line_numbers) = fsel_config.dmenu.show_line_numbers {
        default.dmenu_show_line_numbers = show_line_numbers;
    }
    if let Some(wrap_long_lines) = fsel_config.dmenu.wrap_long_lines {
        default.dmenu_wrap_long_lines = wrap_long_lines;
    }
    if let Some(exit_if_empty) = fsel_config.dmenu.exit_if_empty {
        default.dmenu_exit_if_empty = exit_if_empty;
    }
    if let Some(disable_mouse) = fsel_config.dmenu.disable_mouse {
        default.dmenu_disable_mouse = Some(disable_mouse);
    }
    if let Some(hard_stop) = fsel_config.dmenu.hard_stop {
        default.dmenu_hard_stop = Some(hard_stop);
    }
    if let Some(rounded_borders) = fsel_config.dmenu.rounded_borders {
        default.dmenu_rounded_borders = Some(rounded_borders);
    }
    if let Some(cursor) = fsel_config.dmenu.cursor.as_deref() {
        default.dmenu_cursor = Some(cursor.to_string());
    }
    default.dmenu_highlight_color =
        parse_optional_color(fsel_config.dmenu.highlight_color.as_deref());
    default.dmenu_main_border_color =
        parse_optional_color(fsel_config.dmenu.main_border_color.as_deref());
    default.dmenu_items_border_color =
        parse_optional_color(fsel_config.dmenu.items_border_color.as_deref());
    default.dmenu_input_border_color =
        parse_optional_color(fsel_config.dmenu.input_border_color.as_deref());
    default.dmenu_main_text_color =
        parse_optional_color(fsel_config.dmenu.main_text_color.as_deref());
    default.dmenu_items_text_color =
        parse_optional_color(fsel_config.dmenu.items_text_color.as_deref());
    default.dmenu_input_text_color =
        parse_optional_color(fsel_config.dmenu.input_text_color.as_deref());
    default.dmenu_header_title_color =
        parse_optional_color(fsel_config.dmenu.header_title_color.as_deref());
    default.dmenu_title_panel_height_percent = fsel_config.dmenu.title_panel_height_percent;
    default.dmenu_input_panel_height = fsel_config.dmenu.input_panel_height;
    default.dmenu_title_panel_position =
        parse_mode_panel_position(fsel_config.dmenu.title_panel_position);
}

fn apply_cclip_config(default: &mut Opts, fsel_config: &FselConfig) {
    default.cclip_image_preview = fsel_config.cclip.image_preview;
    default.cclip_hide_inline_image_message = fsel_config.cclip.hide_inline_image_message;
    default.cclip_show_tag_color_names = fsel_config.cclip.show_tag_color_names;
    default.cclip_show_line_numbers = fsel_config.cclip.show_line_numbers;
    default.cclip_wrap_long_lines = fsel_config.cclip.wrap_long_lines;
    default.cclip_disable_mouse = fsel_config.cclip.disable_mouse;
    default.cclip_hard_stop = fsel_config.cclip.hard_stop;
    default.cclip_rounded_borders = fsel_config.cclip.rounded_borders;
    if let Some(cursor) = fsel_config.cclip.cursor.as_deref() {
        default.cclip_cursor = Some(cursor.to_string());
    }
    default.cclip_highlight_color =
        parse_optional_color(fsel_config.cclip.highlight_color.as_deref());
    default.cclip_main_border_color =
        parse_optional_color(fsel_config.cclip.main_border_color.as_deref());
    default.cclip_items_border_color =
        parse_optional_color(fsel_config.cclip.items_border_color.as_deref());
    default.cclip_input_border_color =
        parse_optional_color(fsel_config.cclip.input_border_color.as_deref());
    default.cclip_main_text_color =
        parse_optional_color(fsel_config.cclip.main_text_color.as_deref());
    default.cclip_items_text_color =
        parse_optional_color(fsel_config.cclip.items_text_color.as_deref());
    default.cclip_input_text_color =
        parse_optional_color(fsel_config.cclip.input_text_color.as_deref());
    default.cclip_header_title_color =
        parse_optional_color(fsel_config.cclip.header_title_color.as_deref());
    default.cclip_title_panel_height_percent = fsel_config.cclip.title_panel_height_percent;
    default.cclip_input_panel_height = fsel_config.cclip.input_panel_height;
    default.cclip_title_panel_position =
        parse_mode_panel_position(fsel_config.cclip.title_panel_position);
}

fn parse_optional_color(value: Option<&str>) -> Option<ratatui::style::Color> {
    value.and_then(|color| string_to_color(color).ok())
}

fn parse_mode_panel_position(value: Option<PanelPosition>) -> Option<PanelPosition> {
    value
}

#[cfg(test)]
mod tests {
    use super::apply_config_defaults;
    use crate::cli::Opts;
    use crate::config::FselConfig;
    use crate::ui::PanelPosition;

    #[test]
    fn mode_specific_panel_positions_preserve_all_explicit_values() {
        let mut config = FselConfig::default();
        config.dmenu.title_panel_position = Some(PanelPosition::Middle);
        config.cclip.title_panel_position = Some(PanelPosition::Top);

        let mut opts = Opts::default();
        apply_config_defaults(&mut opts, &config);

        assert_eq!(opts.dmenu_title_panel_position, Some(PanelPosition::Middle));
        assert_eq!(opts.cclip_title_panel_position, Some(PanelPosition::Top));
    }

    #[test]
    fn app_launcher_filter_actions_override_beats_general_default() {
        let mut config = FselConfig::default();
        config.app_launcher.filter_actions = Some(true);

        let mut opts = Opts::default();
        apply_config_defaults(&mut opts, &config);

        assert!(opts.filter_actions);
    }

    #[test]
    fn app_launcher_auto_hide_duplicates_is_opt_in() {
        let mut opts = Opts::default();
        apply_config_defaults(&mut opts, &FselConfig::default());
        assert!(!opts.auto_hide_duplicates);

        let mut config = FselConfig::default();
        config.app_launcher.auto_hide_duplicates = Some(true);
        apply_config_defaults(&mut opts, &config);

        assert!(opts.auto_hide_duplicates);
    }
}
