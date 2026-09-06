//! HTML clipboard decoding and lightweight visible-text rendering.

use encoding_rs::{Encoding, UTF_8};
use std::borrow::Cow;

const HIDDEN_ELEMENTS: &[&str] = &["head", "noscript", "script", "style", "svg", "template"];
const RAW_TEXT_ELEMENTS: &[&str] = &["script", "style"];
const BOUNDARY_ELEMENTS: &[&str] = &[
    "address",
    "article",
    "aside",
    "blockquote",
    "br",
    "dd",
    "div",
    "dl",
    "dt",
    "fieldset",
    "figcaption",
    "figure",
    "footer",
    "form",
    "h1",
    "h2",
    "h3",
    "h4",
    "h5",
    "h6",
    "header",
    "hr",
    "li",
    "main",
    "nav",
    "ol",
    "p",
    "pre",
    "section",
    "table",
    "td",
    "th",
    "tr",
    "ul",
];

pub(crate) fn is_html_mime(mime_type: &str) -> bool {
    let essence = mime_essence(mime_type);
    essence.eq_ignore_ascii_case("text/html")
        || essence.eq_ignore_ascii_case("application/xhtml+xml")
}

pub(crate) fn is_textual_mime(mime_type: &str) -> bool {
    let normalized = mime_essence(mime_type).to_ascii_lowercase();
    normalized.starts_with("text/")
        || matches!(
            normalized.as_str(),
            "application/ecmascript"
                | "application/javascript"
                | "application/json"
                | "application/sql"
                | "application/toml"
                | "application/x-javascript"
                | "application/x-yaml"
                | "application/xml"
                | "application/yaml"
        )
        || normalized.ends_with("+json")
        || normalized.ends_with("+xml")
}

pub(crate) fn decode_text_bytes(mime_type: &str, bytes: &[u8]) -> Result<String, &'static str> {
    let encoding = match charset(mime_type) {
        Some(label) => Encoding::for_label(label.as_bytes()).ok_or("unsupported MIME charset")?,
        None => UTF_8,
    };
    let (decoded, _, _) = encoding.decode(bytes);
    Ok(decoded.into_owned())
}

pub(crate) fn text_for_display(mime_type: &str, content: &str) -> String {
    if is_html_mime(mime_type) {
        to_plain_text(content)
    } else {
        content.to_string()
    }
}

fn mime_essence(mime_type: &str) -> &str {
    mime_type.split(';').next().unwrap_or(mime_type).trim()
}

fn charset(mime_type: &str) -> Option<&str> {
    mime_type.split(';').skip(1).find_map(|parameter| {
        let (name, value) = parameter.split_once('=')?;
        name.trim()
            .eq_ignore_ascii_case("charset")
            .then(|| value.trim().trim_matches(['\'', '"']))
    })
}

fn to_plain_text(html: &str) -> String {
    let mut renderer = TextRenderer::default();
    let mut cursor = 0;

    while cursor < html.len() {
        let remaining = &html[cursor..];
        if let Some(raw_element) = renderer.raw_text_element()
            && !starts_with_closing_tag(remaining, raw_element)
        {
            let Some(offset) = closing_tag_offset(remaining, raw_element) else {
                break;
            };
            cursor += offset;
            continue;
        }

        if remaining.starts_with("<!--") {
            cursor += comment_len(remaining);
        } else if remaining.starts_with('<') && looks_like_tag(remaining) {
            let Some(tag_len) = tag_len(remaining) else {
                break;
            };
            renderer.push_tag(&remaining[1..tag_len - 1]);
            cursor += tag_len;
        } else {
            let text_len = remaining.find('<').unwrap_or(remaining.len()).max(1);
            renderer.push_text(&remaining[..text_len]);
            cursor += text_len;
        }
    }

    renderer.finish()
}

#[derive(Default)]
struct TextRenderer {
    output: String,
    hidden_elements: Vec<String>,
    pre_depth: usize,
    pending_space: bool,
}

impl TextRenderer {
    fn raw_text_element(&self) -> Option<&str> {
        self.hidden_elements
            .last()
            .filter(|name| is_raw_text_element(name))
            .map(String::as_str)
    }

