use super::helpers::{
    BOOLEAN_EXPECTED, DESKTOP_ICON_MODE_EXPECTED, HORIZONTAL_POSITION_EXPECTED, INTEGER_EXPECTED,
    MATCH_MODE_EXPECTED, OverrideSource, PINNED_ORDER_EXPECTED, RANKING_MODE_EXPECTED,
    set_optional_launch_prefix, set_optional_parsed, set_optional_string,
};
use crate::config::{ConfigError, FselConfig};

pub(super) fn apply(cfg: &mut FselConfig, source: &impl OverrideSource) -> Result<(), ConfigError> {
    set_optional_parsed(
        source,
        "FSEL_APP_LAUNCHER_FILTER_DESKTOP",
        &mut cfg.app_launcher.filter_desktop,
        BOOLEAN_EXPECTED,
    )?;
    set_optional_parsed(
        source,
        "FSEL_APP_LAUNCHER_FILTER_ACTIONS",
        &mut cfg.app_launcher.filter_actions,
        BOOLEAN_EXPECTED,
    )?;
    set_optional_parsed(
        source,
        "FSEL_APP_LAUNCHER_AUTO_HIDE_DUPLICATES",
        &mut cfg.app_launcher.auto_hide_duplicates,
        BOOLEAN_EXPECTED,
    )?;
    set_optional_parsed(
        source,
        "FSEL_APP_LAUNCHER_LIST_EXECUTABLES_IN_PATH",
        &mut cfg.app_launcher.list_executables_in_path,
        BOOLEAN_EXPECTED,
    )?;
    set_optional_parsed(
        source,
        "FSEL_APP_LAUNCHER_HIDE_BEFORE_TYPING",
        &mut cfg.app_launcher.hide_before_typing,
        BOOLEAN_EXPECTED,
    )?;
    set_optional_launch_prefix(
        source,
        "FSEL_APP_LAUNCHER_LAUNCH_PREFIX",
        &mut cfg.app_launcher.launch_prefix,
    )?;
    set_optional_parsed(
        source,
        "FSEL_APP_LAUNCHER_MATCH_MODE",
        &mut cfg.app_launcher.match_mode,
        MATCH_MODE_EXPECTED,
    )?;
    set_optional_parsed(
        source,
        "FSEL_APP_LAUNCHER_RANKING_MODE",
        &mut cfg.app_launcher.ranking_mode,
        RANKING_MODE_EXPECTED,
    )?;
    set_optional_parsed(
        source,
        "FSEL_APP_LAUNCHER_PINNED_ORDER",
        &mut cfg.app_launcher.pinned_order,
        PINNED_ORDER_EXPECTED,
    )?;
    set_optional_parsed(
        source,
        "FSEL_APP_LAUNCHER_CONFIRM_FIRST_LAUNCH",
        &mut cfg.app_launcher.confirm_first_launch,
        BOOLEAN_EXPECTED,
    )?;
    set_optional_parsed(
        source,
        "FSEL_APP_LAUNCHER_PREFIX_DEPTH",
        &mut cfg.app_launcher.prefix_depth,
        INTEGER_EXPECTED,
    )?;
    set_optional_parsed(
        source,
        "FSEL_APP_LAUNCHER_ICON_MODE",
        &mut cfg.app_launcher.icon_mode,
        DESKTOP_ICON_MODE_EXPECTED,
    )?;
    set_optional_parsed(
        source,
        "FSEL_APP_LAUNCHER_ICON_POSITION",
        &mut cfg.app_launcher.icon_position,
        HORIZONTAL_POSITION_EXPECTED,
    )?;
    set_optional_parsed(
        source,
        "FSEL_APP_LAUNCHER_ICON_PREVIEW_WIDTH_PERCENT",
        &mut cfg.app_launcher.icon_preview_width_percent,
        INTEGER_EXPECTED,
    )?;
    set_optional_parsed(
        source,
        "FSEL_APP_LAUNCHER_ICON_LIST_WIDTH",
        &mut cfg.app_launcher.icon_list_width,
        INTEGER_EXPECTED,
    )?;
    set_optional_parsed(
        source,
        "FSEL_APP_LAUNCHER_ICON_LIST_HEIGHT",
        &mut cfg.app_launcher.icon_list_height,
        INTEGER_EXPECTED,
    )?;
    set_optional_parsed(
        source,
        "FSEL_APP_LAUNCHER_ICON_LIST_GAP",
        &mut cfg.app_launcher.icon_list_gap,
        INTEGER_EXPECTED,
    )?;
    set_optional_parsed(
        source,
        "FSEL_APP_LAUNCHER_ICON_LIST_VERTICAL_ALIGN_PERCENT",
        &mut cfg.app_launcher.icon_list_vertical_align_percent,
        INTEGER_EXPECTED,
    )?;
    set_optional_parsed(
        source,
        "FSEL_APP_LAUNCHER_ICON_ARROW_BEFORE",
        &mut cfg.app_launcher.icon_arrow_before,
        BOOLEAN_EXPECTED,
    )?;
    set_optional_parsed(
        source,
        "FSEL_APP_LAUNCHER_ICON_SIZE",
        &mut cfg.app_launcher.icon_size,
        INTEGER_EXPECTED,
    )?;
    set_optional_parsed(
        source,
        "FSEL_APP_LAUNCHER_ICON_HORIZONTAL_ALIGN_PERCENT",
        &mut cfg.app_launcher.icon_horizontal_align_percent,
        INTEGER_EXPECTED,
    )?;
    set_optional_parsed(
        source,
        "FSEL_APP_LAUNCHER_ICON_VERTICAL_ALIGN_PERCENT",
        &mut cfg.app_launcher.icon_vertical_align_percent,
        INTEGER_EXPECTED,
    )?;
    set_optional_string(
        source,
        "FSEL_APP_LAUNCHER_ICON_THEME",
        &mut cfg.app_launcher.icon_theme,
    );
    Ok(())
}
