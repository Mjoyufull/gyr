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
    let (items_content_start, max_visible_rows) = ctx
        .options
        .items_content_bounds(ctx.terminal.size()?.height);
    let items_content_end = items_content_start + max_visible_rows;

    let update_selection_for_mouse_pos = |ui: &mut crate::ui::DmenuUI<'_>| {
        if !ui.shown.is_empty() && mouse_row >= items_content_start && mouse_row < items_content_end
        {
            let row_in_content = mouse_row - items_content_start;
            let hovered_item_index = ui.scroll_offset + row_in_content as usize;
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

            if mouse_row >= items_content_start
                && mouse_row < items_content_end
                && !ctx.ui.shown.is_empty()
            {
                let row_in_content = mouse_row - items_content_start;
                let clicked_item_index = ctx.ui.scroll_offset + row_in_content as usize;
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
                && !ctx.ui.shown.is_empty()
                && ctx.ui.scroll_offset > 0 =>
        {
            ctx.ui.scroll_offset -= 1;
            update_selection_for_mouse_pos(ctx.ui);
        }
        MouseEventKind::ScrollDown
            if matches!(ctx.ui.tag_mode, TagMode::Normal)
                && !ctx.ui.shown.is_empty()
                && max_visible_rows > 0 =>
        {
            let max_visible = max_visible_rows as usize;
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