    fn push_tag(&mut self, raw_tag: &str) {
        let tag = Tag::parse(raw_tag);
        if tag.name.is_empty() {
            return;
        }

        if let Some(hidden) = self.hidden_elements.last() {
            if tag.closing && tag.name.eq_ignore_ascii_case(hidden) {
                self.hidden_elements.pop();
            } else if !tag.closing && !tag.self_closing && is_hidden_element(tag.name) {
                self.hidden_elements.push(tag.name.to_ascii_lowercase());
            }
            return;
        }

        if tag.closing && tag.name.eq_ignore_ascii_case("pre") {
            self.pre_depth = self.pre_depth.saturating_sub(1);
        }
        if is_boundary_element(tag.name) {
            self.pending_space = !self.output.is_empty();
        }
        if !tag.closing && !tag.self_closing {
            if is_hidden_element(tag.name) {
                self.hidden_elements.push(tag.name.to_ascii_lowercase());
            } else if tag.name.eq_ignore_ascii_case("pre") {
                self.pre_depth += 1;
            }
        }
    }

    fn push_text(&mut self, text: &str) {
        if !self.hidden_elements.is_empty() {
            return;
        }

        let decoded = html_escape::decode_html_entities(text);
        match decoded {
            Cow::Borrowed(text) => self.push_decoded(text),
            Cow::Owned(text) => self.push_decoded(&text),
        }
    }

    fn push_decoded(&mut self, decoded: &str) {
        if self.pre_depth > 0 {
            self.push_preformatted(decoded);
        } else {
            for ch in decoded.chars() {
                self.push_char(ch);
            }
        }
    }

    fn push_preformatted(&mut self, text: &str) {
        if self.pending_space {
            if !self.output.is_empty() && !self.output.ends_with(char::is_whitespace) {
                self.output.push('\n');
            }
            self.pending_space = false;
        }
        self.output
            .push_str(&text.replace("\r\n", "\n").replace('\r', "\n"));
    }

    fn push_char(&mut self, ch: char) {
        if ch.is_whitespace() {
            self.pending_space = !self.output.is_empty();
            return;
        }

        if self.pending_space {
            if !self.output.ends_with(char::is_whitespace) {
                self.output.push(' ');
            }
            self.pending_space = false;
        }
        self.output.push(ch);
    }

    fn finish(self) -> String {
        self.output
    }
}

struct Tag<'a> {
    name: &'a str,
    closing: bool,
    self_closing: bool,
}

impl<'a> Tag<'a> {
    fn parse(raw_tag: &'a str) -> Self {
        let trimmed = raw_tag.trim();
        let closing = trimmed.starts_with('/');
        let body = trimmed.trim_start_matches('/').trim_start();
        let name_len = body
            .find(|ch: char| !ch.is_ascii_alphanumeric() && ch != '-' && ch != ':')
            .unwrap_or(body.len());

        Self {
            name: &body[..name_len],
            closing,
            self_closing: body.trim_end().ends_with('/'),
        }
    }
}

fn looks_like_tag(remaining: &str) -> bool {
    matches!(
        remaining.as_bytes().get(1),
        Some(b'!' | b'/' | b'?' | b'A'..=b'Z' | b'a'..=b'z')
    )
}

fn comment_len(remaining: &str) -> usize {
    remaining
        .find("-->")
        .map_or(remaining.len(), |end| end + "-->".len())
}

fn tag_len(remaining: &str) -> Option<usize> {
    let mut quote = None;
    for (offset, ch) in remaining.char_indices().skip(1) {
        match (quote, ch) {
            (Some(opening), closing) if opening == closing => quote = None,
            (None, '"' | '\'') => quote = Some(ch),
            (None, '>') => return Some(offset + ch.len_utf8()),
            _ => {}
        }
    }
    None
}

fn closing_tag_offset(input: &str, name: &str) -> Option<usize> {
    input
        .char_indices()
        .find_map(|(offset, _)| starts_with_closing_tag(&input[offset..], name).then_some(offset))
}

