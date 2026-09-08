//! Launcher keyboard and mouse event handling against rendered panel geometry.

use crate::cli::Opts;
use crate::core::hidden_entries::{HiddenEntryStore, NewHiddenEntry};
use crate::core::ranking::current_unix_seconds;
use crate::core::state::{Message, State};
use crate::ui::InputEvent as Event;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEvent, MouseEventKind};

pub(crate) fn handle_event(
    state: &mut State,
    event: Event<crossterm::event::KeyEvent>,
    cli: &Opts,
    db: &std::sync::Arc<redb::Database>,
    hidden_store: &HiddenEntryStore,
    terminal_area: ratatui::layout::Rect,
) {
    match event {
        Event::Input(key) => handle_key_event(state, key, cli, db, hidden_store, terminal_area),
        Event::Mouse(mouse_event) => handle_mouse_event(state, mouse_event, cli, terminal_area),
        Event::Tick | Event::Render => {}
    }
}

fn handle_key_event(
    state: &mut State,
    key: KeyEvent,
    cli: &Opts,
    db: &std::sync::Arc<redb::Database>,
    hidden_store: &HiddenEntryStore,
    terminal_area: ratatui::layout::Rect,
) {
    let max_visible = crate::ui::launcher_visible_rows(terminal_area, cli);
    state.clear_status_message();

    let msg = if cli.keybinds.matches_exit(key.code, key.modifiers) {
        Message::Exit
    } else if cli.keybinds.matches_select(key.code, key.modifiers) {
        Message::Select
    } else if cli.keybinds.matches_up(key.code, key.modifiers) {
        Message::MoveUp
    } else if cli.keybinds.matches_down(key.code, key.modifiers) {
        Message::MoveDown
    } else if cli.keybinds.matches_left(key.code, key.modifiers) {
        Message::MoveUp
    } else if cli.keybinds.matches_right(key.code, key.modifiers) {
        Message::MoveDown
    } else if cli.keybinds.matches_backspace(key.code, key.modifiers) {
        Message::Backspace
    } else if cli.keybinds.matches_pin(key.code, key.modifiers) {
        toggle_selected_pin(state, db);
        refresh_info(state, cli);
        return;
    } else if cli.keybinds.matches_hide(key.code, key.modifiers) {
        match hide_selected_entry(state, hidden_store) {
            Ok(Some(name)) => {
                state.set_status_message(format!("Hidden {name}; use the unhide binding to undo"));
            }
            Ok(None) => {}
            Err(error) => {
                state.set_status_message(format!("Could not hide entry: {error}"));
            }
        }
        refresh_info(state, cli);
        return;
    } else if cli.keybinds.matches_unhide_last(key.code, key.modifiers) {
        match unhide_last_entry(state, hidden_store) {
            Ok(Some((name, true))) => state.set_status_message(format!("Restored {name}")),
            Ok(Some((name, false))) => {
                state.set_status_message(format!("Cleared hidden record for unavailable {name}"));
            }
            Ok(None) => state.set_status_message("No manual hides to restore"),
            Err(error) => {
                state.set_status_message(format!("Could not restore entry: {error}"));
            }
        }
        refresh_info(state, cli);
        return;
    } else {
        match key.code {
            KeyCode::Char(c)
                if !key.modifiers.contains(KeyModifiers::CONTROL)
                    && !key.modifiers.contains(KeyModifiers::ALT) =>
            {
                Message::CharInput(c)
            }
            KeyCode::Home => Message::MoveFirst,
            KeyCode::End => Message::MoveLast,
            KeyCode::Tab => Message::MoveDown,
            KeyCode::BackTab => Message::MoveUp,
            _ => Message::Tick,
        }
    };

    if key.modifiers.contains(KeyModifiers::CONTROL)
        && key.code == KeyCode::Char('c')
        && !cli.keybinds.matches_exit(key.code, key.modifiers)
    {
        state.should_exit = true;
    }

    let oriented = if cli.panels.rotation >= 180 {
        match msg {
            Message::MoveUp => Message::MoveDown,
            Message::MoveDown => Message::MoveUp,
            other => other,
        }
    } else {
        msg
    };
    crate::core::state::update(state, oriented, cli.hard_stop, max_visible);
    refresh_info(state, cli);
}

