use super::selection::{
    copy_selected_and_exit, delete_selected_item, keep_selection_visible, move_to_first,
    move_to_last,
};
use super::{EventContext, EventOutcome, LoopControl};
use crate::ui::{AsyncInput, Keybinds, TagMode};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use eyre::Result;

#[derive(Debug, PartialEq)]
enum KeyAction {
    ImagePreview,
    BeginTagCreation,
    BeginTagRemoval,
    Delete,
    Exit,
    Select,
    Input(char),
    Backspace,
    First,
    Last,
    Down,
    Up,
    Ignore,
}

fn key_action(keybinds: &Keybinds, key: KeyEvent) -> KeyAction {
    let code = key.code;
    let modifiers = key.modifiers;

    if keybinds.matches_image_preview(code, modifiers) {
        KeyAction::ImagePreview
    } else if keybinds.matches_tag(code, modifiers) {
        KeyAction::BeginTagCreation
    } else if keybinds.matches_tag_removal(code, modifiers) {
        KeyAction::BeginTagRemoval
    } else if keybinds.matches_cclip_delete(code, modifiers) {
        KeyAction::Delete
    } else if keybinds.matches_exit(code, modifiers) {
        KeyAction::Exit
    } else if keybinds.matches_select(code, modifiers) {
        KeyAction::Select
    } else if keybinds.matches_backspace(code, modifiers) {
        KeyAction::Backspace
    } else if keybinds.matches_left(code, modifiers) {
        KeyAction::First
    } else if keybinds.matches_right(code, modifiers) {
        KeyAction::Last
    } else if keybinds.matches_down(code, modifiers) {
        KeyAction::Down
    } else if keybinds.matches_up(code, modifiers) {
        KeyAction::Up
    } else {
        match (code, modifiers) {
            (KeyCode::Char(character), KeyModifiers::NONE)
            | (KeyCode::Char(character), KeyModifiers::SHIFT) => KeyAction::Input(character),
            _ => KeyAction::Ignore,
        }
    }
}

pub(super) async fn handle_key_event(
    ctx: &mut EventContext<'_, '_>,
    input: &mut AsyncInput,
    key: KeyEvent,
) -> Result<EventOutcome> {
    let needs_redraw = true;

    match key_action(&ctx.cli.keybinds, key) {
        KeyAction::ImagePreview => {
            ctx.image_runtime
                .show_fullscreen_preview(ctx.terminal, input)
                .await?;
        }
        KeyAction::BeginTagCreation => {
            super::super::tags::begin_tag_creation(ctx.ui, ctx.image_runtime, ctx.terminal)?;
        }
        KeyAction::BeginTagRemoval => {
            super::super::tags::begin_tag_removal(ctx.ui);
        }
        KeyAction::Delete => {
            if matches!(ctx.ui.tag_mode, TagMode::Normal) {
                delete_selected_item(ctx)?;
            }
        }
        KeyAction::Exit => {
            if ctx.ui.tag_mode != TagMode::Normal {
                ctx.ui.tag_mode = TagMode::Normal;
            } else {
                return Ok(EventOutcome {
                    control: LoopControl::Exit,
                    needs_redraw,
                });
            }
        }
        KeyAction::Select => {
            if matches!(ctx.ui.tag_mode, TagMode::Normal) {
                if copy_selected_and_exit(ctx)? {
                    return Ok(EventOutcome {
                        control: LoopControl::Exit,
                        needs_redraw,
                    });
                }
            } else {
                super::super::tags::submit_tag_mode(super::super::tags::TagSubmitContext {
                    ui: ctx.ui,
                    cli: ctx.cli,
                    db: ctx.db,
                    tag_metadata_map: ctx.tag_metadata_map,
                    tag_metadata_formatter: ctx.tag_metadata_formatter,
                    show_line_numbers: ctx.options.show_line_numbers,
                    show_tag_color_names: ctx.options.show_tag_color_names,
                    max_visible: ctx.max_visible,
                });
            }
        }
        KeyAction::Input(character) => {
            push_char(ctx.ui, character);
        }
        KeyAction::Backspace => {
            pop_char(ctx.ui);
        }
        KeyAction::First => {
            if matches!(ctx.ui.tag_mode, TagMode::Normal) {
                move_to_first(ctx.ui);
            }
        }
        KeyAction::Last => {
            if matches!(ctx.ui.tag_mode, TagMode::Normal) {
                move_to_last(
                    ctx.ui,
                    ctx.options.max_visible_items(ctx.terminal.size()?.into()),
                );
            }
        }
        KeyAction::Down => {
            if ctx.options.panels.rotation >= 180 {
                handle_up(ctx)?;
            } else {
                handle_down(ctx)?;
            }
        }
        KeyAction::Up => {
            if ctx.options.panels.rotation >= 180 {
                handle_down(ctx)?;
            } else {
                handle_up(ctx)?;
            }
        }
        KeyAction::Ignore => {}
    }

    Ok(EventOutcome {
        control: LoopControl::Continue,
        needs_redraw,
    })
}

