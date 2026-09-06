//! Shared selector state, filtering, content previews, and cclip fetch coordination.

mod content;
mod filter;
mod tag_mode;

use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::sync::atomic::AtomicUsize;
use std::time::Instant;
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};

use nucleo_matcher::{Config, Matcher};
use ratatui::text::Line;

use crate::common::Item;

pub use tag_mode::TagMode;

type CclipContentResult = (u64, String, Option<String>);

/// Dmenu-specific UI for filtering and sorting.
pub struct DmenuUI<'a> {
    /// Hidden items (they don't match the current query).
    pub hidden: Vec<Item>,
    /// Shown items (they match the current query).
    pub shown: Vec<Item>,
    /// Current selection (index of `self.shown`).
    pub selected: Option<usize>,
    /// Info text for content display.
    pub text: Vec<Line<'a>>,
    /// User query (used for matching).
    pub query: String,
    /// Scroll offset for the list.
    pub scroll_offset: usize,
    /// Whether to wrap long lines in content display.
    pub wrap_long_lines: bool,
    /// Show line numbers.
    pub show_line_numbers: bool,
    /// Match mode (exact or fuzzy).
    pub match_mode: crate::cli::MatchMode,
    /// Match against specific columns.
    pub match_nth: Option<Vec<usize>>,
    /// Tag mode state.
    pub tag_mode: TagMode,
    /// Cache for clipboard content to avoid repeated cclip calls.
    content_cache: HashMap<String, String>,
    /// Clipboard rows currently being fetched.
    content_requests: HashSet<String>,
    /// Clipboard rows whose fetch or decoding failed.
    content_failures: HashMap<String, Instant>,
    /// Completed clipboard fetch sender shared with bounded workers.
    content_sender: UnboundedSender<CclipContentResult>,
    /// Completed clipboard fetches drained by the UI thread.
    content_receiver: UnboundedReceiver<CclipContentResult>,
    /// Fetches still occupying one of this selector's worker slots.
    content_in_flight: Arc<AtomicUsize>,
    /// Invalidates results from an earlier item or verbosity generation.
    content_generation: u64,
    /// Controls raw content and diagnostics in cclip previews.
    cclip_verbosity: u64,
    /// Temporary error/info message with expiration time.
    pub temp_message: Option<(String, Instant)>,
    #[doc(hidden)]
    matcher: Matcher,
}

impl<'a> DmenuUI<'a> {
    /// Creates a new DmenuUI from a `Vec<Item>`.
    pub fn new(items: Vec<Item>, wrap_long_lines: bool, show_line_numbers: bool) -> DmenuUI<'a> {
        let (content_sender, content_receiver) = tokio::sync::mpsc::unbounded_channel();
        let mut ui = DmenuUI {
            shown: vec![],
            hidden: items,
            selected: None,
            text: vec![],
            query: String::new(),
            scroll_offset: 0,
            wrap_long_lines,
            show_line_numbers,
            match_mode: crate::cli::MatchMode::Fuzzy,
            match_nth: None,
            tag_mode: TagMode::Normal,
            content_cache: HashMap::new(),
            content_requests: HashSet::new(),
            content_failures: HashMap::new(),
            content_sender,
            content_receiver,
            content_in_flight: Arc::new(AtomicUsize::new(0)),
            content_generation: 0,
            cclip_verbosity: 0,
            temp_message: None,
            matcher: Matcher::new(Config::DEFAULT.match_paths()),
        };
        ui.filter();
        ui
    }

    /// Set match mode.
    pub fn set_match_mode(&mut self, mode: crate::cli::MatchMode) {
        self.match_mode = mode;
    }

    /// Set match_nth columns.
    pub fn set_match_nth(&mut self, columns: Option<Vec<usize>>) {
        self.match_nth = columns;
    }

    /// Set cclip preview verbosity, clearing content fetched under the previous view.
    pub fn set_cclip_verbosity(&mut self, verbosity: u64) {
        if self.cclip_verbosity != verbosity {
            self.reset_cclip_content();
        }
        self.cclip_verbosity = verbosity;
    }

    /// Return whether a background clipboard-content request can wake the renderer.
    pub(crate) fn has_cclip_content_activity(&self) -> bool {
        !self.content_requests.is_empty()
            || !self.content_receiver.is_empty()
            || self
                .content_in_flight
                .load(std::sync::atomic::Ordering::Acquire)
                > 0
    }

    /// Set a temporary message that expires after 2 seconds.
    pub fn set_temp_message(&mut self, message: String) {
        self.temp_message = Some((message, Instant::now()));
    }

    /// Clear temporary message if expired (2 seconds).
    pub fn clear_expired_message(&mut self) {
        if let Some((_, timestamp)) = &self.temp_message
            && timestamp.elapsed() > std::time::Duration::from_secs(2)
        {
            self.temp_message = None;
        }
    }

    /// Force clear temporary message.
    #[allow(dead_code)]
    pub fn clear_temp_message(&mut self) {
        self.temp_message = None;
    }

    /// Replace the underlying items while preserving the current query and match settings.
    #[allow(dead_code)]
    pub fn set_items(&mut self, items: Vec<Item>) {
        self.hidden = items;
        self.shown.clear();
        self.selected = None;
        self.scroll_offset = 0;
        self.reset_cclip_content();
        self.filter();
    }

    fn reset_cclip_content(&mut self) {
        self.content_cache.clear();
        self.content_requests.clear();
        self.content_failures.clear();
        self.content_generation = self.content_generation.wrapping_add(1);
    }

    fn temp_message_text(&self) -> Option<&str> {
        self.temp_message
            .as_ref()
            .map(|(message, _)| message.as_str())
    }
}
