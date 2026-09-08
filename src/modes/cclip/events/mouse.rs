//! Cclip mouse selection mapped to the visible history panel.

use super::selection::copy_selected_and_exit_at;
use super::{EventContext, EventOutcome, LoopControl};
use crate::ui::TagMode;
use crossterm::event::{MouseButton, MouseEvent, MouseEventKind};
use eyre::Result;

pub(super) fn handle_mouse_event(
    ctx: &mut EventContext<'_, '_>,
    mouse_event: MouseEvent,
) -> Result<EventOutcome> {
    let mouse_row = mouse_event.row;
    let geometry = ctx.options.result_layout(ctx.terminal.size()?.into());
    let hit = geometry.hit(mouse_event.column, mouse_row);
    let max_visible_rows = geometry.capacity();

    let update_selection_for_mouse_pos = |ui: &mut crate::ui::DmenuUI<'_>| {
        if !ui.shown.is_empty()
            && let Some(row_in_content) = hit
        {
            let hovered_item_index = ui.scroll_offset + row_in_content;
            if hovered_item_index < ui.shown.len() {
                ui.selected = Some(hovered_item_index);
            }
        }
    };

    match mouse_event.kind {
        MouseEventKind::Moved => {
            if matches!(ctx.ui.tag_mode, TagMode::Normal) {
                update_selection_for_mouse_pos(ctx.ui);
            }
        }
        MouseEventKind::Down(MouseButton::Left) => {
            if !matches!(ctx.ui.tag_mode, TagMode::Normal) {
                return Ok(EventOutcome {
                    control: LoopControl::Continue,
                    needs_redraw: true,
                });
            }

            if !ctx.ui.shown.is_empty()
                && let Some(row_in_content) = hit
            {
                let clicked_item_index = ctx.ui.scroll_offset + row_in_content;
                if clicked_item_index < ctx.ui.shown.len()
                    && copy_selected_and_exit_at(ctx, clicked_item_index)?
                {
                    return Ok(EventOutcome {
                        control: LoopControl::Exit,
                        needs_redraw: true,
                    });
                }
            }
        }
        MouseEventKind::ScrollUp
            if matches!(ctx.ui.tag_mode, TagMode::Normal)
                && hit.is_some()
                && !ctx.ui.shown.is_empty()
                && ctx.ui.scroll_offset > 0 =>
        {
            ctx.ui.scroll_offset -= 1;
            update_selection_for_mouse_pos(ctx.ui);
        }
        MouseEventKind::ScrollDown
            if matches!(ctx.ui.tag_mode, TagMode::Normal)
                && hit.is_some()
                && !ctx.ui.shown.is_empty()
                && max_visible_rows > 0 =>
        {
            let max_visible = max_visible_rows;
            if ctx.ui.scroll_offset + max_visible < ctx.ui.shown.len() {
                ctx.ui.scroll_offset += 1;
                update_selection_for_mouse_pos(ctx.ui);
            }
        }
        _ => {}
    }

    Ok(EventOutcome {
        control: LoopControl::Continue,
        needs_redraw: true,
    })
}
