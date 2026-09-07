use eyre::{Result, eyre};
use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui_image::picker::{Picker, ProtocolType};
use ratatui_image::protocol::StatefulProtocol;
use ratatui_image::{Resize, StatefulImage};
use std::collections::{HashMap, VecDeque};
use std::io::{Cursor, Read};
use std::process::Stdio;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, OnceLock};

const MAX_SVG_DIMENSION: f32 = 2048.0;
const MAX_SVG_DOCUMENT_BYTES: usize = 32 * 1024 * 1024;
const MAX_SVG_PROBE_BYTES: u64 = 64 * 1024;
const MAX_SVG_EMBEDDED_RASTER_DIMENSION: u32 = 4096;
const MAX_SVG_EMBEDDED_RASTER_PIXELS: u64 = 2048 * 2048;
const SVG_NAMESPACE: &str = "http://www.w3.org/2000/svg";
static SVG_FONT_DATABASE: OnceLock<Arc<resvg::usvg::fontdb::Database>> = OnceLock::new();

/// Combined display state to track what's currently on screen
#[derive(Debug, Clone, PartialEq)]
pub enum DisplayState {
    /// No content displayed
    Empty,
    /// Image content is displayed with rowid
    Image(String),
    /// Image is currently loading in background
    Loading(String),
    /// Failed to load with error message
    Failed(String),
}

/// Single atomic state tracker to eliminate lock contention
pub static DISPLAY_STATE: Mutex<DisplayState> = Mutex::new(DisplayState::Empty);

/// Manages image loading and rendering using ratatui-image
pub struct ImageManager {
    picker: Picker,
    current_rowid: Option<String>,
    cache: HashMap<String, StatefulProtocol>,
    cache_order: VecDeque<String>,
    cache_capacity: usize,
}

impl ImageManager {
    /// Initialize the image manager with the picker chosen by the caller.
    pub fn new(picker: Picker) -> Self {
        Self {
            picker,
            current_rowid: None,
            cache: HashMap::new(),
            cache_order: VecDeque::new(),
            cache_capacity: 50,
        }
    }

    /// Check if an image is already in cache
    pub fn is_cached(&self, rowid: &str) -> bool {
        self.cache.contains_key(rowid)
    }

    /// Decode image bytes and cache the terminal protocol under `key`.
    pub async fn load_image_bytes(&mut self, key: &str, bytes: Vec<u8>) -> Result<()> {
        if self.cache.contains_key(key) {
            self.update_lru(key);
            return Ok(());
        }

        let protocol = Self::prepare_image_bytes(self.picker.clone(), bytes).await?;

        self.insert_protocol(key.to_string(), protocol);
        Ok(())
    }

    /// Decode bytes and prepare terminal protocol state off the async executor.
    pub(crate) async fn prepare_image_bytes(
        picker: Picker,
        bytes: Vec<u8>,
    ) -> Result<StatefulProtocol> {
        tokio::task::spawn_blocking(move || Self::prepare_image_bytes_blocking(picker, bytes))
            .await?
    }

    /// Decode bytes and prepare terminal protocol state from a blocking worker.
    pub(crate) fn prepare_image_bytes_blocking(
        picker: Picker,
        bytes: Vec<u8>,
    ) -> Result<StatefulProtocol> {
        let image = decode_image(&bytes)?;
        Ok(picker.new_resize_protocol(image))
    }

    /// Return whether bytes have a supported raster signature or contain SVG markup.
    pub fn recognizes_image_bytes(bytes: &[u8]) -> bool {
        image::guess_format(bytes).is_ok() || looks_like_svg(bytes)
    }

    /// Insert a prepared terminal image protocol into the bounded cache.
    pub fn insert_protocol(&mut self, key: String, protocol: StatefulProtocol) {
        self.cache.insert(key.clone(), protocol);
        self.update_lru(&key);
        self.enforce_cache_capacity();
    }

    /// Set current image to display (must be in cache)
    pub fn set_image(&mut self, rowid: &str) {
        if self.cache.contains_key(rowid) {
            self.current_rowid = Some(rowid.to_string());
            self.update_lru(rowid);
            self.update_display_state(DisplayState::Image(rowid.to_string()));
        }
    }