fn handle_mouse_event(
    state: &mut State,
    mouse_event: MouseEvent,
    cli: &Opts,
    terminal_area: ratatui::layout::Rect,
) {
    let metrics = list_metrics(terminal_area, cli);

    let msg = match mouse_event.kind {
        MouseEventKind::Down(MouseButton::Left) => {
            if let Some(index) = metrics
                .hit(mouse_event.column, mouse_event.row)
                .map(|index| state.scroll_offset + index)
                .filter(|index| *index < state.shown.len())
            {
                crate::core::state::update(
                    state,
                    Message::SelectIndex(index),
                    cli.hard_stop,
                    metrics.capacity(),
                );
                Message::Select
            } else {
                Message::Tick
            }
        }
        MouseEventKind::Moved => metrics
            .hit(mouse_event.column, mouse_event.row)
            .map(|index| state.scroll_offset + index)
            .filter(|index| *index < state.shown.len())
            .map(Message::SelectIndex)
            .unwrap_or(Message::Tick),
        MouseEventKind::ScrollDown => {
            if metrics.hit(mouse_event.column, mouse_event.row).is_some()
                && !state.shown.is_empty()
                && state.scroll_offset + metrics.capacity() < state.shown.len()
            {
                state.scroll_offset += 1;
                if let Some(index) = metrics.hit(mouse_event.column, mouse_event.row) {
                    state.selected = Some((state.scroll_offset + index).min(state.shown.len() - 1));
                }
                refresh_info(state, cli);
            }
            Message::Tick
        }
        MouseEventKind::ScrollUp => {
            if metrics.hit(mouse_event.column, mouse_event.row).is_some()
                && !state.shown.is_empty()
                && state.scroll_offset > 0
            {
                state.scroll_offset -= 1;
                if let Some(index) = metrics.hit(mouse_event.column, mouse_event.row) {
                    state.selected = Some((state.scroll_offset + index).min(state.shown.len() - 1));
                }
                refresh_info(state, cli);
            }
            Message::Tick
        }
        _ => Message::Tick,
    };

    if !matches!(msg, Message::Tick) {
        if crate::cli::DEBUG_ENABLED.load(std::sync::atomic::Ordering::Relaxed) {
            crate::core::debug_logger::log_event(&format!("State update via Mouse: {:?}", msg));
        }

        crate::core::state::update(state, msg, cli.hard_stop, metrics.capacity());
        refresh_info(state, cli);
    }
}

fn toggle_selected_pin(state: &mut State, db: &std::sync::Arc<redb::Database>) {
    let Some(index) = state.selected else {
        return;
    };
    let Some(app) = state.shown.get(index).cloned() else {
        return;
    };

    let Ok(is_pinned) = crate::core::database::toggle_pin(db, &app.name) else {
        return;
    };

    for entry in &mut state.apps {
        if entry.name == app.name {
            entry.pinned = is_pinned;
        }
    }

    let frecency_data = crate::core::database::load_frecency(db);
    state.pin_timestamps = crate::core::database::load_pin_timestamps(db);
    crate::core::ranking::sort_by_ranking(
        &mut state.apps,
        &frecency_data,
        state.ranking_mode,
        state.pinned_order_mode,
        &state.pin_timestamps,
        current_unix_seconds(),
    );
    state.refresh_visibility();
}

fn hide_selected_entry(
    state: &mut State,
    hidden_store: &HiddenEntryStore,
) -> eyre::Result<Option<String>> {
    let Some(index) = state.selected else {
        return Ok(None);
    };
    let Some(app) = state.shown.get(index).cloned() else {
        return Ok(None);
    };
    let Some(entry_key) = app.entry_key() else {
        return Ok(None);
    };
    let Some(source_display) = app.source_display() else {
        return Ok(None);
    };
    let hidden_at_unix_ms = std::time::SystemTime::now()
        .duration_since(std::time::SystemTime::UNIX_EPOCH)
        .map(|duration| u64::try_from(duration.as_millis()).unwrap_or(u64::MAX))
        .unwrap_or(0);
    let new_entry = NewHiddenEntry::new(
        entry_key,
        app.name.as_str(),
        source_display,
        hidden_at_unix_ms,
    );
    let hidden_entry = hidden_store.insert(new_entry)?;
    state.hide_entry(hidden_entry.entry_key().clone());
    Ok(Some(app.name))
}

fn unhide_last_entry(
    state: &mut State,
    hidden_store: &HiddenEntryStore,
) -> eyre::Result<Option<(String, bool)>> {
    if let Some(hidden_entry) = hidden_store.remove_last()? {
        let is_available = state
            .apps
            .iter()
            .any(|app| app.entry_key().as_ref() == Some(hidden_entry.entry_key()));
        state.unhide_entry(hidden_entry.entry_key());
        return Ok(Some((
            hidden_entry.display_name().to_string(),
            is_available,
        )));
    }
    Ok(None)
}

fn refresh_info(state: &mut State, cli: &Opts) {
    state.update_info(
        cli.highlight_color,
        cli.fancy_mode,
        cli.verbose.unwrap_or(0),
    );
}

fn list_metrics(
    terminal_area: ratatui::layout::Rect,
    cli: &Opts,
) -> crate::ui::result_layout::ResultLayout {
    crate::ui::launcher_result_layout(terminal_area, cli)
}
