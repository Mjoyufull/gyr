mod fullscreen;
mod loading;

use super::state::CclipOptions;
use crate::ui::{DISPLAY_STATE, DisplayState, DmenuUI, ImageManager, TagMode};
use eyre::Result;
use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui_image::picker::Picker;
use std::collections::HashSet;
use std::sync::Arc;
use tokio::sync::{Mutex, mpsc};

pub(super) struct ImageRuntime {
    image_manager: Option<Arc<Mutex<ImageManager>>>,
    failed_rowids: Arc<Mutex<HashSet<String>>>,
    redraw_tx: mpsc::UnboundedSender<()>,
    pub(super) redraw_rx: mpsc::UnboundedReceiver<()>,
    image_preview_enabled: bool,
    image_preview_allowed: bool,
    image_preview_forced: bool,
    stdio_picker_detection_attempted: bool,
    cached_is_sixel: bool,
    detected_adapter: crate::ui::GraphicsAdapter,
    previous_was_image: bool,
    current_is_image: bool,
    current_rowid: Option<String>,
    force_buffer_sync: bool,
}

impl ImageRuntime {
    pub(super) async fn new(options: &CclipOptions, ui: &mut DmenuUI<'_>) -> Self {
        let image_preview_allowed = options.explicit_image_preview != Some(false);
        let image_preview_forced = options.explicit_image_preview == Some(true);
        let detected_adapter = if image_preview_allowed {
            crate::ui::GraphicsAdapter::detect(None)
        } else {
            crate::ui::GraphicsAdapter::None
        };
        let supports_graphics = !matches!(detected_adapter, crate::ui::GraphicsAdapter::None);
        let image_preview_enabled =
            image_preview_allowed && options.image_preview_enabled(supports_graphics);
        let image_manager = image_preview_enabled
            .then(|| Arc::new(Mutex::new(ImageManager::new(detected_adapter.picker()))));
        let failed_rowids = Arc::new(Mutex::new(HashSet::<String>::new()));
        let (redraw_tx, redraw_rx) = mpsc::unbounded_channel::<()>();

        let cached_is_sixel = matches!(detected_adapter, crate::ui::GraphicsAdapter::Sixel);

        if matches!(detected_adapter, crate::ui::GraphicsAdapter::None) && image_preview_enabled {
            ui.set_temp_message(
                "image_preview enabled but terminal graphics detection found no high-resolution protocol (using half-block fallback)".to_string(),
            );
        }

        Self {
            image_manager,
            failed_rowids,
            redraw_tx,
            redraw_rx,
            image_preview_enabled,
            image_preview_allowed,
            image_preview_forced,
            stdio_picker_detection_attempted: false,
            cached_is_sixel,
            detected_adapter,
            previous_was_image: false,
            current_is_image: false,
            current_rowid: None,
            force_buffer_sync: false,
        }
    }

    pub(super) fn preview_enabled(&self) -> bool {
        self.image_preview_enabled
    }

    pub(super) fn detected_adapter(&self) -> crate::ui::GraphicsAdapter {
        self.detected_adapter
    }

    pub(super) fn current_is_image(&self) -> bool {
        self.current_is_image
    }

    pub(super) fn needs_stdio_picker_detection_for_selection(&self, ui: &DmenuUI<'_>) -> bool {
        self.image_preview_allowed
            && !self.stdio_picker_detection_attempted
            && selected_image_rowid(ui).is_some()
    }

    pub(super) fn detect_stdio_picker_for_selection(&mut self, ui: &mut DmenuUI<'_>) -> bool {
        if !self.needs_stdio_picker_detection_for_selection(ui) {
            return false;
        }

        self.stdio_picker_detection_attempted = true;
        let Ok(picker) = Picker::from_query_stdio() else {
            return false;
        };

        let detected_adapter = crate::ui::GraphicsAdapter::detect(Some(&picker));
        let supports_graphics = !matches!(detected_adapter, crate::ui::GraphicsAdapter::None);
        self.image_preview_enabled = self.image_preview_forced || supports_graphics;
        self.cached_is_sixel = matches!(detected_adapter, crate::ui::GraphicsAdapter::Sixel);
        self.detected_adapter = detected_adapter;
        self.image_manager = self
            .image_preview_enabled
            .then(|| Arc::new(Mutex::new(ImageManager::new(picker))));
        self.reset_display_state_for_manager_replacement();
        self.force_buffer_sync = true;

        if self.image_preview_forced && !supports_graphics {
            ui.set_temp_message(
                "image_preview enabled but terminal graphics detection found no high-resolution protocol (using half-block fallback)".to_string(),
            );
        }

        true
    }

