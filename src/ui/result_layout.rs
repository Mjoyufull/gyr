//! Visible result slots, rotation, and pointer hit testing in one coordinate system.

use ratatui::layout::{Position, Rect};

#[derive(Debug, Clone, Copy)]
pub(crate) struct ResultLayout {
    area: Rect,
    width: u16,
    height: u16,
    horizontal: bool,
    grid_columns: Option<u16>,
    reverse: bool,
}

impl ResultLayout {
    pub(crate) fn keep_visible(self, selected: Option<usize>, offset: &mut usize) {
        if let Some(index) = selected {
            let step = self.scroll_step();
            *offset = *offset / step * step;
            if index < *offset {
                *offset = index / step * step;
            }
            let capacity = self.capacity();
            if capacity > 0 && index >= offset.saturating_add(capacity) {
                *offset = (index - capacity + step) / step * step;
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
            grid_columns: None,
            reverse: settings.rotation >= 180,
        }
    }

    pub(crate) fn grid(
        area: Rect,
        row_height: u16,
        columns: u16,
        settings: &super::PanelSettings,
    ) -> Self {
        let columns = columns.max(1).min((area.width / 8).max(1)).min(area.width);
        let mut layout = Self::new(area, row_height, settings);
        layout.width = area.width.checked_div(columns).unwrap_or(1);
        layout.grid_columns = Some(columns);
        layout
    }

    pub(crate) fn scroll_step(self) -> usize {
        self.grid_columns.map_or(1, |columns| {
            if self.horizontal {
                usize::from(self.area.height / self.height).max(1)
            } else {
                usize::from(columns).max(1)
            }
        })
    }

    pub(crate) fn navigation_step(self, vertical: bool) -> usize {
        if self.grid_columns.is_some() && vertical != self.horizontal {
            self.scroll_step()
        } else {
            1
        }
    }

    pub(crate) fn capacity(self) -> usize {
        if let Some(columns) = self.grid_columns {
            return usize::from(columns) * usize::from(self.area.height / self.height);
        }
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
        };
        if let Some(columns) = self.grid_columns {
            let rows = usize::from(self.area.height / self.height);
            let columns = usize::from(columns);
            let (column, row) = if self.horizontal {
                (physical / rows, physical % rows)
            } else {
                (physical % columns, physical / columns)
            };
            return Rect::new(
                self.area.x + column as u16 * self.width,
                self.area.y + row as u16 * self.height,
                self.width,
                self.height,
            );
        }
        Rect::new(
            self.area.x
                + if self.horizontal {
                    physical as u16 * self.width
                } else {
                    0
                },
            self.area.y
                + if self.horizontal {
                    0
                } else {
                    physical as u16 * self.height
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
        if let Some(columns) = self.grid_columns {
            let x = (column - self.area.x) / self.width;
            let y = (row - self.area.y) / self.height;
            let rows = self.area.height / self.height;
            if x >= columns || y >= rows {
                return None;
            }
            let physical = if self.horizontal {
                usize::from(x) * usize::from(rows) + usize::from(y)
            } else {
                usize::from(y) * usize::from(columns) + usize::from(x)
            };
            return Some(if self.reverse {
                self.capacity() - 1 - physical
            } else {
                physical
            });
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
    fn grid_slots_and_scroll_offsets_remain_aligned_when_resized() {
        for rotation in [0, 90, 180, 270] {
            let settings = super::super::PanelSettings {
                rotation,
                ..Default::default()
            };
            for width in [0, 1, 7, 19, 80] {
                let layout = ResultLayout::grid(Rect::new(3, 5, width, 21), 4, 4, &settings);
                for index in 0..layout.capacity() {
                    let area = layout.slot(index);
                    assert_eq!(layout.hit(area.x, area.y), Some(index));
                    assert_eq!(layout.hit(area.right() - 1, area.bottom() - 1), Some(index));
                }
                if layout.capacity() > 0 {
                    let mut offset = 0;
                    layout.keep_visible(Some(41), &mut offset);
                    assert_eq!(offset % layout.scroll_step(), 0);
                    assert!(offset <= 41 && 41 < offset + layout.capacity());
                }
                assert_eq!(layout.hit(3 + width, 5), None);
            }
        }
    }

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
