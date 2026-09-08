use super::PanelPosition;
use ratatui::layout::{Constraint, Direction, Layout, Rect};

/// Fixed three-panel layout metadata used by dmenu-like modes.
pub(crate) struct PanelLayout {
    pub chunks: [Rect; 3],
    pub content_panel_index: usize,
    pub items_panel_index: usize,
    pub input_panel_index: usize,
}

pub(crate) fn effective_content_height(total_height: u16, content_panel_percent: u16) -> u16 {
    if total_height == 0 || content_panel_percent == 0 {
        0
    } else {
        ((total_height as f32 * content_panel_percent as f32 / 100.0).round() as u16)
            .max(3)
            .min(total_height)
    }
}

#[cfg(test)]
pub(crate) fn items_panel_height(
    total_height: u16,
    content_height: u16,
    input_panel_height: u16,
) -> u16 {
    total_height.saturating_sub(content_height.saturating_add(input_panel_height))
}

#[cfg(test)]
pub(crate) fn items_panel_bounds(
    total_height: u16,
    content_height: u16,
    input_panel_height: u16,
    position: PanelPosition,
) -> (u16, u16) {
    let items_height = items_panel_height(total_height, content_height, input_panel_height);
    match position {
        PanelPosition::Top => (content_height, items_height),
        PanelPosition::Middle | PanelPosition::Bottom => (0, items_height),
    }
}

pub(crate) fn split_content_panels(
    area: Rect,
    content_height: u16,
    input_panel_height: u16,
    position: PanelPosition,
) -> PanelLayout {
    match position {
        PanelPosition::Top => {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(content_height),
                    Constraint::Min(1),
                    Constraint::Length(input_panel_height),
                ])
                .split(area);
            PanelLayout {
                chunks: [chunks[0], chunks[1], chunks[2]],
                content_panel_index: 0,
                items_panel_index: 1,
                input_panel_index: 2,
            }
        }
        PanelPosition::Middle => {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Min(1),
                    Constraint::Length(content_height),
                    Constraint::Length(input_panel_height),
                ])
                .split(area);
            PanelLayout {
                chunks: [chunks[0], chunks[1], chunks[2]],
                content_panel_index: 1,
                items_panel_index: 0,
                input_panel_index: 2,
            }
        }
        PanelPosition::Bottom => {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Min(1),
                    Constraint::Length(input_panel_height),
                    Constraint::Length(content_height),
                ])
                .split(area);
            PanelLayout {
                chunks: [chunks[0], chunks[1], chunks[2]],
                content_panel_index: 2,
                items_panel_index: 0,
                input_panel_index: 1,
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        PanelLayout, effective_content_height, items_panel_bounds, items_panel_height,
        split_content_panels,
    };
    use crate::ui::PanelPosition;
    use ratatui::layout::Rect;

    #[test]
    fn effective_content_height_allows_zero() {
        assert_eq!(effective_content_height(40, 0), 0);
    }

    #[test]
    fn effective_content_height_keeps_visible_panels_usable() {
        assert_eq!(effective_content_height(20, 1), 3);
    }

    #[test]
    fn effective_content_height_respects_zero_and_small_totals() {
        assert_eq!(effective_content_height(0, 50), 0);
        assert_eq!(effective_content_height(2, 50), 2);
    }

    #[test]
    fn items_panel_height_saturates_when_panels_exceed_total_height() {
        assert_eq!(items_panel_height(10, u16::MAX, u16::MAX), 0);
    }

    #[test]
    fn items_panel_height_matches_normal_layout_math() {
        assert_eq!(items_panel_height(30, 10, 3), 17);
    }

    #[test]
    fn items_panel_bounds_follow_panel_position() {
        assert_eq!(items_panel_bounds(30, 10, 3, PanelPosition::Top), (10, 17));
        assert_eq!(
            items_panel_bounds(30, 10, 3, PanelPosition::Middle),
            (0, 17)
        );
        assert_eq!(
            items_panel_bounds(30, 10, 3, PanelPosition::Bottom),
            (0, 17)
        );
    }

    #[test]
    fn split_content_panels_assigns_expected_indexes() {
        let PanelLayout {
            chunks,
            content_panel_index,
            items_panel_index,
            input_panel_index,
        } = split_content_panels(Rect::new(0, 0, 80, 30), 10, 3, PanelPosition::Bottom);

        assert_eq!(content_panel_index, 2);
        assert_eq!(items_panel_index, 0);
        assert_eq!(input_panel_index, 1);
        assert_eq!(chunks[content_panel_index].height, 10);
        assert_eq!(chunks[input_panel_index].height, 3);
    }
}