    /// Update LRU order for a rowid
    fn update_lru(&mut self, rowid: &str) {
        if let Some(pos) = self.cache_order.iter().position(|r| r == rowid) {
            self.cache_order.remove(pos);
        }
        self.cache_order.push_back(rowid.to_string());
    }

    /// Update the global display state (sync version)
    fn update_display_state(&self, state: DisplayState) {
        let mut lock = DISPLAY_STATE.lock().unwrap_or_else(|e| e.into_inner());
        *lock = state;
    }

    /// Load image data from cclip and prepare it for rendering
    pub async fn load_cclip_image(&mut self, rowid: &str) -> Result<()> {
        // Check cache first
        if self.cache.contains_key(rowid) {
            self.update_lru(rowid);
            return Ok(());
        }

        // Run cclip get to fetch image bytes using tokio
        let child = tokio::process::Command::new("cclip")
            .args(["get", rowid])
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()?;

        const CCLIP_GET_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(5);
        let output = match tokio::time::timeout(CCLIP_GET_TIMEOUT, child.wait_with_output()).await {
            Ok(Ok(out)) => out,
            Ok(Err(e)) => return Err(eyre!("cclip get io error for rowid {}: {}", rowid, e)),
            Err(_) => {
                // Timed out: the future is dropped and the Child inside it is dropped,
                // which terminates the cclip process.
                return Err(eyre!(
                    "cclip get timed out after {:?} for rowid: {}",
                    CCLIP_GET_TIMEOUT,
                    rowid
                ));
            }
        };
        if !output.status.success() {
            return Err(eyre!("cclip get failed for rowid: {}", rowid));
        }

        let bytes = output.stdout;
        if bytes.is_empty() {
            return Err(eyre!("No data received from cclip get {}", rowid));
        }

        self.load_image_bytes(rowid, bytes).await?;

        Ok(())
    }

    /// Render the current image into the given area
    pub fn render(&mut self, f: &mut Frame, area: Rect) -> Result<()> {
        if let Some(rowid) = &self.current_rowid
            && let Some(protocol) = self.cache.get_mut(rowid)
        {
            f.render_stateful_widget(
                StatefulImage::default().resize(Resize::Fit(None)),
                area,
                protocol,
            );

            // Propagate encoding/resize errors
            if let Some(Err(e)) = protocol.last_encoding_result() {
                return Err(eyre!("Image encoding failed: {}", e));
            }
        }
        Ok(())
    }

    /// Render a cached image without changing the manager's current selection.
    pub fn render_cached(&mut self, f: &mut Frame, key: &str, area: Rect) -> Result<bool> {
        let encoding_failed = {
            let Some(protocol) = self.cache.get_mut(key) else {
                return Ok(false);
            };

            f.render_stateful_widget(
                StatefulImage::default().resize(Resize::Fit(None)),
                area,
                protocol,
            );
            protocol
                .last_encoding_result()
                .is_some_and(|result| result.is_err())
        };
        if encoding_failed {
            self.cache.remove(key);
            self.cache_order.retain(|cached| cached != key);
            return Ok(false);
        }

        Ok(true)
    }

    pub fn clear(&mut self) {
        self.current_rowid = None;
        self.cache.clear();
        self.cache_order.clear();
        self.update_display_state(DisplayState::Empty);
    }

    fn enforce_cache_capacity(&mut self) {
        if self.cache_order.len() > self.cache_capacity
            && let Some(old_key) = self.cache_order.pop_front()
        {
            self.cache.remove(&old_key);
        }
    }
}

fn decode_image(bytes: &[u8]) -> Result<image::DynamicImage> {
    match image::load_from_memory(bytes) {
        Ok(image) => Ok(image),
        Err(raster_error) if looks_like_svg(bytes) => decode_svg(bytes)
            .map_err(|svg_error| eyre!("Image decode failed: {raster_error}; {svg_error}")),
        Err(error) => Err(error.into()),
    }
}