fn starts_with_closing_tag(input: &str, name: &str) -> bool {
    if input.as_bytes().get(..2) != Some(b"</") {
        return false;
    }
    let name_end = name.len().saturating_add(2);
    let Some(candidate) = input.get(2..name_end) else {
        return false;
    };
    if !candidate.eq_ignore_ascii_case(name) {
        return false;
    }
    input[name_end..]
        .chars()
        .next()
        .is_some_and(|ch| ch == '>' || ch.is_ascii_whitespace())
}

fn is_hidden_element(name: &str) -> bool {
    HIDDEN_ELEMENTS
        .iter()
        .any(|element| name.eq_ignore_ascii_case(element))
}

fn is_raw_text_element(name: &str) -> bool {
    RAW_TEXT_ELEMENTS
        .iter()
        .any(|element| name.eq_ignore_ascii_case(element))
}

fn is_boundary_element(name: &str) -> bool {
    BOUNDARY_ELEMENTS
        .iter()
        .any(|element| name.eq_ignore_ascii_case(element))
}

#[cfg(test)]
mod tests {
    use super::{decode_text_bytes, is_textual_mime, text_for_display, to_plain_text};

    #[test]
    fn renders_visible_text_without_tags_or_metadata() {
        let html = concat!(
            r#"<meta http-equiv="content-type" content="text/html; charset=utf-8">"#,
            r#"<div class="message"><strong>Hello</strong> world</div>"#
        );
        assert_eq!(to_plain_text(html), "Hello world");
    }

    #[test]
    fn decodes_standard_named_and_numeric_entities() {
        let html = "<p>&copy; Tom &amp; Jerry&nbsp;&#x1F63A; &euro; &mdash; &#62;</p>";
        assert_eq!(to_plain_text(html), "© Tom & Jerry 😺 € — >");
    }

    #[test]
    fn omits_raw_text_even_when_it_contains_tag_like_text() {
        let html = concat!(
            "<style>.secret::after { content: '<style>'; }</style>",
            "<p>Visible</p>",
            "<script>const café = '<script>'; alert(café)</script>",
            "<svg><title>Icon</title></svg>"
        );
        assert_eq!(to_plain_text(html), "Visible");
    }

    #[test]
    fn preserves_preformatted_whitespace() {
        let html = "<pre>first\n  second\n\tthird</pre>";
        assert_eq!(to_plain_text(html), "first\n  second\n\tthird");
    }

    #[test]
    fn preserves_preformatted_edge_newlines() {
        let html = "<pre>\nfirst\n</pre>";
        assert_eq!(to_plain_text(html), "\nfirst\n");
    }

    #[test]
    fn preserves_boundaries_between_block_elements() {
        let html = "<div>first</div><div>second<br>third</div>";
        assert_eq!(to_plain_text(html), "first second third");
    }

    #[test]
    fn leaves_non_html_content_unchanged() {
        let text = "2 < 3 & plain";
        assert_eq!(text_for_display("text/plain;charset=utf-8", text), text);
    }

    #[test]
    fn drops_unfinished_markup_from_truncated_previews() {
        assert_eq!(to_plain_text("<strong title=\"unfinished 😺"), "");
    }

    #[test]
    fn ampersands_without_entities_remain_plain_text() {
        let text = "&".repeat(32_768);
        assert_eq!(to_plain_text(&text), text);
    }

    #[test]
    fn decodes_the_declared_mime_charset() {
        let decoded = decode_text_bytes("text/html; charset=iso-8859-1", b"caf\xe9")
            .expect("declared charset should decode");
        assert_eq!(decoded, "café");
    }

    #[test]
    fn replaces_invalid_default_utf8_sequences() {
        let decoded = decode_text_bytes("text/html", b"<p>caf\xe9</p>")
            .expect("invalid UTF-8 should remain displayable");
        assert_eq!(decoded, "<p>caf�</p>");
    }

    #[test]
    fn recognizes_textual_application_mime_types() {
        assert!(is_textual_mime("application/javascript"));
        assert!(is_textual_mime("application/problem+json"));
        assert!(!is_textual_mime("application/octet-stream"));
    }
}
