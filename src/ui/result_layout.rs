//! Visible result slots, rotation, and pointer hit testing in one coordinate system.

use ratatui::layout::{Position, Rect};

#[derive(Debug, Clone, Copy)]
pub(crate) struct ResultLayout {
    area: Rect,
    width: u16,
    height: u16,
    horizontal: bool,
    reverse: bool,
}

impl ResultLayout {
    pub(crate) fn keep_visible(self, selected: Option<usize>, offset: &mut usize) {
        if let Some(index) = selected {
            if index < *offset {
                *offset = index;
            }
            let capacity = self.capacity();
            if capacity > 0 && index >= offset.saturating_add(capacity) {
                *offset = index.saturating_sub(capacity - 1);
            }
        }
    }
    pub(crate) fn new(area: Rect, row_height: u16, settings: &super::PanelSettings) -> Self {
        let horizontal = settings.horizontal();
        Self {
            area,
            width: if horizontal {
                settings.item_width.min(area.width).max(1)
            } else {
                area.width.max(1)
            },
            height: row_height.max(1),
            horizontal,
            reverse: settings.rotation >= 180,
        }
    }

    pub(crate) fn capacity(self) -> usize {
        if self.area.width == 0 || self.area.height < self.height {
            return 0;
        }
        usize::from(if self.horizontal {
            self.area.width / self.width
        } else {
            self.area.height / self.height
        })
    }

    pub(crate) fn slot(self, index: usize) -> Rect {
        let capacity = self.capacity();
        if index >= capacity {
            return Rect::default();
        }
        let physical = if self.reverse {
            capacity - 1 - index
        } else {
            index
        } as u16;
        Rect::new(
            self.area.x
                + if self.horizontal {
                    physical * self.width
                } else {
                    0
                },
            self.area.y
                + if self.horizontal {
                    0
                } else {
                    physical * self.height
                },
            self.width.min(self.area.width),
            self.height,
        )
    }

    pub(crate) fn hit(self, column: u16, row: u16) -> Option<usize> {
        let position = Position::new(column, row);
        if !self.area.contains(position) {
            return None;
        }
        let physical = usize::from(if self.horizontal {
            if row >= self.area.y + self.height {
                return None;
            }
            (column - self.area.x) / self.width
        } else {
            (row - self.area.y) / self.height
        });
        let capacity = self.capacity();
        if physical >= capacity {
            return None;
        }
        Some(if self.reverse {
            capacity - 1 - physical
        } else {
            physical
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pointer_and_rendering_agree_in_all_orientations() {
        for rotation in [0, 90, 180, 270] {
            let settings = super::super::PanelSettings {
                rotation,
                ..Default::default()
            };
            let layout = ResultLayout::new(Rect::new(8, 5, 80, 30), 3, &settings);
            for index in 0..layout.capacity() {
                let slot = layout.slot(index);
                assert_eq!(layout.hit(slot.x, slot.y), Some(index));
            }
            assert_eq!(layout.hit(0, 0), None);
        }
    }
}