fn looks_like_svg(bytes: &[u8]) -> bool {
    if bytes.starts_with(&[0x1f, 0x8b]) {
        let mut prefix = Vec::with_capacity(MAX_SVG_PROBE_BYTES as usize);
        flate2::read::GzDecoder::new(bytes)
            .take(MAX_SVG_PROBE_BYTES)
            .read_to_end(&mut prefix)
            .is_ok()
            && has_svg_document_root(&prefix)
    } else {
        has_svg_document_root(bytes)
    }
}

fn decode_svg(bytes: &[u8]) -> Result<image::DynamicImage> {
    let document = bounded_svg_document(bytes, MAX_SVG_DOCUMENT_BYTES)?;
    if !has_svg_document_root(&document) {
        return Err(eyre!("Input is not an SVG document"));
    }
    let tree = resvg::usvg::Tree::from_data(&document, &svg_options(svg_needs_fonts(&document)))?;
    let source_size = tree.size();
    let scale = (MAX_SVG_DIMENSION / source_size.width())
        .min(MAX_SVG_DIMENSION / source_size.height())
        .min(1.0);
    let width = (source_size.width() * scale).round().max(1.0) as u32;
    let height = (source_size.height() * scale).round().max(1.0) as u32;
    let mut pixmap = resvg::tiny_skia::Pixmap::new(width, height)
        .ok_or_else(|| eyre!("Failed to allocate SVG raster buffer"))?;
    resvg::render(
        &tree,
        resvg::tiny_skia::Transform::from_scale(scale, scale),
        &mut pixmap.as_mut(),
    );
    let mut rgba_bytes = pixmap.take();
    unpremultiply_rgba(&mut rgba_bytes);
    let rgba = image::RgbaImage::from_raw(width, height, rgba_bytes)
        .ok_or_else(|| eyre!("Failed to convert SVG raster buffer"))?;
    Ok(image::DynamicImage::ImageRgba8(rgba))
}

fn bounded_svg_document(bytes: &[u8], limit: usize) -> Result<Vec<u8>> {
    if bytes.starts_with(&[0x1f, 0x8b]) {
        let decoder = flate2::read::GzDecoder::new(bytes);
        let mut document = Vec::with_capacity(bytes.len().saturating_mul(2).min(limit));
        decoder.take(limit as u64 + 1).read_to_end(&mut document)?;
        if document.len() > limit {
            return Err(eyre!("Decompressed SVG exceeds {limit} bytes"));
        }
        Ok(document)
    } else if bytes.len() > limit {
        Err(eyre!("SVG exceeds {limit} bytes"))
    } else {
        Ok(bytes.to_vec())
    }
}

fn has_svg_document_root(bytes: &[u8]) -> bool {
    let Ok(mut text) = std::str::from_utf8(bytes) else {
        return false;
    };
    text = text.strip_prefix('\u{feff}').unwrap_or(text);

    loop {
        text = text.trim_start();
        if text.starts_with("<?") {
            let Some(end) = text.find("?>") else {
                return false;
            };
            text = &text[end + 2..];
        } else if text.starts_with("<!--") {
            let Some(end) = text.find("-->") else {
                return false;
            };
            text = &text[end + 3..];
        } else if text.starts_with("<!DOCTYPE") {
            let Some(rest) = skip_doctype(text) else {
                return false;
            };
            text = rest;
        } else {
            break;
        }
    }

    let Some(after_open) = text.strip_prefix('<') else {
        return false;
    };
    let name_end = after_open
        .char_indices()
        .find_map(|(index, character)| {
            (character.is_whitespace() || matches!(character, '>' | '/')).then_some(index)
        })
        .unwrap_or(after_open.len());
    let qualified_name = &after_open[..name_end];
    let (prefix, local_name) = qualified_name
        .split_once(':')
        .map_or((None, qualified_name), |(prefix, local)| {
            (Some(prefix), local)
        });
    if local_name != "svg" || prefix.is_some_and(str::is_empty) {
        return false;
    }

    svg_root_namespace_matches(&after_open[name_end..], prefix)
}

