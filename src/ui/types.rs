use serde::Deserialize;
use std::str::FromStr;

/// Title panel position
#[derive(Debug, Deserialize, Clone, Copy, PartialEq, Default)]
#[serde(rename_all = "lowercase")]
pub enum PanelPosition {
    /// Panel at the top (default behavior)
    #[default]
    Top,
    /// Panel in the middle (where results/apps usually are)
    Middle,
    /// Panel at the bottom (above input field)
    Bottom,
}

impl FromStr for PanelPosition {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "top" => Ok(PanelPosition::Top),
            "middle" => Ok(PanelPosition::Middle),
            "bottom" => Ok(PanelPosition::Bottom),
            _ => Err(format!(
                "Invalid panel position: '{}'. Valid options: top, middle, bottom",
                s
            )),
        }
    }
}

/// Horizontal placement within a panel or list row.
#[derive(Debug, Deserialize, Clone, Copy, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub enum HorizontalPosition {
    /// Place content on the left.
    Left,
    /// Place content on the right.
    #[default]
    Right,
}

impl FromStr for HorizontalPosition {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value.trim().to_lowercase().as_str() {
            "left" => Ok(Self::Left),
            "right" => Ok(Self::Right),
            _ => Err(format!(
                "Invalid horizontal position: '{value}'. Valid options: left, right"
            )),
        }
    }
}
