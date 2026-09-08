use super::items::reload_visible_history;
use crate::cli::Opts;
use crate::ui::{DmenuUI, TagMode};
use eyre::{Result, WrapErr};
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;
use std::collections::HashMap;
use std::io;

use super::{TagMetadata, TagMetadataFormatter};

pub(super) struct TagSubmitContext<'a, 'ui> {
    pub(super) ui: &'a mut DmenuUI<'ui>,
    pub(super) cli: &'a Opts,
    pub(super) db: &'a std::sync::Arc<redb::Database>,
    pub(super) tag_metadata_map: &'a mut HashMap<String, TagMetadata>,
    pub(super) tag_metadata_formatter: &'a mut TagMetadataFormatter,
    pub(super) show_line_numbers: bool,
    pub(super) show_tag_color_names: bool,
    pub(super) max_visible: usize,
}

pub(super) fn begin_tag_creation(
    ui: &mut DmenuUI<'_>,
    image_runtime: &mut super::image::ImageRuntime,
    terminal: &mut Terminal<CrosstermBackend<io::Stderr>>,
) -> Result<()> {
    image_runtime.clear_inline_image();
    image_runtime.request_buffer_sync();
    crate::ui::terminal::clear_fullscreen(terminal).wrap_err("Failed to clear terminal")?;

    if let Some(selected_idx) = ui.selected
        && !ui.shown.is_empty()
        && selected_idx < ui.shown.len()
    {
        let selected_item = ui.shown[selected_idx].original_line.clone();
        let available_tags = super::scan::get_all_tags().unwrap_or_default();
        ui.tag_mode = TagMode::PromptingTagName {
            input: String::new(),
            selected_item: Some(selected_item),
            available_tags,
            selected_tag: None,
        };
    } else {
        ui.set_temp_message("No item selected - cannot create tag".to_string());
    }

    Ok(())
}

pub(super) fn begin_tag_removal(ui: &mut DmenuUI<'_>) {
    if let Some(selected_idx) = ui.selected
        && selected_idx < ui.shown.len()
    {
        let item = &ui.shown[selected_idx];
        let selected_item = Some(item.original_line.clone());
        match super::CclipItem::from_line(item.original_line.clone()) {
            Ok(cclip_item) if !cclip_item.tags.is_empty() => {
                ui.tag_mode = TagMode::RemovingTag {
                    input: cclip_item.tags[0].clone(),
                    tags: cclip_item.tags.clone(),
                    selected: Some(0),
                    selected_item,
                };
            }
            Ok(cclip_item) => {
                ui.tag_mode = TagMode::RemovingTag {
                    input: String::new(),
                    tags: cclip_item.tags.clone(),
                    selected: None,
                    selected_item,
                };
            }
            Err(error) => {
                ui.set_temp_message(format!("Failed to parse item: {}", error));
            }
        }
    } else {
        ui.set_temp_message("No item selected - cannot remove tag".to_string());
    }
}

pub(super) fn submit_tag_mode(mut ctx: TagSubmitContext<'_, '_>) {
    match ctx.ui.tag_mode.clone() {
        TagMode::PromptingTagName {
            input,
            selected_item,
            ..
        } => submit_tag_name(ctx.ui, input, selected_item),
        TagMode::PromptingTagEmoji {
            tag_name,
            input,
            selected_item,
        } => submit_tag_emoji(ctx.ui, tag_name, input, selected_item),
        TagMode::PromptingTagColor {
            tag_name,
            emoji,
            input,
            selected_item,
        } => submit_tag_color(&mut ctx, tag_name, emoji, input, selected_item),
        TagMode::RemovingTag {
            input,
            selected_item,
            ..
        } => submit_tag_removal(&mut ctx, input, selected_item),
        TagMode::Normal => {}
    }
}

fn submit_tag_name(ui: &mut DmenuUI<'_>, input: String, selected_item: Option<String>) {
    let tag_name = input.trim().to_string();
    if tag_name.is_empty() {
        ui.tag_mode = TagMode::PromptingTagEmoji {
            tag_name: String::new(),
            input: String::new(),
            selected_item,
        };
        return;
    }

    let existing_tag_metadata = if let Some(ref item_line) = selected_item {
        match super::CclipItem::from_line(item_line.clone()) {
            Ok(cclip_item) if cclip_item.tags.contains(&tag_name) => Some(tag_name.clone()),
            _ => None,
        }
    } else {
        None
    };

    if existing_tag_metadata.is_some() {
        ui.set_temp_message(format!("Tag '{}' already applied (editing)", tag_name));
    }

    ui.tag_mode = TagMode::PromptingTagEmoji {
        tag_name,
        input: String::new(),
        selected_item,
    };
}

