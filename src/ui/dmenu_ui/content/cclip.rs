//! Lazy, bounded cclip content hydration for the selected preview.

use super::super::DmenuUI;
use std::process::Command;

const MAX_CONTENT_REQUESTS: usize = 4;

impl<'a> DmenuUI<'a> {
    /// Check if an Item is a cclip item (has tab-separated format with rowid).
    pub(super) fn is_cclip_item(&self, item: &crate::common::Item) -> bool {
        if item.original_line.trim().is_empty() {
            return false;
        }

        let parts: Vec<&str> = item.original_line.splitn(3, '\t').collect();
        if parts.len() >= 2 {
            return parts[0].trim().parse::<u64>().is_ok();
        }

        false
    }

    /// Check if an Item is a cclip image item by parsing its original line.
    pub fn is_cclip_image_item(&self, item: &crate::common::Item) -> bool {
        if item.original_line.trim().is_empty() {
            return false;
        }

        let parts: Vec<&str> = item.original_line.splitn(4, '\t').collect();
        if parts.len() >= 2 {
            let mime_type = parts[1].trim();
            return !mime_type.is_empty() && mime_type.starts_with("image/");
        }

        false
    }

    /// Get actual clipboard content for display.
    pub(super) fn get_cclip_content_for_display(&mut self, item: &crate::common::Item) -> String {
        let parts: Vec<&str> = item.original_line.splitn(4, '\t').collect();

        if parts.len() >= 3 {
            let rowid = parts[0].trim();
            let mime_type = parts[1].trim();
            let preview = parts[2];

            self.drain_cclip_content_results();
            let content = if let Some(cached_content) = self.content_cache.get(rowid) {
                cached_content.clone()
            } else if self.content_failures.contains(rowid) {
                format!("[Failed to get content for rowid {rowid}]")
            } else if crate::modes::cclip::html::is_textual_mime(mime_type) {
                let raw_content = self.cclip_verbosity > 0;
                self.start_cclip_content_request(rowid, mime_type, raw_content);
                let display_preview = content_for_view(mime_type, preview, raw_content);
                if display_preview.is_empty() {
                    "[Loading content...]".to_string()
                } else {
                    display_preview
                }
            } else {
                format!("[{mime_type} content]")
            };

            self.add_diagnostics(rowid, mime_type, content)
        } else if parts.len() >= 2 {
            format!("[{} content]", parts[1].trim())
        } else {
            item.original_line.clone()
        }
    }

    fn start_cclip_content_request(&mut self, rowid: &str, mime_type: &str, raw_content: bool) {
        if self.content_requests.contains(rowid)
            || self.content_failures.contains(rowid)
            || self.content_requests.len() >= MAX_CONTENT_REQUESTS
        {
            return;
        }

        let rowid_owned = rowid.to_string();
        let mime_type_owned = mime_type.to_string();
        let generation = self.content_generation;
        let sender = self.content_sender.clone();
        self.content_requests.insert(rowid.to_string());
        std::thread::spawn(move || {
            let content = Command::new("cclip")
                .args(["get", &rowid_owned])
                .output()
                .ok()
                .filter(|output| output.status.success())
                .and_then(|output| {
                    decode_content_for_view(&mime_type_owned, output.stdout, raw_content)
                });
            let _ = sender.send((generation, rowid_owned, content));
        });
    }

    pub(super) fn drain_cclip_content_results(&mut self) {
        while let Ok((generation, rowid, content)) = self.content_receiver.try_recv() {
            if generation != self.content_generation {
                continue;
            }
            self.content_requests.remove(&rowid);
            if let Some(content) = content {
                self.content_cache.insert(rowid, content);
            } else {
                self.content_failures.insert(rowid);
            }
        }
    }

    fn add_diagnostics(&self, rowid: &str, mime_type: &str, content: String) -> String {
        if self.cclip_verbosity < 3 {
            return content;
        }

        format!("[cclip rowid={rowid} mime={mime_type} view=raw] {content}")
    }

    pub(crate) fn get_cclip_diagnostics(&self, item: &crate::common::Item) -> Option<String> {
        if self.cclip_verbosity < 3 {
            return None;
        }

        let mut parts = item.original_line.splitn(3, '\t');
        let rowid = parts.next()?.trim();
        let mime_type = parts.next()?.trim();
        Some(format!("[cclip rowid={rowid} mime={mime_type} view=image]"))
    }