fn skip_doctype(text: &str) -> Option<&str> {
    let body = &text["<!DOCTYPE".len()..];
    let mut quote = None;
    let mut subset_depth = 0_u32;
    let mut offset = 0;
    while offset < body.len() {
        let remaining = &body[offset..];
        let character = remaining.chars().next()?;
        if let Some(expected) = quote {
            if character == expected {
                quote = None;
            }
            offset += character.len_utf8();
            continue;
        }
        if remaining.starts_with("<!--") {
            let end = remaining.find("-->")?;
            offset += end + "-->".len();
            continue;
        }
        if remaining.starts_with("<?") {
            let end = remaining.find("?>")?;
            offset += end + "?>".len();
            continue;
        }
        match character {
            '\'' | '"' => quote = Some(character),
            '[' => subset_depth = subset_depth.saturating_add(1),
            ']' => subset_depth = subset_depth.saturating_sub(1),
            '>' if subset_depth == 0 => {
                let end = "<!DOCTYPE".len() + offset + character.len_utf8();
                return Some(&text[end..]);
            }
            _ => {}
        }
        offset += character.len_utf8();
    }
    None
}

fn svg_root_namespace_matches(mut attributes: &str, prefix: Option<&str>) -> bool {
    let mut namespace = None;

    loop {
        attributes = attributes.trim_start();
        if attributes.starts_with('>') || attributes.starts_with("/>") {
            break;
        }
        let name_end = attributes
            .char_indices()
            .find_map(|(index, character)| {
                (character.is_whitespace() || matches!(character, '=' | '/' | '>')).then_some(index)
            })
            .unwrap_or(attributes.len());
        if name_end == 0 {
            return false;
        }
        let name = &attributes[..name_end];
        attributes = attributes[name_end..].trim_start();
        let Some(after_equals) = attributes.strip_prefix('=') else {
            return false;
        };
        attributes = after_equals.trim_start();
        let Some(delimiter) = attributes
            .chars()
            .next()
            .filter(|value| matches!(value, '\'' | '"'))
        else {
            return false;
        };
        attributes = &attributes[delimiter.len_utf8()..];
        let Some(value_end) = attributes.find(delimiter) else {
            return false;
        };
        let value = &attributes[..value_end];
        let is_namespace = match prefix {
            Some(prefix) => name.strip_prefix("xmlns:") == Some(prefix),
            None => name == "xmlns",
        };
        if is_namespace {
            let Some(value) = xml_attribute_value(value) else {
                return false;
            };
            namespace = Some(value);
        }
        attributes = &attributes[value_end + delimiter.len_utf8()..];
    }

    match prefix {
        Some(_) => namespace.as_deref() == Some(SVG_NAMESPACE),
        None => namespace.is_none() || namespace.as_deref() == Some(SVG_NAMESPACE),
    }
}

fn xml_attribute_value(value: &str) -> Option<String> {
    let mut decoded = String::with_capacity(value.len());
    let mut rest = value;
    while let Some(start) = rest.find('&') {
        decoded.push_str(&rest[..start]);
        rest = &rest[start + 1..];
        let end = rest.find(';')?;
        let reference = &rest[..end];
        let character = match reference {
            "amp" => '&',
            "lt" => '<',
            "gt" => '>',
            "apos" => '\'',
            "quot" => '"',
            value if value.starts_with("#x") => {
                char::from_u32(u32::from_str_radix(&value[2..], 16).ok()?)?
            }
            value if value.starts_with('#') => char::from_u32(value[1..].parse().ok()?)?,
            _ => return None,
        };
        decoded.push(character);
        rest = &rest[end + 1..];
    }
    decoded.push_str(rest);
    Some(decoded)
}

fn svg_needs_fonts(document: &[u8]) -> bool {
    let Ok(document) = std::str::from_utf8(document) else {
        return true;
    };
    let options = roxmltree::ParsingOptions {
        allow_dtd: true,
        ..roxmltree::ParsingOptions::default()
    };
    roxmltree::Document::parse_with_options(document, options).map_or(true, |document| {
        document.descendants().any(|node| {
            node.is_element() && {
                let name = node.tag_name();
                name.name() == "text"
                    && (name.namespace().is_none() || name.namespace() == Some(SVG_NAMESPACE))
            }
        })
    })
}

