//! Ensure fullscreen invalidation never reads a terminal cursor response.

use ratatui::Terminal;
use ratatui::backend::{Backend, ClearType, TestBackend, WindowSize};
use ratatui::buffer::Cell;
use ratatui::layout::{Position, Size};
use ratatui::widgets::Paragraph;
use std::convert::Infallible;

struct NoCursorQuery(TestBackend);

impl Backend for NoCursorQuery {
    type Error = Infallible;

    fn draw<'a, I>(&mut self, content: I) -> Result<(), Self::Error>
    where
        I: Iterator<Item = (u16, u16, &'a Cell)>,
    {
        self.0.draw(content)
    }

    fn hide_cursor(&mut self) -> Result<(), Self::Error> {
        self.0.hide_cursor()
    }
    fn show_cursor(&mut self) -> Result<(), Self::Error> {
        self.0.show_cursor()
    }
    fn get_cursor_position(&mut self) -> Result<Position, Self::Error> {
        panic!("fullscreen redraw must not compete with the input reader")
    }
    fn set_cursor_position<P: Into<Position>>(&mut self, position: P) -> Result<(), Self::Error> {
        self.0.set_cursor_position(position)
    }
    fn clear(&mut self) -> Result<(), Self::Error> {
        self.0.clear()
    }
    fn clear_region(&mut self, kind: ClearType) -> Result<(), Self::Error> {
        self.0.clear_region(kind)
    }
    fn size(&self) -> Result<Size, Self::Error> {
        self.0.size()
    }
    fn window_size(&mut self) -> Result<WindowSize, Self::Error> {
        self.0.window_size()
    }
    fn flush(&mut self) -> Result<(), Self::Error> {
        self.0.flush()
    }
}

#[test]
fn fullscreen_clear_redraws_unchanged_text_without_cursor_query() {
    let mut terminal = Terminal::new(NoCursorQuery(TestBackend::new(20, 4)))
        .expect("test terminal should initialize");
    for _ in 0..2 {
        super::clear_fullscreen(&mut terminal).expect("fullscreen invalidation should succeed");
        terminal
            .draw(|frame| frame.render_widget(Paragraph::new("unchanged"), frame.area()))
            .expect("text should redraw after clearing");
        assert_eq!(terminal.backend().0.buffer()[(0, 0)].symbol(), "u");
    }
}
