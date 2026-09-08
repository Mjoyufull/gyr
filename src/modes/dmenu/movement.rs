//! Opt-in, session-only panel docking controls; selector data remains untouched.

use super::options::DmenuOptions;
use crate::ui::InputEvent;
use crate::ui::panels::PanelSide;
use crossterm::event::{KeyCode, KeyModifiers, MouseButton, MouseEventKind};
use ratatui::Frame;
use ratatui::layout::{Position, Rect};
use ratatui::style::Style;
use ratatui::widgets::{Clear, Paragraph};

pub(super) struct PanelEditor {
    enabled: bool,
    active: bool,
    focused: usize,
    dragging: bool,
}

impl PanelEditor {
    pub(super) fn new(enabled: bool) -> Self {
        Self {
            enabled,
            active: false,
            focused: 0,
            dragging: false,
        }
    }

    pub(super) fn handle(
        &mut self,
        event: &InputEvent<crossterm::event::KeyEvent>,
        options: &mut DmenuOptions,
        area: Rect,
    ) -> bool {
        if !self.enabled {
            return false;
        }
        if let InputEvent::Input(key) = event
            && key.code == KeyCode::Char('p')
            && key.modifiers == KeyModifiers::ALT
        {
            self.active = !self.active;
            self.dragging = false;
            return true;
        }
        if !self.active {
            return false;
        }
        match event {
            InputEvent::Input(key) => {
                match key.code {
                    KeyCode::Esc | KeyCode::Enter => {
                        self.active = false;
                        self.dragging = false;
                    }
                    KeyCode::Tab => {
                        self.focused = (self.focused + 1) % (2 + options.custom_panels.len())
                    }
                    KeyCode::BackTab => {
                        self.focused = (self.focused + 1 + options.custom_panels.len())
                            % (2 + options.custom_panels.len())
                    }
                    KeyCode::Left => self.dock(options, PanelSide::Left),
                    KeyCode::Right => self.dock(options, PanelSide::Right),
                    KeyCode::Up => self.dock(options, PanelSide::Top),
                    KeyCode::Down => self.dock(options, PanelSide::Bottom),
                    KeyCode::Char('+') | KeyCode::Char('=') => self.resize(options, true),
                    KeyCode::Char('-') => self.resize(options, false),
                    _ => {}
                }
                true
            }
            InputEvent::Mouse(mouse) => {
                match mouse.kind {
                    MouseEventKind::Down(MouseButton::Left) => {
                        let (layout, custom) = options.split_all(area);
                        let rectangles = std::iter::once(layout.chunks[layout.content_panel_index])
                            .chain(std::iter::once(layout.chunks[layout.input_panel_index]))
                            .chain(custom);
                        if let Some(index) = rectangles
                            .into_iter()
                            .position(|r| r.contains(Position::new(mouse.column, mouse.row)))
                        {
                            self.focused = index;
                            self.dragging = true;
                        }
                    }
                    MouseEventKind::Up(MouseButton::Left) => self.dragging = false,
                    MouseEventKind::Drag(MouseButton::Left) if self.dragging => {
                        let layout = options.split_layout(area);
                        self.dock(
                            options,
                            nearest_edge(
                                layout.chunks[layout.items_panel_index],
                                mouse.column,
                                mouse.row,
                            ),
                        );
                    }
                    MouseEventKind::ScrollUp => self.resize(options, true),
                    MouseEventKind::ScrollDown => self.resize(options, false),
                    _ => {}
                }
                true
            }
            _ => false,
        }
    }

    fn dock(&self, options: &mut DmenuOptions, physical_side: PanelSide) {
        let side = physical_side.rotated((360 - options.panels.rotation) % 360);
        match self.focused {
            0 => options.panels.info_position = Some(side),
            1 => options.panels.input_position = Some(side),
            index => {
                if let Some(panel) = options.custom_panels.get_mut(index - 2) {
                    panel.position = side;
                }
            }
        }
    }

    fn resize(&self, options: &mut DmenuOptions, grow: bool) {
        let (value, step, maximum) = match self.focused {
            0 => (
                options
                    .panels
                    .info_size
                    .get_or_insert(options.content_panel_height_percent),
                5,
                90,
            ),
            1 => (
                options
                    .panels
                    .input_size
                    .get_or_insert(options.input_panel_height),
                1,
                u16::MAX,
            ),
            index => {
                let Some(panel) = options.custom_panels.get_mut(index - 2) else {
                    return;
                };
                (&mut panel.size, 5, 90)
            }
        };
        *value = if grow {
            value.saturating_add(step).min(maximum)
        } else {
            value.saturating_sub(step)
        };
    }

    pub(super) fn render(&self, frame: &mut Frame, options: &DmenuOptions) {
        if !self.active || frame.area().is_empty() {
            return;
        }
        let name = match self.focused {
            0 => "preview",
            1 => "input",
            index => options
                .custom_panels
                .get(index - 2)
                .map_or("panel", |p| p.name.as_str()),
        };
        let area = Rect::new(frame.area().x, frame.area().y, frame.area().width, 1);
        frame.render_widget(Clear, area);
        frame.render_widget(
            Paragraph::new(format!(
                "Move {name}: Tab focus | arrows/drag dock | +/-/wheel size | Esc done"
            ))
            .style(
                Style::default()
                    .fg(options.highlight_color)
                    .bg(options.input_background_color),
            ),
            area,
        );
    }
}

fn nearest_edge(area: Rect, x: u16, y: u16) -> PanelSide {
    [
        (x.abs_diff(area.left()), PanelSide::Left),
        (x.abs_diff(area.right().saturating_sub(1)), PanelSide::Right),
        (y.abs_diff(area.top()), PanelSide::Top),
        (
            y.abs_diff(area.bottom().saturating_sub(1)),
            PanelSide::Bottom,
        ),
    ]
    .into_iter()
    .min_by_key(|(distance, _)| *distance)
    .map_or(PanelSide::Top, |(_, side)| side)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::KeyEvent;

    #[test]
    fn editing_is_opt_in_and_movement_respects_rotation_and_bounds() {
        let area = Rect::new(0, 0, 80, 40);
        let toggle = InputEvent::Input(KeyEvent::new(KeyCode::Char('p'), KeyModifiers::ALT));
        let mut options = DmenuOptions::from_cli(&crate::cli::Opts::default());
        assert!(!PanelEditor::new(false).handle(&toggle, &mut options, area));
        let mut editor = PanelEditor::new(true);
        assert!(editor.handle(&toggle, &mut options, area));
        options.panels.rotation = 90;
        editor.dock(&mut options, PanelSide::Left);
        assert_eq!(
            options.panels.info_position.unwrap().rotated(90),
            PanelSide::Left
        );
        for _ in 0..30 {
            editor.resize(&mut options, true);
        }
        assert_eq!(options.panels.info_size, Some(90));
        for _ in 0..30 {
            editor.resize(&mut options, false);
        }
        assert_eq!(options.panels.info_size, Some(0));
        assert!(editor.handle(
            &InputEvent::Input(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE)),
            &mut options,
            area
        ));
        assert!(!editor.active);
    }
}
