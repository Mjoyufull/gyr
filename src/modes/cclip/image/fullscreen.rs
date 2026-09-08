use super::ImageRuntime;
use crate::ui::AsyncInput;
use eyre::{Result, WrapErr};
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;
use std::io;

impl ImageRuntime {
    pub(in crate::modes::cclip) async fn show_fullscreen_preview(
        &mut self,
        terminal: &mut Terminal<CrosstermBackend<io::Stderr>>,
        input: &mut AsyncInput,
    ) -> Result<()> {
        if !self.current_is_image {
            return Ok(());
        }

        let Some(manager) = &mut self.image_manager else {
            return Ok(());
        };

        let mut consecutive_errors: u8 = 0;
        let preview_result = async {
            loop {
                let mut render_error = Ok(());
                terminal.draw(|frame| {
                    if let Ok(mut manager_lock) = manager.try_lock()
                        && let Err(error) = manager_lock.render(frame, frame.area())
                    {
                        render_error = Err(error);
                    }
                })?;
                render_error?;

                match input.next().await {
                    Some(crate::ui::InputEvent::Input(key_event)) => {
                        consecutive_errors = 0;
                        match (key_event.code, key_event.modifiers) {
                            (crossterm::event::KeyCode::Esc, _)
                            | (crossterm::event::KeyCode::Char('q'), _)
                            | (
                                crossterm::event::KeyCode::Char('c'),
                                crossterm::event::KeyModifiers::CONTROL,
                            ) => break,
                            _ => {}
                        }
                    }
                    Some(_) => {
                        consecutive_errors = 0;
                    }
                    None => {
                        consecutive_errors += 1;
                        if consecutive_errors >= 3 {
                            break;
                        }
                    }
                }
            }
            Ok::<(), eyre::Report>(())
        }
        .await;

        crate::ui::terminal::clear_fullscreen(terminal).wrap_err("Failed to clear terminal")?;
        self.restore_display_state();
        self.request_buffer_sync();
        preview_result
    }
}
