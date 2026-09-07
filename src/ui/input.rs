//! Async input handling using crossterm's EventStream
//!
//! This module provides async event handling for the TUI, using tokio and crossterm's EventStream.
//! AsyncInput is infrastructure for future async migration.

#![allow(dead_code)]

use crossterm::event::{Event as CrosstermEvent, EventStream, KeyCode, KeyEvent, MouseEvent};
use futures::StreamExt;
use std::time::Duration;
use tokio::sync::mpsc;
use tokio::time::interval;

/// Builder for `AsyncInput`
#[non_exhaustive]
#[derive(Debug, Clone, Copy)]
pub struct Config {
    pub exit_key: KeyCode,
    pub tick_rate: Duration,
    pub render_rate: Option<Duration>,
    pub disable_mouse: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            exit_key: KeyCode::Esc,
            tick_rate: Duration::from_millis(250),
            render_rate: Some(Duration::from_millis(16)),
            disable_mouse: false,
        }
    }
}

impl Config {
    /// Creates a new sync `Input` with the configuration in `Self`
    /// blocking input handler for simple modes
    pub fn init(self) -> Input {
        Input::with_config(self)
    }

    /// Creates a new async `AsyncInput` with the configuration in `Self`
    /// Used by async modes (app_launcher when migrated)
    pub fn init_async(self) -> AsyncInput {
        AsyncInput::with_config(self)
    }
}

#[derive(Debug)]
pub enum Event<I> {
    Input(I),
    Mouse(MouseEvent),
    Tick,
    Render,
}

/// Async input handler using crossterm's EventStream
pub struct AsyncInput {
    rx: mpsc::UnboundedReceiver<Event<KeyEvent>>,
    _task: tokio::task::JoinHandle<()>,
}

impl AsyncInput {
    fn handle_terminal_event(
        event: CrosstermEvent,
        tx: &mpsc::UnboundedSender<Event<KeyEvent>>,
        config: Config,
    ) -> bool {
        match event {
            CrosstermEvent::Key(key)
                if matches!(
                    key.kind,
                    crossterm::event::KeyEventKind::Press | crossterm::event::KeyEventKind::Repeat
                ) =>
            {
                if tx.send(Event::Input(key)).is_err() {
                    return true;
                }
                if key.code == config.exit_key {
                    return true;
                }
            }
            CrosstermEvent::Mouse(mouse)
                if !config.disable_mouse && tx.send(Event::Mouse(mouse)).is_err() =>
            {
                return true;
            }
            CrosstermEvent::Resize(_, _) if tx.send(Event::Render).is_err() => {
                return true;
            }
            _ => {}
        }

        false
    }

    pub fn with_config(config: Config) -> Self {
        let (tx, rx) = mpsc::unbounded_channel();

        let _task = tokio::spawn(async move {
            let mut reader = EventStream::new();
            let mut tick_interval = interval(config.tick_rate);
            let mut render_interval = config.render_rate.map(interval);

            loop {
                tokio::select! {
                    // Handle terminal events
                    maybe_event = reader.next() => {
                        match maybe_event {
                            Some(Ok(event)) => {
                                if Self::handle_terminal_event(event, &tx, config) {
                                    return;
                                }
                            }
                            Some(Err(_)) => {
                                // Event read error, exit
                                return;
                            }
                            None => {
                                // Stream ended
                                return;
                            }
                        }
                    }
                    // Tick events for periodic updates
                    _ = tick_interval.tick() => {
                        if tx.send(Event::Tick).is_err() {
                            return;
                        }
                    }
                    // Render events for frame rate control (optional)
                    _ = async {
                        match render_interval.as_mut() {
                            Some(interval) => {
                                interval.tick().await;
                            }
                            None => std::future::pending::<()>().await,
                        }
                    } => {
                        if tx.send(Event::Render).is_err() {
                            return;
                        }
                    }
                }
            }
        });

        Self { rx, _task }
    }

    /// Next event (async)
    pub async fn next(&mut self) -> Option<Event<KeyEvent>> {
        self.rx.recv().await
    }

    /// Stop the background event reader before code temporarily reads terminal stdio directly.
    pub async fn shutdown(mut self) {
        self._task.abort();
        let _ = (&mut self._task).await;
    }
}

impl Drop for AsyncInput {
    fn drop(&mut self) {
        self._task.abort();
    }
}

// =============================================================================
// LEGACY SYNC INPUT (kept for backwards compatibility with dmenu/cclip modes)
// =============================================================================

use std::sync::mpsc as std_mpsc;
use std::thread;

/// Legacy sync input handler (for modes not yet migrated to async)
pub struct Input {
    rx: std_mpsc::Receiver<Event<KeyEvent>>,
    _input_handle: thread::JoinHandle<()>,
    _tick_handle: thread::JoinHandle<()>,
}

impl Input {
    pub fn with_config(config: Config) -> Self {
        let (tx, rx) = std_mpsc::channel();

        let _input_handle = {
            let tx = tx.clone();

            thread::spawn(move || {
                loop {
                    if let Ok(true) = crossterm::event::poll(Duration::from_millis(100))
                        && let Ok(event) = crossterm::event::read()
                    {
                        match event {
                            CrosstermEvent::Key(key) => {
                                if tx.send(Event::Input(key)).is_err() {
                                    return;
                                }
                                if key.code == config.exit_key {
                                    return;
                                }
                            }
                            CrosstermEvent::Mouse(mouse)
                                if !config.disable_mouse
                                    && tx.send(Event::Mouse(mouse)).is_err() =>
                            {
                                return;
                            }
                            _ => {}
                        }
                    }
                }
            })
        };

        let _tick_handle = {
            thread::spawn(move || {
                loop {
                    if tx.send(Event::Tick).is_err() {
                        break;
                    }
                    thread::sleep(config.tick_rate);
                }
            })
        };

        Self {
            rx,
            _input_handle,
            _tick_handle,
        }
    }

    /// Next key pressed by user.
    pub fn next(&self) -> Result<Event<KeyEvent>, std_mpsc::RecvError> {
        self.rx.recv()
    }

    /// Next key pressed by user with timeout.
    #[allow(dead_code)]
    pub fn next_timeout(
        &self,
        timeout: Duration,
    ) -> Result<Event<KeyEvent>, std_mpsc::RecvTimeoutError> {
        self.rx.recv_timeout(timeout)
    }
}

#[cfg(test)]
mod tests {
    use super::{AsyncInput, Config, Event};
    use crossterm::event::{
        Event as CrosstermEvent, KeyCode, KeyEvent, KeyEventKind, KeyModifiers,
    };
    use tokio::sync::mpsc;

    #[test]
    fn async_input_forwards_repeated_keys() {
        let (tx, mut rx) = mpsc::unbounded_channel();
        let key = KeyEvent::new_with_kind(KeyCode::Down, KeyModifiers::NONE, KeyEventKind::Repeat);

        assert!(!AsyncInput::handle_terminal_event(
            CrosstermEvent::Key(key),
            &tx,
            Config::default(),
        ));
        assert!(matches!(rx.try_recv(), Ok(Event::Input(received)) if received == key));
    }

    #[test]
    fn async_input_ignores_key_release_events() {
        let (tx, mut rx) = mpsc::unbounded_channel();
        let key = KeyEvent::new_with_kind(KeyCode::Down, KeyModifiers::NONE, KeyEventKind::Release);

        assert!(!AsyncInput::handle_terminal_event(
            CrosstermEvent::Key(key),
            &tx,
            Config::default(),
        ));
        assert!(rx.try_recv().is_err());
    }
}
