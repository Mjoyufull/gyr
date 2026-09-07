use eyre::{Result, eyre};
use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui_image::picker::{Picker, ProtocolType};
use ratatui_image::protocol::{Protocol, StatefulProtocol};
use ratatui_image::{Image, Resize, StatefulImage};
use std::collections::{HashMap, VecDeque};
use std::io::{Cursor, Read};
use std::path::Path;
use std::process::Stdio;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, OnceLock};

const MAX_SVG_DIMENSION: f32 = 2048.0;
const MAX_SVG_DOCUMENT_BYTES: usize = 32 * 1024 * 1024;
const MAX_SVG_PROBE_BYTES: u64 = 64 * 1024;
const MAX_SVG_EMBEDDED_RASTER_DIMENSION: u32 = 4096;
const MAX_SVG_EMBEDDED_RASTER_PIXELS: u64 = 2048 * 2048;
const SVG_NAMESPACE: &str = "http://www.w3.org/2000/svg";
const MAX_IMAGE_FILE_BYTES: u64 = 32 * 1024 * 1024;
const MAX_ICON_RASTER_DIMENSION: u32 = 4096;
const MAX_ICON_DECODED_BYTES: u64 = 64 * 1024 * 1024;
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
    cache: HashMap<String, CachedProtocol>,
    cache_weights: HashMap<String, u64>,
    cache_order: VecDeque<String>,
    cache_capacity: usize,
    cache_weight: u64,
    cache_weight_capacity: u64,
}

enum CachedProtocol {
    Fixed(Protocol),
    Resizable(StatefulProtocol),
}

pub(crate) struct PreparedFixedImage {
    pub(crate) protocol: Protocol,
    pub(crate) retained_bytes: u64,
    pub(crate) top_overflow_rows: u16,
}

impl ImageManager {
    /// Initialize the image manager with the picker chosen by the caller.
    pub fn new(picker: Picker) -> Self {
        Self {
            picker,
            current_rowid: None,
            cache: HashMap::new(),
            cache_weights: HashMap::new(),
            cache_order: VecDeque::new(),
            cache_capacity: 50,
            cache_weight: 0,
            cache_weight_capacity: u64::MAX,
        }
    }

    /// Limit the retained decoded image data while preserving the entry-count bound.
    pub(crate) fn with_cache_weight_limit(mut self, capacity: u64) -> Self {
        self.cache_weight_capacity = capacity;
        self.enforce_cache_capacity();
        self
    }

    /// Check if an image is already in cache
    pub fn is_cached(&self, rowid: &str) -> bool {
        self.cache.contains_key(rowid)
    }

