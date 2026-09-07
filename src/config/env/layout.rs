use super::helpers::{
    INTEGER_EXPECTED, OverrideSource, PANEL_POSITION_EXPECTED, set_optional_parsed, set_parsed,
};
use crate::config::{ConfigError, FselConfig};

pub(super) fn apply(cfg: &mut FselConfig, source: &impl OverrideSource) -> Result<(), ConfigError> {
    set_optional_parsed(
        source,
        "FSEL_PANELS_INFO_POSITION",
        &mut cfg.panels.info_position,
        "top, right, bottom, or left",
    )?;
    set_optional_parsed(
        source,
        "FSEL_PANELS_INPUT_POSITION",
        &mut cfg.panels.input_position,
        "top, right, bottom, or left",
    )?;
    set_optional_parsed(
        source,
        "FSEL_PANELS_INFO_SIZE",
        &mut cfg.panels.info_size,
        INTEGER_EXPECTED,
    )?;
    set_optional_parsed(
        source,
        "FSEL_PANELS_INPUT_SIZE",
        &mut cfg.panels.input_size,
        INTEGER_EXPECTED,
    )?;
    set_parsed(
        source,
        "FSEL_PANELS_ROTATION",
        &mut cfg.panels.rotation,
        INTEGER_EXPECTED,
    )?;
    set_parsed(
        source,
        "FSEL_PANELS_ITEM_WIDTH",
        &mut cfg.panels.item_width,
        INTEGER_EXPECTED,
    )?;
    set_parsed(
        source,
        "FSEL_TITLE_PANEL_HEIGHT_PERCENT",
        &mut cfg.layout.title_panel_height_percent,
        INTEGER_EXPECTED,
    )?;
    set_parsed(
        source,
        "FSEL_INPUT_PANEL_HEIGHT",
        &mut cfg.layout.input_panel_height,
        INTEGER_EXPECTED,
    )?;
    set_parsed(
        source,
        "FSEL_TITLE_PANEL_POSITION",
        &mut cfg.layout.title_panel_position,
        PANEL_POSITION_EXPECTED,
    )?;
    Ok(())
}