    pub(super) fn request_buffer_sync(&mut self) {
        self.force_buffer_sync = true;
    }

    pub(super) fn consume_buffer_sync(&mut self) -> bool {
        let force_buffer_sync = self.force_buffer_sync;
        self.force_buffer_sync = false;
        force_buffer_sync
    }

    pub(super) fn needs_terminal_clear(&self) -> bool {
        self.image_preview_enabled
            && self.cached_is_sixel
            && self.previous_was_image != self.current_is_image
    }

    pub(super) fn finish_draw(&mut self) {
        self.previous_was_image = self.current_is_image;
    }

    pub(super) fn clear_inline_image(&mut self) {
        if let Some(manager) = &mut self.image_manager
            && let Ok(mut manager_lock) = manager.try_lock()
        {
            manager_lock.clear();
        }
        if let Ok(mut failed) = self.failed_rowids.try_lock() {
            failed.clear();
        }
        self.current_is_image = false;
        self.current_rowid = None;
    }

    pub(super) async fn prepare_for_draw(&mut self, ui: &DmenuUI<'_>) {
        self.current_is_image = false;
        self.current_rowid = None;

        if self.image_preview_enabled
            && matches!(ui.tag_mode, TagMode::Normal)
            && let Some(selected) = ui.selected
            && selected < ui.shown.len()
        {
            let item = &ui.shown[selected];
            if ui.is_cclip_image_item(item) {
                self.current_is_image = true;
                self.current_rowid = ui.get_cclip_rowid(item);
            }
        }

        if !self.image_preview_enabled {
            return;
        }

        if self.current_is_image {
            self.ensure_image_loaded().await;
        } else if self.previous_was_image {
            self.clear_inline_image();
        }
    }

    pub(super) fn render_inline_image(&mut self, frame: &mut Frame, area: Rect) -> Result<bool> {
        if !self.current_is_image {
            return Ok(false);
        }

        if let Some(manager) = &mut self.image_manager
            && let Ok(mut manager_lock) = manager.try_lock()
        {
            manager_lock.render(frame, area)?;
            return Ok(true);
        }

        Ok(false)
    }

    fn restore_display_state(&self) {
        let mut state = DISPLAY_STATE
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        if let Some(rowid) = &self.current_rowid {
            let is_cached = self
                .image_manager
                .as_ref()
                .and_then(|manager| manager.try_lock().ok())
                .is_some_and(|manager| manager.is_cached(rowid));

            if is_cached {
                *state = DisplayState::Image(rowid.clone());
            }
        } else {
            *state = DisplayState::Empty;
        }
    }

    fn reset_display_state_for_manager_replacement(&self) {
        let mut state = DISPLAY_STATE
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        *state = DisplayState::Empty;
    }
}

fn selected_image_rowid(ui: &DmenuUI<'_>) -> Option<String> {
    let selected = ui.selected?;
    let item = ui.shown.get(selected)?;
    ui.is_cclip_image_item(item)
        .then(|| ui.get_cclip_rowid(item))?
}

#[cfg(test)]
mod tests {
    use super::ImageRuntime;
    use crate::cli::PanelPosition;
    use crate::common::Item;
    use crate::ui::{DISPLAY_STATE, DisplayState, DmenuUI, GraphicsAdapter};
    use ratatui::style::Color;
    use std::collections::HashSet;
    use std::sync::Arc;
    use tokio::sync::{Mutex, mpsc};

