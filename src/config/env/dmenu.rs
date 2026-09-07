use super::helpers::{
    BOOLEAN_EXPECTED, INTEGER_EXPECTED, OverrideSource, PANEL_POSITION_EXPECTED,
    set_optional_parsed, set_optional_string,
};
use crate::config::{ConfigError, FselConfig};

pub(super) fn apply(cfg: &mut FselConfig, source: &impl OverrideSource) -> Result<(), ConfigError> {
    set_optional_string(source, "FSEL_DMENU_DELIMITER", &mut cfg.dmenu.delimiter);
    set_optional_string(source, "FSEL_DMENU_PREVIEW", &mut cfg.dmenu.preview);
    set_optional_string(
        source,
        "FSEL_DMENU_PASSWORD_CHARACTER",
        &mut cfg.dmenu.password_character,
    );
    set_optional_parsed(
        source,
        "FSEL_DMENU_SHOW_LINE_NUMBERS",
        &mut cfg.dmenu.show_line_numbers,
        BOOLEAN_EXPECTED,
    )?;
    set_optional_parsed(
        source,
        "FSEL_DMENU_WRAP_LONG_LINES",
        &mut cfg.dmenu.wrap_long_lines,
        BOOLEAN_EXPECTED,
    )?;
    set_optional_parsed(
        source,
        "FSEL_DMENU_EXIT_IF_EMPTY",
        &mut cfg.dmenu.exit_if_empty,
        BOOLEAN_EXPECTED,
    )?;
    set_optional_parsed(
        source,
        "FSEL_DMENU_DISABLE_MOUSE",
        &mut cfg.dmenu.disable_mouse,
        BOOLEAN_EXPECTED,
    )?;
    set_optional_parsed(
        source,
        "FSEL_DMENU_HARD_STOP",
        &mut cfg.dmenu.hard_stop,
        BOOLEAN_EXPECTED,
    )?;
    set_optional_parsed(
        source,
        "FSEL_DMENU_ROUNDED_BORDERS",
        &mut cfg.dmenu.rounded_borders,
        BOOLEAN_EXPECTED,
    )?;
    set_optional_string(source, "FSEL_DMENU_CURSOR", &mut cfg.dmenu.cursor);
    set_optional_string(
        source,
        "FSEL_DMENU_HIGHLIGHT_COLOR",
        &mut cfg.dmenu.highlight_color,
    );
    set_optional_string(
        source,
        "FSEL_DMENU_MAIN_BORDER_COLOR",
        &mut cfg.dmenu.main_border_color,
    );
    set_optional_string(
        source,
        "FSEL_DMENU_ITEMS_BORDER_COLOR",
        &mut cfg.dmenu.items_border_color,
    );
    set_optional_string(
        source,
        "FSEL_DMENU_INPUT_BORDER_COLOR",
        &mut cfg.dmenu.input_border_color,
    );
    set_optional_string(
        source,
        "FSEL_DMENU_MAIN_TEXT_COLOR",
        &mut cfg.dmenu.main_text_color,
    );
    set_optional_string(
        source,
        "FSEL_DMENU_ITEMS_TEXT_COLOR",
        &mut cfg.dmenu.items_text_color,
    );
    set_optional_string(
        source,
        "FSEL_DMENU_INPUT_TEXT_COLOR",
        &mut cfg.dmenu.input_text_color,
    );
    set_optional_string(
        source,
        "FSEL_DMENU_HEADER_TITLE_COLOR",
        &mut cfg.dmenu.header_title_color,
    );
    set_optional_parsed(
        source,
        "FSEL_DMENU_TITLE_PANEL_HEIGHT_PERCENT",
        &mut cfg.dmenu.title_panel_height_percent,
        INTEGER_EXPECTED,
    )?;
    set_optional_parsed(
        source,
        "FSEL_DMENU_INPUT_PANEL_HEIGHT",
        &mut cfg.dmenu.input_panel_height,
        INTEGER_EXPECTED,
    )?;
    set_optional_parsed(
        source,
        "FSEL_DMENU_TITLE_PANEL_POSITION",
        &mut cfg.dmenu.title_panel_position,
        PANEL_POSITION_EXPECTED,
    )?;
    Ok(())
}