    /// Grow the cache when a caller needs a larger working set.
    pub(crate) fn ensure_cache_capacity(&mut self, minimum: usize) {
        self.cache_capacity = self.cache_capacity.max(minimum);
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

    /// Read and prepare an image file from a blocking worker.
    #[cfg(test)]
    pub(crate) fn prepare_image_path(picker: Picker, path: &Path) -> Result<StatefulProtocol> {
        let (image, _) = read_icon_image(path, MAX_SVG_DIMENSION as u32)?;
        Ok(picker.new_resize_protocol(image))
    }

    /// Read, resize, and encode an icon for a fixed terminal-cell area.
    pub(crate) fn prepare_fixed_image_path_with_weight(
        picker: Picker,
        path: &Path,
        area: Rect,
        horizontal_align: u16,
        vertical_align: i16,
    ) -> Result<PreparedFixedImage> {
        let (font_width, font_height) = picker.font_size();
        let pixel_width = u32::from(area.width).saturating_mul(u32::from(font_width));
        let pixel_height = u32::from(area.height).saturating_mul(u32::from(font_height));
        let svg_dimension = pixel_width.max(pixel_height).max(1);
        let (image, _) = read_icon_image(path, svg_dimension)?;
        let normalized = normalize_desktop_icon(
            image,
            pixel_width,
            pixel_height,
            horizontal_align,
            vertical_align,
            font_height,
        );
        let protocol_height = area.height.saturating_add(normalized.top_overflow_rows);
        let protocol_area = Rect::new(area.x, area.y, area.width, protocol_height);
        let retained_bytes = u64::from(area.width)
            .saturating_mul(u64::from(font_width))
            .saturating_mul(u64::from(protocol_height))
            .saturating_mul(u64::from(font_height))
            .saturating_mul(4);
        let protocol = picker.new_protocol(normalized.image, protocol_area, Resize::Scale(None))?;
        Ok(PreparedFixedImage {
            protocol,
            retained_bytes,
            top_overflow_rows: normalized.top_overflow_rows,
        })
    }

    /// Insert a fixed-size protocol prepared by a desktop-icon worker.
    pub(crate) fn insert_fixed_protocol_with_weight(
        &mut self,
        key: String,
        protocol: Protocol,
        weight: u64,
    ) {
        self.insert_cached_protocol(key, CachedProtocol::Fixed(protocol), weight);
    }

    fn insert_cached_protocol(&mut self, key: String, protocol: CachedProtocol, weight: u64) {
        if let Some(previous) = self.cache_weights.insert(key.clone(), weight) {
            self.cache_weight = self.cache_weight.saturating_sub(previous);
        }
        self.cache_weight = self.cache_weight.saturating_add(weight);
        self.cache.insert(key.clone(), protocol);
        self.update_lru(&key);
        self.enforce_cache_capacity();
    }

    /// Return whether bytes have a supported raster signature or contain SVG markup.
    pub fn recognizes_image_bytes(bytes: &[u8]) -> bool {
        image::guess_format(bytes).is_ok() || looks_like_svg(bytes)
    }

    /// Insert a prepared terminal image protocol into the bounded cache.
    pub fn insert_protocol(&mut self, key: String, protocol: StatefulProtocol) {
        self.insert_protocol_with_weight(key, protocol, 0);
    }

    /// Insert a protocol and account for the decoded image bytes it retains.
    pub(crate) fn insert_protocol_with_weight(
        &mut self,
        key: String,
        protocol: StatefulProtocol,
        weight: u64,
    ) {
        self.insert_cached_protocol(key, CachedProtocol::Resizable(protocol), weight);
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
    pub fn render(&mut self, f: &mut Frame, area: Rect) -> Result<bool> {
        let Some(rowid) = &self.current_rowid else {
            return Ok(false);
        };
        let Some(protocol) = self.cache.get_mut(rowid) else {
            return Ok(false);
        };
        match protocol {
            CachedProtocol::Fixed(protocol) => f.render_widget(Image::new(protocol), area),
            CachedProtocol::Resizable(protocol) => {
                f.render_stateful_widget(
                    StatefulImage::default().resize(Resize::Fit(None)),
                    area,
                    protocol,
                );
                if let Some(Err(error)) = protocol.last_encoding_result() {
                    return Err(eyre!("Image encoding failed: {error}"));
                }
            }
        }
        Ok(true)
    }

    /// Render a cached image without changing the manager's current selection.
    pub fn render_cached(&mut self, f: &mut Frame, key: &str, area: Rect) -> Result<bool> {
        if !self.cache.contains_key(key) {
            return Ok(false);
        }
        self.update_lru(key);
        let encoding_failed = {
            let Some(protocol) = self.cache.get_mut(key) else {
                return Ok(false);
            };
            match protocol {
                CachedProtocol::Fixed(protocol) => {
                    f.render_widget(Image::new(protocol), area);
                    false
                }
                CachedProtocol::Resizable(protocol) => {
                    f.render_stateful_widget(
                        StatefulImage::default().resize(Resize::Fit(None)),
                        area,
                        protocol,
                    );
                    protocol
                        .last_encoding_result()
                        .is_some_and(|result| result.is_err())
                }
            }
        };
        if encoding_failed {
            self.remove_cached(key);
            return Ok(false);
        }

        Ok(true)
    }

    pub fn clear(&mut self) {
        self.current_rowid = None;
        self.cache.clear();
        self.cache_weights.clear();
        self.cache_order.clear();
        self.cache_weight = 0;
        self.update_display_state(DisplayState::Empty);
    }

    fn enforce_cache_capacity(&mut self) {
        while self.cache_order.len() > self.cache_capacity
            || self.cache_weight > self.cache_weight_capacity
        {
            let Some(old_key) = self.cache_order.front().cloned() else {
                break;
            };
            self.remove_cached(&old_key);
        }
    }

    fn remove_cached(&mut self, key: &str) {
        self.cache.remove(key);
        if let Some(weight) = self.cache_weights.remove(key) {
            self.cache_weight = self.cache_weight.saturating_sub(weight);
        }
        self.cache_order.retain(|cached| cached != key);
    }
}

struct NormalizedDesktopIcon {
    image: image::DynamicImage,
    top_overflow_rows: u16,
}

fn normalize_desktop_icon(
    image: image::DynamicImage,
    target_width: u32,
    target_height: u32,
    horizontal_align: u16,
    vertical_align: i16,
    font_height: u16,
) -> NormalizedDesktopIcon {
    let source = image.into_rgba8();
    let mut bounds = None::<(u32, u32, u32, u32)>;
    for (x, y, pixel) in source.enumerate_pixels() {
        if pixel[3] <= 8 {
            continue;
        }
        bounds = Some(bounds.map_or((x, y, x, y), |(left, top, right, bottom)| {
            (left.min(x), top.min(y), right.max(x), bottom.max(y))
        }));
    }

    let Some((left, top, right, bottom)) = bounds else {
        return NormalizedDesktopIcon {
            image: image::DynamicImage::new_rgba8(target_width.max(1), target_height.max(1)),
            top_overflow_rows: 0,
        };
    };
    let source_width = right - left + 1;
    let source_height = bottom - top + 1;
    let cropped =
        image::imageops::crop_imm(&source, left, top, source_width, source_height).to_image();

    // A small common inset makes differently padded source files occupy the same visual box.
    let max_width = target_width.saturating_mul(88).div_ceil(100).max(1);
    let max_height = target_height.saturating_mul(88).div_ceil(100).max(1);
    let (width, height) = if u64::from(max_width) * u64::from(source_height)
        <= u64::from(max_height) * u64::from(source_width)
    {
        let height = u64::from(source_height)
            .saturating_mul(u64::from(max_width))
            .div_ceil(u64::from(source_width)) as u32;
        (max_width, height.max(1))
    } else {
        let width = u64::from(source_width)
            .saturating_mul(u64::from(max_height))
            .div_ceil(u64::from(source_height)) as u32;
        (width.max(1), max_height)
    };
    let resized = image::imageops::resize(
        &cropped,
        width,
        height,
        image::imageops::FilterType::CatmullRom,
    );
    let x = target_width
        .saturating_sub(width)
        .saturating_mul(u32::from(horizontal_align.min(100)))
        / 100;
    let y = i64::from(target_height.saturating_sub(height))
        .saturating_mul(i64::from(vertical_align.clamp(-100, 100)))
        / 100;
    let font_height = u32::from(font_height.max(1));
    let top_overflow_pixels = y.unsigned_abs().min(u64::from(u32::MAX)) as u32;
    let top_overflow_rows = if y < 0 {
        top_overflow_pixels
            .div_ceil(font_height)
            .min(u32::from(u16::MAX)) as u16
    } else {
        0
    };
    let top_padding = u32::from(top_overflow_rows).saturating_mul(font_height);
    let canvas_height = target_height.saturating_add(top_padding).max(1);
    let mut canvas = image::RgbaImage::new(target_width.max(1), canvas_height);
    let canvas_y = i64::from(top_padding).saturating_add(y);
    image::imageops::overlay(&mut canvas, &resized, i64::from(x), canvas_y);
    NormalizedDesktopIcon {
        image: image::DynamicImage::ImageRgba8(canvas),
        top_overflow_rows,
    }
}

fn read_icon_image(path: &Path, svg_dimension: u32) -> Result<(image::DynamicImage, u64)> {
    let file = std::fs::File::open(path)?;
    let file_size = file.metadata()?.len();
    if file_size > MAX_IMAGE_FILE_BYTES {
        return Err(eyre!("Image file exceeds {} bytes", MAX_IMAGE_FILE_BYTES));
    }
    let mut bytes = Vec::with_capacity(file_size as usize);
    file.take(MAX_IMAGE_FILE_BYTES + 1)
        .read_to_end(&mut bytes)?;
    if bytes.len() as u64 > MAX_IMAGE_FILE_BYTES {
        return Err(eyre!("Image file exceeds {} bytes", MAX_IMAGE_FILE_BYTES));
    }
    let image = decode_icon_image_at_size(&bytes, svg_dimension)?;
    let decoded_bytes = image.as_bytes().len() as u64;
    Ok((image, decoded_bytes))
}

fn decode_image(bytes: &[u8]) -> Result<image::DynamicImage> {
    match image::load_from_memory(bytes) {
        Ok(image) => Ok(image),
        Err(raster_error) if looks_like_svg(bytes) => decode_svg(bytes)
            .map_err(|svg_error| eyre!("Image decode failed: {raster_error}; {svg_error}")),
        Err(error) => Err(error.into()),
    }
}

#[cfg(test)]
fn decode_icon_image(bytes: &[u8]) -> Result<image::DynamicImage> {
    decode_icon_image_at_size(bytes, MAX_SVG_DIMENSION as u32)
}

fn decode_icon_image_at_size(bytes: &[u8], svg_dimension: u32) -> Result<image::DynamicImage> {
    if looks_like_svg(bytes) {
        return decode_icon_svg(bytes, svg_dimension);
    }
    if looks_like_xpm(bytes) {
        return decode_xpm(bytes);
    }

    let mut reader = image::ImageReader::new(Cursor::new(bytes)).with_guessed_format()?;
    let mut limits = image::Limits::default();
    limits.max_image_width = Some(MAX_ICON_RASTER_DIMENSION);
    limits.max_image_height = Some(MAX_ICON_RASTER_DIMENSION);
    limits.max_alloc = Some(MAX_ICON_DECODED_BYTES);
    reader.limits(limits);
    Ok(reader.decode()?)
}

fn looks_like_xpm(bytes: &[u8]) -> bool {
    std::str::from_utf8(bytes).is_ok_and(|text| {
        let text = text.trim_start_matches('\u{feff}').trim_start();
        text.starts_with("/* XPM */") || text.starts_with("! XPM2")
    })
}

fn decode_xpm(bytes: &[u8]) -> Result<image::DynamicImage> {
    let text = std::str::from_utf8(bytes).map_err(|error| eyre!("Invalid XPM text: {error}"))?;
    let strings = if text
        .trim_start_matches('\u{feff}')
        .trim_start()
        .starts_with("! XPM2")
    {
        text.lines()
            .skip_while(|line| !line.trim_start().starts_with("! XPM2"))
            .skip(1)
            .map(str::trim_end)
            .filter(|line| !line.trim_start().starts_with('!'))
            .map(str::to_string)
            .collect::<Vec<_>>()
    } else {
        extract_c_strings(text)?
    };
    let header = strings
        .first()
        .ok_or_else(|| eyre!("XPM does not contain a header"))?;
    let mut header_fields = header.split_whitespace();
    let width = parse_xpm_number(header_fields.next(), "width")?;
    let height = parse_xpm_number(header_fields.next(), "height")?;
    let color_count = parse_xpm_number(header_fields.next(), "color count")? as usize;
    let chars_per_pixel = parse_xpm_number(header_fields.next(), "characters per pixel")? as usize;
    if width == 0 || height == 0 || color_count == 0 || chars_per_pixel == 0 {
        return Err(eyre!(
            "XPM dimensions, colors, and characters per pixel must be non-zero"
        ));
    }
    if width > MAX_ICON_RASTER_DIMENSION || height > MAX_ICON_RASTER_DIMENSION {
        return Err(eyre!("XPM dimensions exceed the icon decode limit"));
    }
    let decoded_bytes = u64::from(width)
        .checked_mul(u64::from(height))
        .and_then(|pixels| pixels.checked_mul(4))
        .ok_or_else(|| eyre!("XPM dimensions overflow"))?;
    if decoded_bytes > MAX_ICON_DECODED_BYTES {
        return Err(eyre!("XPM decoded image exceeds the memory limit"));
    }
    let expected_strings = 1usize
        .checked_add(color_count)
        .and_then(|count| count.checked_add(height as usize))
        .ok_or_else(|| eyre!("XPM entry count overflow"))?;
    if strings.len() < expected_strings {
        return Err(eyre!("XPM is missing color or pixel rows"));
    }

    let mut colors = HashMap::<Vec<u8>, [u8; 4]>::with_capacity(color_count);
    for line in &strings[1..=color_count] {
        if line.len() < chars_per_pixel || !line.is_char_boundary(chars_per_pixel) {
            return Err(eyre!("XPM color key is shorter than declared"));
        }
        let key = line.as_bytes()[..chars_per_pixel].to_vec();
        let color = xpm_color_value(&line[chars_per_pixel..])?;
        colors.insert(key, color);
    }

    let row_bytes = (width as usize)
        .checked_mul(chars_per_pixel)
        .ok_or_else(|| eyre!("XPM row width overflow"))?;
    let mut pixels = Vec::with_capacity(decoded_bytes as usize);
    for row in &strings[1 + color_count..expected_strings] {
        if row.len() != row_bytes {
            return Err(eyre!("XPM pixel row has an unexpected width"));
        }
        for key in row.as_bytes().chunks_exact(chars_per_pixel) {
            let color = colors
                .get(key)
                .ok_or_else(|| eyre!("XPM pixel uses an undefined color key"))?;
            pixels.extend_from_slice(color);
        }
    }
    let image = image::RgbaImage::from_raw(width, height, pixels)
        .ok_or_else(|| eyre!("Failed to construct decoded XPM image"))?;
    Ok(image::DynamicImage::ImageRgba8(image))
}

fn parse_xpm_number(value: Option<&str>, field: &str) -> Result<u32> {
    value
        .ok_or_else(|| eyre!("XPM header is missing {field}"))?
        .parse()
        .map_err(|error| eyre!("Invalid XPM {field}: {error}"))
}

fn xpm_color_value(specification: &str) -> Result<[u8; 4]> {
    let fields = specification.split_whitespace().collect::<Vec<_>>();
    let color_index = fields
        .iter()
        .position(|field| field.eq_ignore_ascii_case("c"))
        .ok_or_else(|| eyre!("XPM color entry has no color field"))?;
    let end = fields[color_index + 1..]
        .iter()
        .position(|field| {
            ["s", "m", "g4", "g", "c"]
                .iter()
                .any(|name| field.eq_ignore_ascii_case(name))
        })
        .map_or(fields.len(), |offset| color_index + 1 + offset);
    let value = fields[color_index + 1..end].join("");
    if value.is_empty() {
        return Err(eyre!("XPM color field is empty"));
    }
    if value.eq_ignore_ascii_case("none") {
        return Ok([0, 0, 0, 0]);
    }
    if let Some(color) = parse_extended_xpm_hex(&value) {
        return Ok(color);
    }
    let lowercase = value.to_ascii_lowercase();
    if let Some(gray) = lowercase
        .strip_prefix("gray")
        .or_else(|| lowercase.strip_prefix("grey"))
        .and_then(|percent| percent.parse::<u8>().ok())
        .filter(|percent| *percent <= 100)
    {
        let channel = ((u16::from(gray) * 255 + 50) / 100) as u8;
        return Ok([channel, channel, channel, 255]);
    }
    let color = value
        .parse::<svgtypes::Color>()
        .map_err(|error| eyre!("Unsupported XPM color {value:?}: {error}"))?;
    Ok([color.red, color.green, color.blue, color.alpha])
}

fn parse_extended_xpm_hex(value: &str) -> Option<[u8; 4]> {
    let hex = value.strip_prefix('#')?;
    let component_width = match hex.len() {
        3 | 6 => return None,
        9 => 3,
        12 => 4,
        _ => return None,
    };
    let mut color = [0_u8; 4];
    color[3] = 255;
    for (index, channel) in color[..3].iter_mut().enumerate() {
        let start = index * component_width;
        *channel = u8::from_str_radix(&hex[start..start + 2], 16).ok()?;
    }
    Some(color)
}

fn extract_c_strings(text: &str) -> Result<Vec<String>> {
    let mut strings = Vec::new();
    let mut characters = text.chars().peekable();
    while let Some(character) = characters.next() {
        if character != '"' {
            continue;
        }
        let mut value = String::new();
        let mut closed = false;
        while let Some(character) = characters.next() {
            match character {
                '"' => {
                    closed = true;
                    break;
                }
                '\\' => {
                    let escaped = characters
                        .next()
                        .ok_or_else(|| eyre!("XPM string ends after an escape"))?;
                    match escaped {
                        '\\' | '"' => value.push(escaped),
                        'n' => value.push('\n'),
                        'r' => value.push('\r'),
                        't' => value.push('\t'),
                        '\n' => {}
                        other => value.push(other),
                    }
                }
                other => value.push(other),
            }
        }
        if !closed {
            return Err(eyre!("XPM contains an unterminated string"));
        }
        strings.push(value);
    }
    Ok(strings)
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
    decode_svg_with_limit(bytes, MAX_SVG_DIMENSION, false)
}

fn decode_icon_svg(bytes: &[u8], dimension: u32) -> Result<image::DynamicImage> {
    let dimension = dimension.clamp(1, MAX_SVG_DIMENSION as u32) as f32;
    decode_svg_with_limit(bytes, dimension, true)
}

fn decode_svg_with_limit(
    bytes: &[u8],
    maximum_dimension: f32,
    allow_upscale: bool,
) -> Result<image::DynamicImage> {
    let document = bounded_svg_document(bytes, MAX_SVG_DOCUMENT_BYTES)?;
    if !has_svg_document_root(&document) {
        return Err(eyre!("Input is not an SVG document"));
    }
    let tree = resvg::usvg::Tree::from_data(&document, &svg_options(svg_needs_fonts(&document)))?;
    let source_size = tree.size();
    let mut scale =
        (maximum_dimension / source_size.width()).min(maximum_dimension / source_size.height());
    if !allow_upscale {
        scale = scale.min(1.0);
    }
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
        ImageManager, MAX_IMAGE_FILE_BYTES, bounded_svg_document, decode_icon_image,
        decode_icon_image_at_size, decode_image, decode_xpm, has_svg_document_root, looks_like_svg,
        normalize_desktop_icon, svg_needs_fonts, svg_options, unpremultiply_rgba,
    };
    use flate2::Compression;
    use flate2::write::GzEncoder;
    use ratatui::Terminal;
    use ratatui::backend::TestBackend;
    use ratatui_image::picker::Picker;
    use std::io::{Cursor, Write};
    use std::sync::Arc;

    fn alpha_bounds(image: &image::DynamicImage) -> (u32, u32, u32, u32) {
        let image = image.to_rgba8();
        let points = image
            .enumerate_pixels()
            .filter(|(_, _, pixel)| pixel[3] > 8)
            .map(|(x, y, _)| (x, y))
            .collect::<Vec<_>>();
        let left = points.iter().map(|(x, _)| *x).min().expect("visible pixel");
        let top = points.iter().map(|(_, y)| *y).min().expect("visible pixel");
        let right = points.iter().map(|(x, _)| *x).max().expect("visible pixel");
        let bottom = points.iter().map(|(_, y)| *y).max().expect("visible pixel");
        (left, top, right, bottom)
    }

    fn visible_pixel_count(image: &image::DynamicImage) -> usize {
        image
            .to_rgba8()
            .pixels()
            .filter(|pixel| pixel[3] > 8)
            .count()
    }

    #[test]
    fn desktop_icons_with_different_source_padding_share_one_visual_box() {
        let mut padded = image::RgbaImage::new(64, 64);
        for pixel in padded.enumerate_pixels_mut() {
            let (x, y, pixel) = pixel;
            if (24..40).contains(&x) && (24..40).contains(&y) {
                *pixel = image::Rgba([255, 255, 255, 255]);
            }
        }
        let full = image::RgbaImage::from_pixel(128, 128, image::Rgba([255, 255, 255, 255]));

        let padded = normalize_desktop_icon(
            image::DynamicImage::ImageRgba8(padded),
            100,
            100,
            50,
            50,
            10,
        );
        let full =
            normalize_desktop_icon(image::DynamicImage::ImageRgba8(full), 100, 100, 50, 50, 10);

        assert_eq!(alpha_bounds(&padded.image), (6, 6, 93, 93));
        assert_eq!(alpha_bounds(&full.image), alpha_bounds(&padded.image));
    }

    #[test]
    fn desktop_icon_alignment_uses_the_available_canvas_space() {
        let source = image::RgbaImage::from_pixel(10, 10, image::Rgba([255, 255, 255, 255]));
        let above = normalize_desktop_icon(
            image::DynamicImage::ImageRgba8(source.clone()),
            100,
            100,
            0,
            -100,
            10,
        );
        let left = normalize_desktop_icon(
            image::DynamicImage::ImageRgba8(source.clone()),
            100,
            100,
            0,
            0,
            10,
        );
        let centered = normalize_desktop_icon(
            image::DynamicImage::ImageRgba8(source.clone()),
            100,
            100,
            50,
            50,
            10,
        );
        let right = normalize_desktop_icon(
            image::DynamicImage::ImageRgba8(source),
            100,
            100,
            100,
            100,
            10,
        );

        assert_eq!(above.top_overflow_rows, 2);
        assert_eq!(alpha_bounds(&above.image), (0, 8, 87, 95));
        assert_eq!(
            visible_pixel_count(&above.image),
            visible_pixel_count(&left.image)
        );
        assert_eq!(left.top_overflow_rows, 0);
        assert_eq!(alpha_bounds(&left.image), (0, 0, 87, 87));
        assert_eq!(centered.top_overflow_rows, 0);
        assert_eq!(alpha_bounds(&centered.image), (6, 6, 93, 93));
        assert_eq!(right.top_overflow_rows, 0);
        assert_eq!(alpha_bounds(&right.image), (12, 12, 99, 99));
    }

    #[test]
    fn weighted_cache_evicts_oldest_decoded_image() {
        let picker = Picker::halfblocks();
        let mut manager = ImageManager::new(picker.clone()).with_cache_weight_limit(4);
        manager.insert_protocol_with_weight(
            "first".to_string(),
            picker
                .clone()
                .new_resize_protocol(image::DynamicImage::new_rgba8(1, 1)),
            4,
        );
        manager.insert_protocol_with_weight(
            "second".to_string(),
            picker.new_resize_protocol(image::DynamicImage::new_rgba8(1, 1)),
            4,
        );

        assert!(!manager.is_cached("first"));
        assert!(manager.is_cached("second"));
        assert_eq!(manager.cache_weight, 4);
    }

    #[test]
    fn render_reports_when_no_cached_image_was_drawn() {
        let mut manager = ImageManager::new(Picker::halfblocks());
        let mut terminal = Terminal::new(TestBackend::new(10, 4)).expect("terminal should open");
        let mut rendered = true;

        terminal
            .draw(|frame| {
                rendered = manager
                    .render(frame, frame.area())
                    .expect("empty image rendering should succeed");
            })
            .expect("terminal should draw");

        assert!(!rendered);
    }

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
    fn desktop_svg_icons_render_at_the_requested_resolution() {
        let svg = br##"<svg xmlns="http://www.w3.org/2000/svg" width="16" height="8">
            <rect width="16" height="8" fill="#ff0000"/>
        </svg>"##;

        let image = decode_icon_image_at_size(svg, 128).expect("SVG icon should decode");

        assert_eq!((image.width(), image.height()), (128, 64));
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
    fn decodes_xpm_icons_with_named_and_transparent_colors() {
        let xpm = br#"/* XPM */
static char *icon[] = {
"2 2 3 1",
". c #ff0000",
"  c None",
"+ c navy",
".+",
" ."
};
"#;

        let image = decode_xpm(xpm).expect("XPM should decode").to_rgba8();

        assert_eq!(image.dimensions(), (2, 2));
        assert_eq!(image.get_pixel(0, 0).0, [255, 0, 0, 255]);
        assert_eq!(image.get_pixel(1, 0).0, [0, 0, 128, 255]);
        assert_eq!(image.get_pixel(0, 1).0, [0, 0, 0, 0]);
        assert_eq!(image.get_pixel(1, 1).0, [255, 0, 0, 255]);
        assert!(decode_icon_image(xpm).is_ok());
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
    fn image_path_read_rejects_oversized_files() {
        let path = std::env::temp_dir().join(format!("fsel-oversized-icon-{}", std::process::id()));
        let file = std::fs::File::create(&path).expect("sparse test file should be created");
        file.set_len(MAX_IMAGE_FILE_BYTES + 1)
            .expect("sparse test file should be sized");

        let error =
            ImageManager::prepare_image_path(ratatui_image::picker::Picker::halfblocks(), &path)
                .err()
                .expect("oversized image should be rejected");

        assert!(error.to_string().contains("exceeds"));
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn icon_decode_rejects_oversized_raster_dimensions() {
        let image = image::DynamicImage::new_rgba8(4097, 1);
        let mut encoded = Cursor::new(Vec::new());
        image
            .write_to(&mut encoded, image::ImageFormat::Png)
            .expect("test PNG should encode");

        assert!(decode_icon_image(encoded.get_ref()).is_err());
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