    /// Get image info for display in the preview panel.
    pub fn get_image_info(&self, item: &crate::common::Item) -> String {
        if !self.is_cclip_image_item(item) {
            return String::new();
        }

        let parts: Vec<&str> = item.original_line.splitn(4, '\t').collect();
        if parts.len() >= 3 {
            let preview = parts[2];
            if !preview.is_empty() {
                preview.to_string()
            } else {
                "Unknown Image".to_string()
            }
        } else {
            "Unknown Image".to_string()
        }
    }

    /// Get the rowid for any cclip item (not just images).
    pub fn get_cclip_rowid(&self, item: &crate::common::Item) -> Option<String> {
        let trimmed = item.original_line.trim();
        if trimmed.is_empty() {
            return None;
        }

        let parts: Vec<&str> = trimmed.splitn(2, '\t').collect();
        let rowid = parts[0].trim();
        if !rowid.is_empty() && rowid.chars().all(|c| c.is_ascii_digit()) {
            return Some(rowid.to_string());
        }

        None
    }
}

fn content_for_view(mime_type: &str, content: &str, raw_content: bool) -> String {
    if raw_content {
        content.to_string()
    } else {
        crate::modes::cclip::html::text_for_display(mime_type, content)
    }
}

fn decode_content_for_view(mime_type: &str, bytes: Vec<u8>, raw_content: bool) -> Option<String> {
    if !crate::modes::cclip::html::is_textual_mime(mime_type) {
        return None;
    }

    let content = crate::modes::cclip::html::decode_text_bytes(mime_type, &bytes).ok()?;
    Some(content_for_view(mime_type, &content, raw_content))
}

#[cfg(test)]
mod tests {
    use super::{DmenuUI, decode_content_for_view};

    #[test]
    fn fetched_html_is_rendered_by_default() {
        let content =
            decode_content_for_view("text/html", b"<p>Hello &amp; goodbye</p>".to_vec(), false);

        assert_eq!(content.as_deref(), Some("Hello & goodbye"));
    }

    #[test]
    fn verbose_fetched_html_stays_raw() {
        let html = "<p>Hello &amp; goodbye</p>";
        let content = decode_content_for_view("text/html", html.as_bytes().to_vec(), true);

        assert_eq!(content.as_deref(), Some(html));
    }

    #[test]
    fn fetched_html_uses_its_declared_charset() {
        let content = decode_content_for_view(
            "text/html;charset=iso-8859-1",
            b"<p>caf\xe9</p>".to_vec(),
            false,
        );

        assert_eq!(content.as_deref(), Some("café"));
    }

    #[test]
    fn textual_application_mime_is_displayed() {
        let content = decode_content_for_view(
            "application/javascript",
            b"const answer = 42;".to_vec(),
            false,
        );

        assert_eq!(content.as_deref(), Some("const answer = 42;"));
    }

    #[test]
    fn png_bytes_are_never_lossily_rendered_as_text() {
        let png = b"\x89PNG\r\n\x1a\n\0\0\0\rIHDR".to_vec();

        assert_eq!(decode_content_for_view("text/html", png, false), None);
    }

    #[test]
    fn binary_mime_is_rejected_even_when_its_bytes_are_valid_utf8() {
        let bytes = b"apparently readable binary".to_vec();

        assert_eq!(decode_content_for_view("image/svg", bytes, true), None);
    }

    #[test]
    fn completed_content_requests_clear_the_pending_wakeup() {
        let mut ui = DmenuUI::new(Vec::new(), false, false);
        ui.content_requests.insert("42".to_string());
        ui.content_sender
            .send((
                ui.content_generation,
                "42".to_string(),
                Some("ready".to_string()),
            ))
            .expect("content result should send");

        assert!(ui.has_pending_cclip_content());
        ui.drain_cclip_content_results();

        assert!(!ui.has_pending_cclip_content());
        assert_eq!(
            ui.content_cache.get("42").map(String::as_str),
            Some("ready")
        );
    }

    #[test]
    fn stale_content_generations_are_discarded() {
        let mut ui = DmenuUI::new(Vec::new(), false, false);
        ui.content_sender
            .send((
                ui.content_generation.wrapping_add(1),
                "42".to_string(),
                Some("stale".to_string()),
            ))
            .expect("stale result should send");

        ui.drain_cclip_content_results();

        assert!(!ui.content_cache.contains_key("42"));
    }
}
