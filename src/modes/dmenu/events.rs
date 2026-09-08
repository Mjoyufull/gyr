//! Dmenu keyboard and mouse behavior using rendered list geometry.

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEvent, MouseEventKind};

use crate::ui::{DmenuUI, Keybinds};

use super::options::DmenuOptions;

pub(super) enum LoopOutcome {
    Continue,
    Exit,
    Print(String),
}

pub(super) fn handle_key_event(
    ui: &mut DmenuUI,
    key: KeyEvent,
    options: &DmenuOptions,
    terminal_area: ratatui::layout::Rect,
) -> LoopOutcome {
    match (key.code, key.modifiers) {
        (code, modifiers)
            if matches_dmenu_binding(
                &options.keybinds,
                code,
                modifiers,
                Keybinds::matches_exit,
            ) =>
        {
            return LoopOutcome::Exit;
        }
        (code, modifiers)
            if matches_dmenu_binding(
                &options.keybinds,
                code,
                modifiers,
                Keybinds::matches_select,
            ) =>
        {
            return handle_submit(ui, options);
        }
        (code, modifiers)
            if matches_dmenu_binding(
                &options.keybinds,
                code,
                modifiers,
                Keybinds::matches_backspace,
            ) =>
        {
            ui.query.pop();
            ui.filter();
            auto_select_if_single_match(ui, options);
        }
        (code, modifiers)
            if matches_dmenu_binding(
                &options.keybinds,
                code,
                modifiers,
                Keybinds::matches_left,
            ) =>
        {
            if options.panels.horizontal() {
                move_selection(ui, options, terminal_area, -1);
            } else {
                move_to_first(ui);
            }
        }
        (code, modifiers)
            if matches_dmenu_binding(
                &options.keybinds,
                code,
                modifiers,
                Keybinds::matches_right,
            ) =>
        {
            if options.panels.horizontal() {
                move_selection(ui, options, terminal_area, 1);
            } else {
                move_to_last(ui, options, terminal_area);
            }
        }
        (code, modifiers)
            if matches_dmenu_binding(
                &options.keybinds,
                code,
                modifiers,
                Keybinds::matches_down,
            ) =>
        {
            move_selection(ui, options, terminal_area, 1);
        }
        (code, modifiers)
            if matches_dmenu_binding(&options.keybinds, code, modifiers, Keybinds::matches_up) =>
        {
            move_selection(ui, options, terminal_area, -1);
        }
        (KeyCode::Char(ch), KeyModifiers::NONE) | (KeyCode::Char(ch), KeyModifiers::SHIFT) => {
            ui.query.push(ch);
            ui.filter();
            auto_select_if_single_match(ui, options);
        }
        _ => {}
    }

    ui.info(options.highlight_color);
    LoopOutcome::Continue
}

fn matches_dmenu_binding(
    keybinds: &Keybinds,
    code: KeyCode,
    modifiers: KeyModifiers,
    matches_configured: fn(&Keybinds, KeyCode, KeyModifiers) -> bool,
) -> bool {
    matches_configured(keybinds, code, modifiers)
        || (matches!(
            code,
            KeyCode::Esc
                | KeyCode::Enter
                | KeyCode::Backspace
                | KeyCode::Left
                | KeyCode::Right
                | KeyCode::Down
                | KeyCode::Up
        ) && matches_configured(keybinds, code, KeyModifiers::NONE))
}

fn move_to_first(ui: &mut DmenuUI<'_>) {
    if !ui.shown.is_empty() {
        ui.selected = Some(0);
        ui.scroll_offset = 0;
    }
}

fn move_to_last(
    ui: &mut DmenuUI<'_>,
    options: &DmenuOptions,
    terminal_area: ratatui::layout::Rect,
) {
    let Some(last_index) = ui.shown.len().checked_sub(1) else {
        return;
    };

    ui.selected = Some(last_index);
    let max_visible = options.max_visible_items(terminal_area);
    if max_visible > 0 && ui.shown.len() > max_visible {
        ui.scroll_offset = ui.shown.len().saturating_sub(max_visible);
    } else {
        ui.scroll_offset = 0;
    }
}