fn push_char(ui: &mut crate::ui::DmenuUI<'_>, character: char) {
    match &mut ui.tag_mode {
        TagMode::PromptingTagName { input, .. }
        | TagMode::PromptingTagColor { input, .. }
        | TagMode::PromptingTagEmoji { input, .. }
        | TagMode::RemovingTag { input, .. } => input.push(character),
        TagMode::Normal => {
            ui.query.push(character);
            ui.filter();
        }
    }
}

fn pop_char(ui: &mut crate::ui::DmenuUI<'_>) {
    match &mut ui.tag_mode {
        TagMode::PromptingTagName { input, .. }
        | TagMode::PromptingTagColor { input, .. }
        | TagMode::PromptingTagEmoji { input, .. }
        | TagMode::RemovingTag { input, .. } => {
            input.pop();
        }
        TagMode::Normal => {
            ui.query.pop();
            ui.filter();
        }
    }
}

fn handle_down(ctx: &mut EventContext<'_, '_>) -> Result<()> {
    match &ctx.ui.tag_mode {
        TagMode::PromptingTagName { .. } => {
            ctx.ui.cycle_tag_creation_selection(1);
            return Ok(());
        }
        TagMode::RemovingTag { .. } => {
            ctx.ui.cycle_removal_selection(1);
            return Ok(());
        }
        TagMode::PromptingTagEmoji { .. } | TagMode::PromptingTagColor { .. } => {
            return Ok(());
        }
        TagMode::Normal => {}
    }

    if let Some(selected) = ctx.ui.selected {
        ctx.ui.selected = if ctx.ui.shown.is_empty() {
            Some(selected)
        } else if selected + 1 < ctx.ui.shown.len() {
            Some(selected + 1)
        } else if !ctx.options.hard_stop {
            Some(0)
        } else {
            Some(selected)
        };
        keep_selection_visible(
            ctx.ui,
            ctx.options.max_visible_items(ctx.terminal.size()?.into()),
        );
    }

    Ok(())
}

fn handle_up(ctx: &mut EventContext<'_, '_>) -> Result<()> {
    match &ctx.ui.tag_mode {
        TagMode::PromptingTagName { .. } => {
            ctx.ui.cycle_tag_creation_selection(-1);
            return Ok(());
        }
        TagMode::RemovingTag { .. } => {
            ctx.ui.cycle_removal_selection(-1);
            return Ok(());
        }
        TagMode::PromptingTagEmoji { .. } | TagMode::PromptingTagColor { .. } => {
            return Ok(());
        }
        TagMode::Normal => {}
    }

    if let Some(selected) = ctx.ui.selected {
        ctx.ui.selected = if selected > 0 {
            Some(selected - 1)
        } else if !ctx.options.hard_stop && !ctx.ui.shown.is_empty() {
            Some(ctx.ui.shown.len() - 1)
        } else {
            Some(selected)
        };
        keep_selection_visible(
            ctx.ui,
            ctx.options.max_visible_items(ctx.terminal.size()?.into()),
        );
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{KeyAction, key_action};
    use crate::ui::Keybinds;
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

    fn configured_keybinds() -> Keybinds {
        toml::from_str(
            r#"
down = [{ key = "j", modifiers = "alt" }]
up = [{ key = "k", modifiers = "alt" }]
"#,
        )
        .expect("valid keybind config")
    }

    #[test]
    fn configured_navigation_is_classified_before_text_input() {
        let keybinds = configured_keybinds();

        assert_eq!(
            key_action(
                &keybinds,
                KeyEvent::new(KeyCode::Char('j'), KeyModifiers::ALT)
            ),
            KeyAction::Down
        );
        assert_eq!(
            key_action(
                &keybinds,
                KeyEvent::new(KeyCode::Char('k'), KeyModifiers::ALT)
            ),
            KeyAction::Up
        );
    }

    #[test]
    fn replaced_navigation_does_not_keep_default_bindings() {
        let keybinds = configured_keybinds();

        assert_eq!(
            key_action(&keybinds, KeyEvent::new(KeyCode::Down, KeyModifiers::NONE)),
            KeyAction::Ignore
        );
        assert_eq!(
            key_action(
                &keybinds,
                KeyEvent::new(KeyCode::Char('n'), KeyModifiers::CONTROL)
            ),
            KeyAction::Ignore
        );
    }
}
