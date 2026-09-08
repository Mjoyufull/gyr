//! Fullscreen terminal lifecycle without competing cursor-response readers.

use crossterm::{
    ExecutableCommand,
    event::{DisableMouseCapture, EnableMouseCapture},
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use eyre::{Result, WrapErr, eyre};
use std::io;

/// Invalidate the fullscreen buffers without querying the cursor on the input stream.
/// Ratatui's `clear` preserves the cursor by reading a reply; active event readers
/// can consume that reply. Fullscreen selectors redraw their cursor explicitly.
pub(crate) fn clear_fullscreen<B: ratatui::backend::Backend>(
    terminal: &mut ratatui::Terminal<B>,
) -> Result<(), B::Error> {
    let area = terminal.size()?.into();
    terminal.resize(area)
}

pub(crate) fn setup_terminal(disable_mouse: bool) -> Result<()> {
    enable_raw_mode().wrap_err("Failed to enable raw mode")?;

    if let Err(error) = io::stderr().execute(EnterAlternateScreen) {
        let _ = disable_raw_mode();
        return Err(error).wrap_err("Failed to enter alternate screen");
    }

    if !disable_mouse && let Err(error) = io::stderr().execute(EnableMouseCapture) {
        let _ = io::stderr().execute(LeaveAlternateScreen);
        let _ = disable_raw_mode();
        return Err(error).wrap_err("Failed to enable mouse capture");
    }

    Ok(())
}

pub(crate) fn shutdown_terminal(disable_mouse: bool) -> Result<()> {
    let mut first_error = None;

    if !disable_mouse {
        record_terminal_error(
            &mut first_error,
            io::stderr()
                .execute(DisableMouseCapture)
                .map(|_| ())
                .wrap_err("Failed to disable mouse capture"),
        );
    }

    record_terminal_error(
        &mut first_error,
        io::stderr()
            .execute(LeaveAlternateScreen)
            .map(|_| ())
            .wrap_err("Failed to leave alternate screen"),
    );
    record_terminal_error(
        &mut first_error,
        disable_raw_mode().wrap_err("Failed to disable raw mode"),
    );

    if let Some(error) = first_error {
        Err(error)
    } else {
        Ok(())
    }
}

fn record_terminal_error(first_error: &mut Option<eyre::Report>, result: Result<()>) {
    if let Err(error) = result
        && first_error.is_none()
    {
        *first_error = Some(eyre!(error));
    }
}

#[cfg(test)]
mod tests;