pub(super) fn handle_mouse_event(
    ui: &mut DmenuUI,
    mouse_event: MouseEvent,
    options: &DmenuOptions,
    terminal_area: ratatui::layout::Rect,
) -> LoopOutcome {
    let geometry = options.result_layout(terminal_area);
    let hit = geometry.hit(mouse_event.column, mouse_event.row);
    let Some(relative) = hit else {
        return LoopOutcome::Continue;
    };
    let index = ui.scroll_offset + relative;
    match mouse_event.kind {
        MouseEventKind::Moved if index < ui.shown.len() => {
            ui.selected = Some(index);
            ui.info(options.highlight_color);
        }
        MouseEventKind::Down(MouseButton::Left) if index < ui.shown.len() => {
            return LoopOutcome::Print(selected_output(ui, options, index));
        }
        MouseEventKind::ScrollUp if ui.scroll_offset > 0 => {
            ui.scroll_offset -= 1;
        }
        MouseEventKind::ScrollDown if ui.scroll_offset + geometry.capacity() < ui.shown.len() => {
            ui.scroll_offset += 1;
        }
        _ => {}
    }
    if !ui.shown.is_empty() {
        ui.selected = Some((ui.scroll_offset + relative).min(ui.shown.len() - 1));
    }
    LoopOutcome::Continue
}

fn handle_submit(ui: &mut DmenuUI, options: &DmenuOptions) -> LoopOutcome {
    auto_select_if_single_match(ui, options);

    if let Some(selected) = ui.selected
        && selected < ui.shown.len()
    {
        return LoopOutcome::Print(selected_output(ui, options, selected));
    }

    if !options.only_match && !ui.query.is_empty() {
        return LoopOutcome::Print(ui.query.clone());
    }

    if options.only_match {
        LoopOutcome::Continue
    } else {
        LoopOutcome::Exit
    }
}

fn selected_output(ui: &DmenuUI, options: &DmenuOptions, selected: usize) -> String {
    if options.index_original_mode {
        ui.shown[selected].line_number.to_string()
    } else if options.index_mode {
        selected.to_string()
    } else if let Some(ref accept_cols) = options.accept_nth {
        ui.shown[selected].get_accept_nth_output(accept_cols)
    } else {
        ui.shown[selected].original_line.clone()
    }
}

fn auto_select_if_single_match(ui: &mut DmenuUI, options: &DmenuOptions) {
    if options.auto_select && ui.shown.len() == 1 {
        ui.selected = Some(0);
    }
}

fn move_selection(
    ui: &mut DmenuUI,
    options: &DmenuOptions,
    terminal_area: ratatui::layout::Rect,
    direction: i32,
) {
    let Some(selected) = ui.selected else {
        return;
    };

    let Some(last_index) = ui.shown.len().checked_sub(1) else {
        return;
    };

    let forward = (direction > 0) != (options.panels.rotation >= 180);
    ui.selected = if forward {
        if selected < last_index {
            Some(selected + 1)
        } else if !options.hard_stop {
            Some(0)
        } else {
            Some(selected)
        }
    } else if selected > 0 {
        Some(selected - 1)
    } else if !options.hard_stop {
        Some(last_index)
    } else {
        Some(selected)
    };

    let Some(new_selected) = ui.selected else {
        return;
    };

    let max_visible = options.max_visible_items(terminal_area);
    if max_visible == 0 {
        ui.scroll_offset = 0;
    } else if new_selected < ui.scroll_offset {
        ui.scroll_offset = new_selected;
    } else if new_selected >= ui.scroll_offset + max_visible {
        ui.scroll_offset = new_selected.saturating_sub(max_visible - 1);
    }
}

#[cfg(test)]
mod tests {
    use crate::cli::Opts;
    use crate::common::Item;
    use crate::ui::{DmenuUI, Keybinds};
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

    use super::{DmenuOptions, LoopOutcome, handle_key_event};

