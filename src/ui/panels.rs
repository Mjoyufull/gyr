//! Panel docking around the result list. Geometry is shared by drawing and hit testing.
//! Quarter turns move panel edges and the list axis; terminal text stays upright.

use ratatui::layout::{Constraint, Layout, Rect};
use serde::Deserialize;
use std::str::FromStr;

/// Edge of the result list occupied by a supporting panel.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PanelSide {
    /// Above the results.
    Top,
    /// To the right of the results.
    Right,
    /// Below the results.
    Bottom,
    /// To the left of the results.
    Left,
}

impl FromStr for PanelSide {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "top" => Ok(Self::Top),
            "right" => Ok(Self::Right),
            "bottom" => Ok(Self::Bottom),
            "left" => Ok(Self::Left),
            _ => Err("panel side must be top, right, bottom, or left".to_owned()),
        }
    }
}

impl PanelSide {
    pub(crate) fn rotated(self, rotation: u16) -> Self {
        let index = match self {
            Self::Top => 0,
            Self::Right => 1,
            Self::Bottom => 2,
            Self::Left => 3,
        };
        [Self::Top, Self::Right, Self::Bottom, Self::Left][(index + usize::from(rotation / 90)) % 4]
    }

    pub(crate) fn horizontal(self) -> bool {
        matches!(self, Self::Left | Self::Right)
    }
}

/// Optional docking controls; omitted positions preserve legacy layouts.
#[derive(Debug, Clone, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct PanelSettings {
    /// Override the information panel edge.
    pub info_position: Option<PanelSide>,
    /// Override the input panel edge.
    pub input_position: Option<PanelSide>,
    /// Information panel's percentage of the terminal along its docking axis.
    pub info_size: Option<u16>,
    /// Input panel thickness in terminal cells along its docking axis.
    pub input_size: Option<u16>,
    /// Clockwise quarter turn: 0, 90, 180, or 270 degrees.
    pub rotation: u16,
    /// Width of each result in a horizontal list.
    pub item_width: u16,
}

impl Default for PanelSettings {
    fn default() -> Self {
        Self {
            info_position: None,
            input_position: None,
            info_size: None,
            input_size: None,
            rotation: 0,
            item_width: 24,
        }
    }
}

impl PanelSettings {
    pub(crate) fn validate(&self) -> Result<(), String> {
        if !matches!(self.rotation, 0 | 90 | 180 | 270) {
            return Err("layout rotation must be 0, 90, 180, or 270".to_owned());
        }
        if self.info_size.is_some_and(|size| size > 90) {
            return Err("info size must be between 0 and 90 percent".to_owned());
        }
        if self.item_width == 0 {
            return Err("item width must be at least 1 column".to_owned());
        }
        Ok(())
    }

    pub(crate) fn enabled(&self) -> bool {
        self.info_position.is_some()
            || self.input_position.is_some()
            || self.info_size.is_some()
            || self.input_size.is_some()
            || self.rotation != 0
    }

    pub(crate) fn horizontal(&self) -> bool {
        matches!(self.rotation, 90 | 270)
    }

    pub(crate) fn split(
        &self,
        area: Rect,
        info_percent: u16,
        input_cells: u16,
        legacy_position: super::PanelPosition,
    ) -> (Rect, Rect, Rect) {
        let info_side = self
            .info_position
            .unwrap_or(match legacy_position {
                super::PanelPosition::Top => PanelSide::Top,
                super::PanelPosition::Middle | super::PanelPosition::Bottom => PanelSide::Bottom,
            })
            .rotated(self.rotation);
        let input_side = self
            .input_position
            .unwrap_or(PanelSide::Bottom)
            .rotated(self.rotation);
        let total = if info_side.horizontal() {
            area.width
        } else {
            area.height
        };
        let info_cells = ((u32::from(total) * u32::from(self.info_size.unwrap_or(info_percent))
            + 50)
            / 100) as u16;
        let (info, remaining) = dock(area, info_side, info_cells);
        let (input, items) = dock(
            remaining,
            input_side,
            self.input_size.unwrap_or(input_cells),
        );
        (info, input, items)
    }
}

/// Reserve an edge while retaining at least one cell for results when possible.
pub(crate) fn dock(area: Rect, side: PanelSide, cells: u16) -> (Rect, Rect) {
    let total = if side.horizontal() {
        area.width
    } else {
        area.height
    };
    let length = cells.min(total.saturating_sub(1));
    let before = matches!(side, PanelSide::Top | PanelSide::Left);
    let constraints = if before {
        [Constraint::Length(length), Constraint::Min(0)]
    } else {
        [Constraint::Min(0), Constraint::Length(length)]
    };
    let layout = if side.horizontal() {
        Layout::horizontal(constraints)
    } else {
        Layout::vertical(constraints)
    };
    let chunks = layout.split(area);
    if before {
        (chunks[0], chunks[1])
    } else {
        (chunks[1], chunks[0])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn docked_panels_do_not_overlap_results_at_any_rotation() {
        for rotation in [0, 90, 180, 270] {
            for width in [0, 1, 2, 80] {
                let area = Rect::new(3, 4, width, 24);
                let settings = PanelSettings {
                    rotation,
                    ..PanelSettings::default()
                };
                let (info, input, items) =
                    settings.split(area, 30, 3, super::super::PanelPosition::Top);
                assert_eq!(info.intersection(items).area(), 0);
                assert_eq!(input.intersection(items).area(), 0);
                assert_eq!(info.area() + input.area() + items.area(), area.area());
            }
        }
    }
}