fn svg_options(load_system_fonts: bool) -> resvg::usvg::Options<'static> {
    use resvg::usvg::{ImageHrefResolver, ImageKind};

    let fontdb = if load_system_fonts {
        Arc::clone(SVG_FONT_DATABASE.get_or_init(|| {
            let mut database = resvg::usvg::fontdb::Database::new();
            database.load_system_fonts();
            Arc::new(database)
        }))
    } else {
        Arc::new(resvg::usvg::fontdb::Database::new())
    };
    let remaining_raster_pixels = Arc::new(AtomicU64::new(MAX_SVG_EMBEDDED_RASTER_PIXELS));
    resvg::usvg::Options {
        fontdb,
        image_href_resolver: ImageHrefResolver {
            resolve_data: Box::new(move |mime, data, _| {
                let expected_format = match mime {
                    "image/jpg" | "image/jpeg" => image::ImageFormat::Jpeg,
                    "image/png" => image::ImageFormat::Png,
                    "image/webp" => image::ImageFormat::WebP,
                    _ => return None,
                };
                let reader = image::ImageReader::new(Cursor::new(data.as_slice()))
                    .with_guessed_format()
                    .ok()?;
                if reader.format() != Some(expected_format) {
                    return None;
                }
                let (width, height) = reader.into_dimensions().ok()?;
                if width > MAX_SVG_EMBEDDED_RASTER_DIMENSION
                    || height > MAX_SVG_EMBEDDED_RASTER_DIMENSION
                {
                    return None;
                }
                let pixels = u64::from(width).checked_mul(u64::from(height))?;
                remaining_raster_pixels
                    .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |remaining| {
                        remaining.checked_sub(pixels)
                    })
                    .ok()?;

                match expected_format {
                    image::ImageFormat::Jpeg => Some(ImageKind::JPEG(data)),
                    image::ImageFormat::Png => Some(ImageKind::PNG(data)),
                    image::ImageFormat::WebP => Some(ImageKind::WEBP(data)),
                    _ => None,
                }
            }),
            resolve_string: Box::new(|_, _| None),
        },
        ..resvg::usvg::Options::default()
    }
}

fn unpremultiply_rgba(bytes: &mut [u8]) {
    let (pixels, _) = bytes.as_chunks_mut::<4>();
    for pixel in pixels {
        let alpha = u16::from(pixel[3]);
        if alpha == 0 || alpha == 255 {
            continue;
        }
        for channel in &mut pixel[..3] {
            *channel = ((u16::from(*channel) * 255 + alpha / 2) / alpha).min(255) as u8;
        }
    }
}

/// Legacy GraphicsAdapter enum to minimize breakage in matches
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum GraphicsAdapter {
    Kitty,
    Sixel,
    None,
}

impl GraphicsAdapter {
    /// Detect the best graphics adapter (legacy)
    pub fn detect(picker: Option<&Picker>) -> Self {
        if let Some(picker) = picker {
            match picker.protocol_type() {
                ProtocolType::Kitty | ProtocolType::Iterm2 => return Self::Kitty,
                ProtocolType::Sixel => return Self::Sixel,
                ProtocolType::Halfblocks => return Self::None,
            }
        }

        let term = std::env::var("TERM").unwrap_or_default();
        let term_program = std::env::var("TERM_PROGRAM").unwrap_or_default();

        if term_program == "kitty" || term.contains("kitty") {
            Self::Kitty
        } else if term.starts_with("foot")
            || term_program == "WezTerm"
            || term.contains("sixel")
            || term.contains("mlterm")
        {
            Self::Sixel
        } else {
            Self::None
        }
    }

    /// Build a picker configured for this detected adapter.
    pub fn picker(self) -> Picker {
        let mut picker = Picker::halfblocks();
        match self {
            Self::Kitty => picker.set_protocol_type(ProtocolType::Kitty),
            Self::Sixel => picker.set_protocol_type(ProtocolType::Sixel),
            Self::None => {}
        }
        picker
    }
}