    #[test]
    fn submit_returns_query_when_no_selection_and_only_match_is_disabled() {
        let mut ui = DmenuUI::new(
            vec![Item::new_simple("a".into(), "a".into(), 1)],
            false,
            false,
        );
        ui.query = "typed".to_string();
        ui.filter();
        ui.selected = None;

        let outcome = handle_key_event(
            &mut ui,
            crossterm::event::KeyEvent::new(
                crossterm::event::KeyCode::Enter,
                crossterm::event::KeyModifiers::NONE,
            ),
            &DmenuOptions::from_cli(&Opts::default()),
            ratatui::layout::Rect::new(0, 0, 80, 20),
        );

        assert!(matches!(outcome, LoopOutcome::Print(output) if output == "typed"));
    }

    #[test]
    fn submit_uses_accept_nth_output_when_requested() {
        let mut ui = DmenuUI::new(
            vec![Item::new_simple(
                "left:right".into(),
                "left:right".into(),
                1,
            )],
            false,
            false,
        );
        ui.filter();

        let cli = Opts {
            dmenu_accept_nth: Some(vec![1]),
            ..Opts::default()
        };

        let outcome = handle_key_event(
            &mut ui,
            crossterm::event::KeyEvent::new(
                crossterm::event::KeyCode::Enter,
                crossterm::event::KeyModifiers::NONE,
            ),
            &DmenuOptions::from_cli(&cli),
            ratatui::layout::Rect::new(0, 0, 80, 20),
        );

        assert!(matches!(outcome, LoopOutcome::Print(output) if output == "left:right"));
    }

    #[test]
    fn configured_navigation_moves_selection_without_typing() {
        let keybinds: Keybinds = toml::from_str(
            r#"
down = [{ key = "j", modifiers = "alt" }]
up = [{ key = "k", modifiers = "alt" }]
"#,
        )
        .expect("valid keybind config");
        let cli = Opts {
            keybinds,
            ..Opts::default()
        };
        let options = DmenuOptions::from_cli(&cli);
        let mut ui = DmenuUI::new(
            vec![
                Item::new_simple("one".into(), "one".into(), 1),
                Item::new_simple("two".into(), "two".into(), 2),
            ],
            false,
            false,
        );
        ui.selected = Some(0);

        let outcome = handle_key_event(
            &mut ui,
            KeyEvent::new(KeyCode::Char('j'), KeyModifiers::ALT),
            &options,
            ratatui::layout::Rect::new(0, 0, 80, 20),
        );

        assert!(matches!(outcome, LoopOutcome::Continue));
        assert_eq!(ui.selected, Some(1));
        assert!(ui.query.is_empty());
    }

    #[test]
    fn default_special_keys_preserve_legacy_modifier_behavior() {
        let options = DmenuOptions::from_cli(&Opts::default());
        let mut submit_ui = DmenuUI::new(
            vec![Item::new_simple("one".into(), "one".into(), 1)],
            false,
            false,
        );
        submit_ui.filter();

        let submit = handle_key_event(
            &mut submit_ui,
            KeyEvent::new(KeyCode::Enter, KeyModifiers::SHIFT),
            &options,
            ratatui::layout::Rect::new(0, 0, 80, 20),
        );

        assert!(matches!(submit, LoopOutcome::Print(output) if output == "one"));

        let mut backspace_ui = DmenuUI::new(Vec::new(), false, false);
        backspace_ui.query = "ab".to_string();
        handle_key_event(
            &mut backspace_ui,
            KeyEvent::new(KeyCode::Backspace, KeyModifiers::ALT),
            &options,
            ratatui::layout::Rect::new(0, 0, 80, 20),
        );
        assert_eq!(backspace_ui.query, "a");
    }

    #[test]
    fn selected_output_returns_original_line_number() {
        let cli = Opts {
            dmenu_index_original_mode: true,
            ..Opts::default()
        };

        let options = super::DmenuOptions::from_cli(&cli);

        let mut ui = DmenuUI::new(
            vec![
                Item::new_simple("2 Suspend".into(), "2 Suspend".into(), 2),
                Item::new_simple("4 Shutdown".into(), "4 Shutdown".into(), 4),
            ],
            false,
            false,
        );

        ui.filter();

        let output = super::selected_output(&ui, &options, 1);

        assert_eq!(output, "4");
    }
}
