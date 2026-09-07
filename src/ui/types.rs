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
    #[default]
    Left,
    /// Place preview content in the center.
    Center,
    /// Place content on the right.
    Right,
}

impl FromStr for HorizontalPosition {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value.trim().to_lowercase().as_str() {
            "left" => Ok(Self::Left),
            "center" => Ok(Self::Center),
            "right" => Ok(Self::Right),
            _ => Err(format!(
                "Invalid horizontal position: '{value}'. Valid options: left, center, right"
            )),
        }
    }
}

/// Visual design used by the launcher input panel.
#[derive(Debug, Deserialize, Clone, Copy, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub enum InputPanelStyle {
    /// Preserve the original bordered prompt and inline counter.
    #[default]
    Classic,
    /// Use an accent rail with the count and key hints in a footer.
    Command,
}

impl FromStr for InputPanelStyle {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value.trim().to_lowercase().as_str() {
            "classic" => Ok(Self::Classic),
            "command" => Ok(Self::Command),
            _ => Err(format!(
                "Invalid input panel style: '{value}'. Valid options: classic, command"
            )),
        }
    }
}
