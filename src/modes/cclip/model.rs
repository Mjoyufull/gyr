//! Parsed cclip history entries and their list labels.

use crate::common::Item;
use eyre::{Result, eyre};

use super::{TagMetadataFormatter, html};

/// Represents a clipboard entry from cclip with MIME type information
#[derive(Debug, Clone)]
pub struct CclipItem {
    pub rowid: String,
    pub mime_type: String,
    pub preview: String,
    pub original_line: String,
    pub tags: Vec<String>,
}

impl CclipItem {
    /// Create a new CclipItem from a tab-separated line from cclip list
    /// Format: rowid\tmime_type\tpreview[\ttags]
    /// The optional `tags` field is a comma-separated list of tag names.
    pub fn from_line(line: String) -> Result<Self> {
        let parts: Vec<&str> = line.splitn(4, '\t').collect();

        if parts.len() < 3 {
            return Err(eyre!(
                "Invalid cclip list format: expected at least 3 tab-separated fields"
            ));
        }

        let tags = if parts.len() >= 4 {
            parts[3]
                .split(',')
                .filter_map(|tag| {
                    let trimmed = tag.trim();
                    if trimmed.is_empty() {
                        None
                    } else {
                        Some(trimmed.to_string())
                    }
                })
                .collect()
        } else {
            Vec::new()
        };

        let mime_type = parts[1].to_string();
        let preview = html::text_for_display(&mime_type, parts[2]);

        Ok(CclipItem {
            rowid: parts[0].to_string(),
            mime_type,
            preview,
            original_line: line,
            tags,
        })
    }

    /// Get a human-readable display name for this item using optional tag metadata formatting
    pub fn get_display_name_with_formatter(
        &self,
        formatter: Option<&TagMetadataFormatter>,
    ) -> String {
        self.get_display_name_with_formatter_options(formatter, true)
    }

    pub fn get_display_name_with_formatter_options(
        &self,
        formatter: Option<&TagMetadataFormatter>,
        include_color_names: bool,
    ) -> String {
        let base_name = match self.mime_type.as_str() {
            mime if mime.starts_with("image/") => {
                format!(
                    "{} ({})",
                    self.preview.chars().take(50).collect::<String>(),
                    mime
                )
            }
            mime if mime.starts_with("text/") => {
                let preview = self.preview.chars().take(80).collect::<String>();
                if preview.is_empty() && html::is_html_mime(mime) {
                    "[HTML content]".to_string()
                } else {
                    preview
                }
            }
            _ => {
                format!(
                    "{} ({})",
                    self.preview.chars().take(50).collect::<String>(),
                    self.mime_type
                )
            }
        };

        format_tags_for_display(&self.tags, base_name, formatter, include_color_names)
    }

    /// Get a human-readable display name without metadata formatting
    pub fn get_display_name(&self) -> String {
        self.get_display_name_with_formatter(None)
    }

    /// Get display name with rowid number prefix (for show_line_numbers)
    pub fn get_display_name_with_number(&self) -> String {
        self.get_display_name_with_number_formatter(None)
    }

    pub fn get_display_name_with_number_formatter(
        &self,
        formatter: Option<&TagMetadataFormatter>,
    ) -> String {
        self.get_display_name_with_number_formatter_options(formatter, true)
    }

    pub fn get_display_name_with_number_formatter_options(
        &self,
        formatter: Option<&TagMetadataFormatter>,
        include_color_names: bool,
    ) -> String {
        let base_name =
            self.get_display_name_with_formatter_options(formatter, include_color_names);
        let id_width = self.rowid.to_string().len().max(3);
        format!("{:<width$} {}", self.rowid, base_name, width = id_width)
    }
}

/// Convert CclipItem to Item for use with existing dmenu infrastructure
impl From<CclipItem> for Item {
    fn from(item: CclipItem) -> Self {
        let mut item_struct = Item::new_simple(
            item.original_line.clone(),
            item.get_display_name(),
            1, // line number, not really applicable for cclip
        );
        item_struct.tags = Some(item.tags);
        item_struct
    }
}

fn format_tags_for_display(
    tags: &[String],
    base: String,
    formatter: Option<&TagMetadataFormatter>,
    include_color_names: bool,
) -> String {
    if tags.is_empty() {
        return base;
    }

    let display_tags: Vec<String> = if let Some(formatter) = formatter {
        formatter.format_tags_with_options(tags, include_color_names)
    } else {
        tags.to_vec()
    };

    format!("[{}] {}", display_tags.join(", "), base)
}

#[cfg(test)]
mod tests {
    use super::CclipItem;

    #[test]
    fn html_items_render_text_but_preserve_original_clipboard_record() {
        let original = concat!(
            "42\ttext/html\t",
            r#"<meta content="text/html"><p>Hello &amp; goodbye</p>"#,
            "\ttag"
        )
        .to_string();

        let item = CclipItem::from_line(original.clone()).expect("valid cclip item");

        assert_eq!(item.preview, "Hello & goodbye");
        assert_eq!(item.get_display_name(), "[tag] Hello & goodbye");
        assert_eq!(item.original_line, original);
    }

    #[test]
    fn truncated_html_markup_uses_the_lazy_content_placeholder() {
        let item = CclipItem::from_line("42\ttext/html\t<meta charset=\"utf-8".to_string())
            .expect("truncated cclip preview should parse");

        assert!(item.preview.is_empty());
        assert_eq!(item.get_display_name(), "[HTML content]");
    }
}
