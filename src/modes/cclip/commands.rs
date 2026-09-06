//! Noninteractive cclip commands and tag-list output.

use eyre::{Result, WrapErr};

use crate::cli::Opts;

pub(super) fn handle_noninteractive_mode(cli: &Opts) -> Result<bool> {
    if cli.cclip_clear_tags {
        clear_tag_metadata()?;
        println!("Cleared all tag metadata from fsel database");
        println!();
        println!("Note: To wipe tags from cclip entries too, use:");
        println!("  fsel --cclip --tag wipe");
        return Ok(true);
    }

    if cli.cclip_wipe_tags {
        super::select::wipe_all_tags().wrap_err("Failed to wipe cclip tags")?;
        println!("Wiped all tags from cclip entries");
        clear_tag_metadata()?;
        println!("Cleared all tag metadata from fsel database");
        return Ok(true);
    }

    if cli.cclip_tag_list {
        print_tag_list(cli)?;
        return Ok(true);
    }

    Ok(false)
}

pub(super) fn load_history(cli: &Opts) -> Result<Vec<super::CclipItem>> {
    if let Some(ref tag_name) = cli.cclip_tag {
        super::scan::get_clipboard_history_by_tag(tag_name).wrap_err(format!(
            "Failed to get clipboard history for tag '{}'",
            tag_name
        ))
    } else {
        super::scan::get_clipboard_history().wrap_err("Failed to get clipboard history from cclip")
    }
}

fn clear_tag_metadata() -> Result<()> {
    let (db, _) = crate::core::database::open_history_db()?;
    let write_txn = db.begin_write()?;
    {
        let mut table = write_txn.open_table(super::metadata::TAG_METADATA_TABLE)?;
        let _ = table.remove("tag_metadata");
    }
    write_txn.commit()?;
    Ok(())
}

fn print_tag_list(cli: &Opts) -> Result<()> {
    let tags = super::scan::get_all_tags().wrap_err("Failed to get tags from cclip")?;

    if tags.is_empty() {
        println!("No tags found");
        return Ok(());
    }

    if let Some(ref tag_name) = cli.cclip_tag {
        println!("Items tagged with '{}':", tag_name);
        let items = super::scan::get_clipboard_history_by_tag(tag_name)
            .wrap_err("Failed to get items by tag")?;

        if items.is_empty() {
            println!("  (no items)");
            return Ok(());
        }

        for item in items {
            let preview = printable_preview(&item);
            if cli.verbose.unwrap_or(0) >= 2 {
                println!("  [{}] {} - {}", item.rowid, item.mime_type, preview);
            } else {
                println!("  {preview}");
            }
        }
        return Ok(());
    }

    println!("Available tags:");
    for tag in tags {
        if cli.verbose.unwrap_or(0) >= 2 {
            let items = match super::scan::get_clipboard_history_by_tag(&tag) {
                Ok(items) => items,
                Err(error) => {
                    eprintln!(
                        "Failed to load clipboard history for tag '{}': {}",
                        tag, error
                    );
                    Vec::new()
                }
            };
            println!("  {} ({} items)", tag, items.len());
        } else {
            println!("  {}", tag);
        }
    }

    Ok(())
}

fn printable_preview(item: &super::CclipItem) -> String {
    if !super::html::is_html_mime(&item.mime_type) || !item.preview.is_empty() {
        return item.preview.clone();
    }

    let fetched = item.get_content_for_preview().ok();
    noninteractive_preview(&item.mime_type, &item.preview, fetched.as_deref())
}

fn noninteractive_preview(mime_type: &str, preview: &str, fetched: Option<&[u8]>) -> String {
    let Some(bytes) = fetched else {
        return if preview.is_empty() && super::html::is_html_mime(mime_type) {
            "[HTML content]".to_string()
        } else {
            preview.to_string()
        };
    };
    super::html::decode_text_bytes(mime_type, bytes).map_or_else(
        |_| preview.to_string(),
        |content| super::html::text_for_display(mime_type, &content),
    )
}

#[cfg(test)]
mod tests {
    use super::noninteractive_preview;

    #[test]
    fn noninteractive_html_output_uses_fetched_visible_text() {
        assert_eq!(
            noninteractive_preview("text/html", "", Some(b"<p>full &amp; readable</p>")),
            "full & readable"
        );
    }

    #[test]
    fn noninteractive_html_output_keeps_preview_when_fetch_fails() {
        assert_eq!(
            noninteractive_preview("text/html", "summary", None),
            "summary"
        );
        assert_eq!(
            noninteractive_preview("text/html", "", None),
            "[HTML content]"
        );
    }
}
