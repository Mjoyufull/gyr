//! Environment overrides for shared selector visuals and input behavior.

use super::helpers::{BOOLEAN_EXPECTED, OverrideSource, set_parsed, set_string};
use crate::config::{ConfigError, FselConfig};

pub(super) fn apply(cfg: &mut FselConfig, source: &impl OverrideSource) -> Result<(), ConfigError> {
    set_string(source, "FSEL_HIGHLIGHT_COLOR", &mut cfg.ui.highlight_color);
    set_string(
        source,
        "FSEL_MAIN_BORDER_COLOR",
        &mut cfg.ui.main_border_color,
    );
    set_string(
        source,
        "FSEL_MAIN_BACKGROUND_COLOR",
        &mut cfg.ui.main_background_color,
    );
    set_string(
        source,
        "FSEL_APPS_BACKGROUND_COLOR",
        &mut cfg.ui.items_background_color,
    );
    set_string(
        source,
        "FSEL_ITEMS_BACKGROUND_COLOR",
        &mut cfg.ui.items_background_color,
    );
    set_string(
        source,
        "FSEL_APPS_BORDER_COLOR",
        &mut cfg.ui.items_border_color,
    );
    set_string(
        source,
        "FSEL_ITEMS_BORDER_COLOR",
        &mut cfg.ui.items_border_color,
    );
    set_string(
        source,
        "FSEL_APPS_SELECTION_BACKGROUND_COLOR",
        &mut cfg.ui.items_selection_background_color,
    );
    set_string(
        source,
        "FSEL_ITEMS_SELECTION_BACKGROUND_COLOR",
        &mut cfg.ui.items_selection_background_color,
    );
    set_string(
        source,
        "FSEL_INPUT_BACKGROUND_COLOR",
        &mut cfg.ui.input_background_color,
    );
    set_string(
        source,
        "FSEL_INPUT_BORDER_COLOR",
        &mut cfg.ui.input_border_color,
    );
    set_string(source, "FSEL_MAIN_TEXT_COLOR", &mut cfg.ui.main_text_color);
    set_string(source, "FSEL_APPS_TEXT_COLOR", &mut cfg.ui.items_text_color);
    set_string(
        source,
        "FSEL_ITEMS_TEXT_COLOR",
        &mut cfg.ui.items_text_color,
    );
    set_string(
        source,
        "FSEL_INPUT_TEXT_COLOR",
        &mut cfg.ui.input_text_color,
    );
    set_string(
        source,
        "FSEL_HEADER_TITLE_COLOR",
        &mut cfg.ui.header_title_color,
    );
    set_string(source, "FSEL_CURSOR", &mut cfg.ui.cursor);
    set_parsed(
        source,
        "FSEL_HARD_STOP",
        &mut cfg.ui.hard_stop,
        BOOLEAN_EXPECTED,
    )?;
    set_parsed(
        source,
        "FSEL_ROUNDED_BORDERS",
        &mut cfg.ui.rounded_borders,
        BOOLEAN_EXPECTED,
    )?;
    set_parsed(
        source,
        "FSEL_SHOW_MAIN_BORDER",
        &mut cfg.ui.show_main_border,
        BOOLEAN_EXPECTED,
    )?;
    set_parsed(
        source,
        "FSEL_SHOW_APPS_BORDER",
        &mut cfg.ui.show_items_border,
        BOOLEAN_EXPECTED,
    )?;
    set_parsed(
        source,
        "FSEL_SHOW_ITEMS_BORDER",
        &mut cfg.ui.show_items_border,
        BOOLEAN_EXPECTED,
    )?;
    set_parsed(
        source,
        "FSEL_SHOW_INPUT_BORDER",
        &mut cfg.ui.show_input_border,
        BOOLEAN_EXPECTED,
    )?;
    set_parsed(
        source,
        "FSEL_SHOW_PANEL_TITLES",
        &mut cfg.ui.show_panel_titles,
        BOOLEAN_EXPECTED,
    )?;
    set_parsed(
        source,
        "FSEL_SHOW_INPUT_COUNT",
        &mut cfg.ui.show_input_count,
        BOOLEAN_EXPECTED,
    )?;
    set_parsed(
        source,
        "FSEL_SHOW_INPUT_PROMPT",
        &mut cfg.ui.show_input_prompt,
        BOOLEAN_EXPECTED,
    )?;
    set_parsed(
        source,
        "FSEL_SHOW_SELECTION_MARKER",
        &mut cfg.ui.show_selection_marker,
        BOOLEAN_EXPECTED,
    )?;
    set_string(
        source,
        "FSEL_SELECTION_MARKER",
        &mut cfg.ui.selection_marker,
    );
    set_parsed(
        source,
        "FSEL_SHOW_PIN_ICONS",
        &mut cfg.ui.show_pin_icons,
        BOOLEAN_EXPECTED,
    )?;
    set_parsed(
        source,
        "FSEL_INPUT_PANEL_STYLE",
        &mut cfg.ui.input_panel_style,
        "classic or command",
    )?;
    set_parsed(
        source,
        "FSEL_APPS_SELECTION_ROUNDED",
        &mut cfg.ui.items_selection_rounded,
        BOOLEAN_EXPECTED,
    )?;
    set_parsed(
        source,
        "FSEL_ITEMS_SELECTION_ROUNDED",
        &mut cfg.ui.items_selection_rounded,
        BOOLEAN_EXPECTED,
    )?;
    set_parsed(
        source,
        "FSEL_DISABLE_MOUSE",
        &mut cfg.ui.disable_mouse,
        BOOLEAN_EXPECTED,
    )?;
    Ok(())
}
