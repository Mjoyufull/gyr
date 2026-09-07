use eyre::{Result, eyre};
use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui_image::picker::{Picker, ProtocolType};
use ratatui_image::protocol::StatefulProtocol;
use ratatui_image::{Resize, StatefulImage};
use std::collections::{HashMap, VecDeque};
use std::process::Stdio;
use std::sync::Mutex;

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
        let image = image::load_from_memory(&bytes)?;
        Ok(picker.new_resize_protocol(image))
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