#[cfg(test)]
mod tests {
    use super::{
        ImageManager, bounded_svg_document, decode_image, has_svg_document_root, looks_like_svg,
        svg_needs_fonts, svg_options, unpremultiply_rgba,
    };
    use flate2::Compression;
    use flate2::write::GzEncoder;
    use std::io::{Cursor, Write};
    use std::sync::Arc;

    #[test]
    fn decodes_svg_bytes_into_rgba_image() {
        let svg = br##"<svg xmlns="http://www.w3.org/2000/svg" width="16" height="8">
            <rect width="16" height="8" fill="#ff0000"/>
        </svg>"##;

        let image = decode_image(svg).expect("SVG should decode");

        assert_eq!(image.width(), 16);
        assert_eq!(image.height(), 8);
    }

    #[test]
    fn font_loading_is_reserved_for_svg_text_elements() {
        assert!(!svg_needs_fonts(
            br#"<svg><path aria-label="text" d="M0 0"/></svg>"#
        ));
        assert!(!svg_needs_fonts(
            br#"<!DOCTYPE svg [<!-- <text> -->]><svg><!-- <text> --><![CDATA[<text>]]><?audit <text>?></svg>"#
        ));
        assert!(svg_needs_fonts(br#"<svg><text>label</text></svg>"#));
        assert!(svg_needs_fonts(
            br#"<svg:svg xmlns:svg="http://www.w3.org/2000/svg"><svg:text>label</svg:text></svg:svg>"#
        ));
        assert!(svg_needs_fonts(
            br#"<!DOCTYPE svg [<!ENTITY label "<text>label</text>">]><svg>&label;</svg>"#
        ));
    }

    #[test]
    fn decodes_gzip_compressed_svg_bytes() {
        let svgz = [
            0x1f, 0x8b, 0x08, 0x00, 0x00, 0x00, 0x00, 0x00, 0x02, 0xff, 0xb3, 0x29, 0x2e, 0x4b,
            0x57, 0xa8, 0xc8, 0xcd, 0xc9, 0x2b, 0xb6, 0x55, 0xca, 0x28, 0x29, 0x29, 0xb0, 0xd2,
            0xd7, 0x2f, 0x2f, 0x2f, 0xd7, 0x2b, 0x37, 0xd6, 0xcb, 0x2f, 0x4a, 0xd7, 0x37, 0x32,
            0x30, 0x30, 0xd0, 0x07, 0xaa, 0x50, 0x52, 0x28, 0xcf, 0x4c, 0x29, 0xc9, 0xb0, 0x55,
            0x32, 0x52, 0x52, 0xc8, 0x48, 0xcd, 0x4c, 0xcf, 0x28, 0xb1, 0x55, 0x32, 0x54, 0xb2,
            0xb3, 0x29, 0x4a, 0x4d, 0x2e, 0xc1, 0x2a, 0xa5, 0x6f, 0x67, 0x03, 0xd2, 0x67, 0x07,
            0x00, 0xf9, 0x0f, 0x22, 0x19, 0x5f, 0x00, 0x00, 0x00,
        ];

        let image = decode_image(&svgz).expect("SVGZ should decode");

        assert_eq!(image.width(), 2);
        assert_eq!(image.height(), 1);
    }

    #[test]
    fn svg_probe_requires_the_document_root() {
        assert!(!looks_like_svg(b"Markdown with <svg>embedded</svg> later"));
        assert!(has_svg_document_root(
            b"\xef\xbb\xbf <?xml version='1.0'?><!-- icon --><svg width='1' height='1'/>"
        ));
    }

    #[test]
    fn svg_probe_skips_complete_doctype_declarations() {
        assert!(has_svg_document_root(
            br#"<!DOCTYPE svg [<!ENTITY color 'red'>]><svg fill='&color;'/>"#
        ));
        assert!(has_svg_document_root(
            br#"<!DOCTYPE svg SYSTEM "x>y"><svg/>"#
        ));
        assert!(has_svg_document_root(
            br#"<!DOCTYPE svg [<!-- ] must not close the subset --><?audit ]?><!ENTITY color 'red'>]><svg/>"#
        ));
    }

    #[test]
    fn svg_probe_accepts_namespace_prefixed_roots() {
        assert!(has_svg_document_root(
            br#"<s:svg xmlns:s="http://www.w3.org/2000/svg" width="1"/>"#
        ));
        assert!(has_svg_document_root(
            br#"<s:svg xmlns:s="http:&#x2f;&#47;www.w3.org/2000/svg" width="1"/>"#
        ));
        assert!(!has_svg_document_root(
            br#"<s:svg xmlns:s="https://example.com/not-svg"/>"#
        ));
    }

    #[test]
    fn decodes_prefixed_and_doctype_svg_documents() {
        let prefixed = decode_image(
            br#"<s:svg xmlns:s="http://www.w3.org/2000/svg" width="1" height="1"><s:rect width="1" height="1"/></s:svg>"#,
        )
        .expect("prefixed SVG should decode");
        let escaped_namespace = decode_image(
            br#"<s:svg xmlns:s="http:&#x2f;&#47;www.w3.org/2000/svg" width="1" height="1"><s:rect width="1" height="1"/></s:svg>"#,
        )
        .expect("prefixed SVG with character references should decode");
        let with_doctype = decode_image(
            br#"<!DOCTYPE svg [<!ENTITY color 'red'>]><svg xmlns="http://www.w3.org/2000/svg" width="1" height="1"><rect width="1" height="1" fill="&color;"/></svg>"#,
        )
        .expect("SVG with an internal DTD subset should decode");

        assert_eq!((prefixed.width(), prefixed.height()), (1, 1));
        assert_eq!(
            (escaped_namespace.width(), escaped_namespace.height()),
            (1, 1)
        );
        assert_eq!((with_doctype.width(), with_doctype.height()), (1, 1));
    }

    #[test]
    fn svgz_decompression_respects_its_limit() {
        let mut encoder = GzEncoder::new(Vec::new(), Compression::best());
        encoder
            .write_all(b"<svg><!-- content beyond the configured limit --></svg>")
            .expect("compressed SVG should be written");
        let compressed = encoder.finish().expect("compressed SVG should finish");

        assert!(bounded_svg_document(&compressed, 16).is_err());
    }

    #[test]
    fn gzip_probe_rejects_non_svg_payloads() {
        let mut encoder = GzEncoder::new(Vec::new(), Compression::fast());
        encoder
            .write_all(b"ordinary compressed output")
            .expect("gzip payload should be written");
        let compressed = encoder.finish().expect("gzip payload should finish");

        assert!(!ImageManager::recognizes_image_bytes(&compressed));
    }

    #[test]
    fn embedded_rasters_share_a_decoded_pixel_budget() {
        let image = image::DynamicImage::new_rgba8(2048, 1024);
        let mut encoded = Cursor::new(Vec::new());
        image
            .write_to(&mut encoded, image::ImageFormat::Png)
            .expect("test PNG should encode");
        let data = Arc::new(encoded.into_inner());
        let options = svg_options(false);
        let resolve = &options.image_href_resolver.resolve_data;

        assert!(resolve("image/png", Arc::clone(&data), &options).is_some());
        assert!(resolve("image/png", Arc::clone(&data), &options).is_some());
        assert!(resolve("image/png", data, &options).is_none());
    }

    #[test]
    fn svg_options_reject_external_image_paths() {
        let options = svg_options(false);

        assert!((options.image_href_resolver.resolve_string)("/dev/zero", &options).is_none());
    }

    #[test]
    fn svg_options_reject_embedded_gifs() {
        let options = svg_options(false);
        let gif = Arc::new(b"GIF89a".to_vec());

        assert!((options.image_href_resolver.resolve_data)("image/gif", gif, &options).is_none());
    }

    #[test]
    fn unpremultiply_restores_translucent_channels() {
        let mut pixel = [64, 32, 16, 128];

        unpremultiply_rgba(&mut pixel);

        assert_eq!(pixel, [128, 64, 32, 128]);
    }
}