fn submit_tag_emoji(
    ui: &mut DmenuUI<'_>,
    tag_name: String,
    input: String,
    selected_item: Option<String>,
) {
    let emoji = if input.trim().is_empty() {
        None
    } else {
        Some(input.trim().to_string())
    };

    if tag_name.is_empty() && emoji.is_none() {
        ui.set_temp_message("Tag requires either a name or an emoji".to_string());
        ui.tag_mode = TagMode::Normal;
        return;
    }

    let final_tag_name = if tag_name.is_empty() {
        emoji.clone().unwrap_or_default()
    } else {
        tag_name
    };

    ui.tag_mode = TagMode::PromptingTagColor {
        tag_name: final_tag_name,
        emoji,
        input: String::new(),
        selected_item,
    };
}

fn submit_tag_color(
    ctx: &mut TagSubmitContext<'_, '_>,
    tag_name: String,
    emoji: Option<String>,
    input: String,
    selected_item: Option<String>,
) {
    let color = if input.trim().is_empty() {
        None
    } else {
        Some(input.trim().to_string())
    };

    let is_editing = if let Some(ref item_line) = selected_item {
        match super::CclipItem::from_line(item_line.clone()) {
            Ok(cclip_item) => cclip_item.tags.contains(&tag_name),
            Err(_) => false,
        }
    } else {
        false
    };

    let Some(item_line) = selected_item else {
        ctx.ui.set_temp_message("No item selected".to_string());
        ctx.ui.tag_mode = TagMode::Normal;
        return;
    };
    let Some(rowid) = rowid_from_item_line(&item_line) else {
        ctx.ui
            .set_temp_message("Unable to determine item id".to_string());
        ctx.ui.tag_mode = TagMode::Normal;
        return;
    };

    if !is_editing && let Err(error) = super::select::tag_item(rowid, &tag_name) {
        ctx.ui
            .set_temp_message(format!("Failed to tag item: {}", error));
        ctx.ui.tag_mode = TagMode::Normal;
        return;
    }

    let mut updated_metadata = ctx.tag_metadata_map.clone();
    updated_metadata.insert(
        tag_name.clone(),
        TagMetadata {
            name: tag_name.clone(),
            color,
            emoji,
        },
    );
    if let Err(error) = super::save_tag_metadata(ctx.db, &updated_metadata) {
        ctx.ui
            .set_temp_message(format!("Failed to save tag metadata: {}", error));
        ctx.ui.tag_mode = TagMode::Normal;
        return;
    }
    *ctx.tag_metadata_map = updated_metadata;
    *ctx.tag_metadata_formatter = TagMetadataFormatter::new(ctx.tag_metadata_map.clone());
    reload_history(ctx);

    ctx.ui.tag_mode = TagMode::Normal;
}

fn submit_tag_removal(
    ctx: &mut TagSubmitContext<'_, '_>,
    input: String,
    selected_item: Option<String>,
) {
    let Some(item_line) = selected_item else {
        ctx.ui.set_temp_message("No item selected".to_string());
        ctx.ui.tag_mode = TagMode::Normal;
        return;
    };
    let Some(rowid) = rowid_from_item_line(&item_line) else {
        ctx.ui
            .set_temp_message("Unable to determine item id".to_string());
        ctx.ui.tag_mode = TagMode::Normal;
        return;
    };

    let trimmed_input = input.trim();
    let tag_to_remove = (!trimmed_input.is_empty()).then_some(trimmed_input);

    match super::select::untag_item(rowid, tag_to_remove) {
        Err(error) => ctx
            .ui
            .set_temp_message(format!("Failed to remove tag: {}", error)),
        Ok(()) => {
            reload_history(ctx);
            ctx.ui.set_temp_message(match tag_to_remove {
                Some(tag) => format!("Removed tag '{tag}'"),
                None => "Removed all tags".to_string(),
            });
        }
    }

    ctx.ui.tag_mode = TagMode::Normal;
}

fn rowid_from_item_line(item_line: &str) -> Option<&str> {
    item_line
        .split('\t')
        .next()
        .filter(|rowid| !rowid.is_empty())
}

fn reload_history(ctx: &mut TagSubmitContext<'_, '_>) {
    reload_visible_history(
        ctx.ui,
        ctx.cli,
        ctx.tag_metadata_formatter,
        ctx.show_line_numbers,
        ctx.show_tag_color_names,
        ctx.max_visible,
    );
}