    fn options_with_image_preview(
        explicit_image_preview: Option<bool>,
    ) -> super::super::state::CclipOptions {
        super::super::state::CclipOptions {
            disable_mouse: false,
            hard_stop: false,
            wrap_long_lines: true,
            show_line_numbers: false,
            show_tag_color_names: false,
            hide_image_message: false,
            highlight_color: Color::LightBlue,
            main_border_color: Color::White,
            items_border_color: Color::White,
            input_border_color: Color::White,
            main_text_color: Color::White,
            items_text_color: Color::White,
            input_text_color: Color::White,
            header_title_color: Color::White,
            rounded_borders: true,
            content_panel_height_percent: 30,
            input_panel_height: 3,
            content_panel_position: PanelPosition::Top,
            cursor: String::new(),
            term_is_foot: false,
            graphics_adapter: GraphicsAdapter::None,
            explicit_image_preview,
        }
    }

    fn cclip_item(rowid: &str, mime_type: &str, preview: &str) -> Item {
        Item::new_simple(
            format!("{rowid}\t{mime_type}\t{preview}"),
            preview.to_string(),
            rowid.parse().expect("rowid should be numeric"),
        )
    }

    #[tokio::test]
    async fn new_skips_image_manager_when_preview_is_explicitly_disabled() {
        let options = options_with_image_preview(Some(false));
        let mut ui = DmenuUI::new(Vec::new(), true, false);

        let runtime = ImageRuntime::new(&options, &mut ui).await;

        assert!(!runtime.preview_enabled());
        assert!(runtime.image_manager.is_none());
        assert_eq!(runtime.detected_adapter(), GraphicsAdapter::None);
    }

    #[tokio::test]
    async fn picker_detection_is_needed_for_selected_image_items() {
        let options = options_with_image_preview(Some(true));
        let mut ui = DmenuUI::new(vec![cclip_item("1", "image/png", "image")], true, false);
        ui.selected = Some(0);
        let runtime = ImageRuntime::new(&options, &mut ui).await;

        assert!(runtime.needs_stdio_picker_detection_for_selection(&ui));
    }

    #[tokio::test]
    async fn picker_detection_is_skipped_for_selected_text_items() {
        let options = options_with_image_preview(Some(true));
        let mut ui = DmenuUI::new(vec![cclip_item("1", "text/plain", "text")], true, false);
        ui.selected = Some(0);
        let runtime = ImageRuntime::new(&options, &mut ui).await;

        assert!(!runtime.needs_stdio_picker_detection_for_selection(&ui));
    }

    #[test]
    fn manager_replacement_resets_display_state_for_current_image() {
        let (redraw_tx, redraw_rx) = mpsc::unbounded_channel();
        let runtime = ImageRuntime {
            image_manager: None,
            failed_rowids: Arc::new(Mutex::new(HashSet::new())),
            redraw_tx,
            redraw_rx,
            image_preview_enabled: true,
            image_preview_allowed: true,
            image_preview_forced: false,
            stdio_picker_detection_attempted: false,
            cached_is_sixel: false,
            detected_adapter: GraphicsAdapter::None,
            previous_was_image: false,
            current_is_image: true,
            current_rowid: Some("42".to_string()),
            force_buffer_sync: false,
        };

        {
            let mut state = DISPLAY_STATE
                .lock()
                .unwrap_or_else(|error| error.into_inner());
            *state = DisplayState::Loading("42".to_string());
        }

        runtime.reset_display_state_for_manager_replacement();

        let state = DISPLAY_STATE
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        assert_eq!(*state, DisplayState::Empty);
    }

    #[test]
    fn restore_display_state_keeps_loading_state_for_uncached_selection() {
        let (redraw_tx, redraw_rx) = mpsc::unbounded_channel();
        let runtime = ImageRuntime {
            image_manager: None,
            failed_rowids: Arc::new(Mutex::new(HashSet::new())),
            redraw_tx,
            redraw_rx,
            image_preview_enabled: true,
            image_preview_allowed: true,
            image_preview_forced: false,
            stdio_picker_detection_attempted: false,
            cached_is_sixel: false,
            detected_adapter: GraphicsAdapter::None,
            previous_was_image: false,
            current_is_image: true,
            current_rowid: Some("42".to_string()),
            force_buffer_sync: false,
        };

        {
            let mut state = DISPLAY_STATE
                .lock()
                .unwrap_or_else(|error| error.into_inner());
            *state = DisplayState::Loading("42".to_string());
        }

        runtime.restore_display_state();

        let state = DISPLAY_STATE
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        assert_eq!(*state, DisplayState::Loading("42".to_string()));
    }
}
