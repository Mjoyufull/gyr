//! Cclip mode - main event loop and TUI.

use crate::cli::Opts;
use crate::ui::DmenuUI;
use eyre::{Result, WrapErr};
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;
use ratatui::widgets::ListState;
use scopeguard::defer;
use std::io;

use super::commands::{handle_noninteractive_mode, load_history};
use super::items::build_items;
use super::state::CclipOptions;

/// Run cclip mode - async TUI event loop for clipboard history.
pub async fn run(cli: &Opts) -> Result<()> {
    if handle_noninteractive_mode(cli)? {
        return Ok(());
    }

    let cclip_items = load_history(cli)?;
    if cclip_items.is_empty() {
        if let Some(tag_name) = &cli.cclip_tag {
            println!("No clipboard items with tag '{}'", tag_name);
        } else {
            println!("No clipboard history available");
        }
        return Ok(());
    }

    let mut options = CclipOptions::from_cli(cli);
    let (db, _) = crate::core::database::open_history_db()?;
    let mut tag_metadata_map = super::load_tag_metadata(&db);
    let mut tag_metadata_formatter = super::TagMetadataFormatter::new(tag_metadata_map.clone());

    let items = build_items(
        cclip_items,
        &tag_metadata_formatter,
        options.show_line_numbers,
        options.show_tag_color_names,
    );

    crate::ui::terminal::setup_terminal(options.disable_mouse)?;
    let disable_mouse = options.disable_mouse;
    defer! {
        let _ = crate::ui::terminal::shutdown_terminal(disable_mouse);
    }

    let backend = CrosstermBackend::new(io::stderr());
    let mut terminal = Terminal::new(backend).wrap_err("Failed to start crossterm terminal")?;
    terminal.hide_cursor().wrap_err("Failed to hide cursor")?;
    crate::ui::terminal::clear_fullscreen(&mut terminal).wrap_err("Failed to clear terminal")?;

    let mut ui = DmenuUI::new(items, options.wrap_long_lines, options.show_line_numbers);
    if let Some(search) = &cli.search_string {
        ui.query = search.clone();
        ui.filter();
    }
    if !ui.shown.is_empty() && ui.selected.is_none() {
        ui.selected = Some(0);
    }

    let mut image_runtime = super::image::ImageRuntime::new(&options, &mut ui).await;
    options.set_graphics_adapter(image_runtime.detected_adapter());
    let mut list_state = ListState::default();
    let mut max_visible = 0usize;
    let mut needs_redraw = true;

    if needs_redraw {
        ui.clear_expired_message();
        image_runtime.prepare_for_draw(&ui).await;
        max_visible = super::render::draw(
            &mut terminal,
            &mut ui,
            &options,
            &tag_metadata_formatter,
            &mut list_state,
            &mut image_runtime,
        )?;
        needs_redraw = false;
    }
    if image_runtime.detect_stdio_picker_for_selection(&mut ui) {
        options.set_graphics_adapter(image_runtime.detected_adapter());
        needs_redraw = true;
    }

    let mut input = options.input_config().init_async();

    loop {
        if needs_redraw {
            ui.clear_expired_message();
            image_runtime.prepare_for_draw(&ui).await;
            max_visible = super::render::draw(
                &mut terminal,
                &mut ui,
                &options,
                &tag_metadata_formatter,
                &mut list_state,
                &mut image_runtime,
            )?;
        }

        tokio::select! {
            Some(_) = image_runtime.redraw_rx.recv() => {
                needs_redraw = true;
            }
            maybe_event = input.next() => {
                let Some(event) = maybe_event else {
                    return Ok(());
                };
                let outcome = super::events::handle_event(
                    super::events::EventContext {
                        ui: &mut ui,
                        terminal: &mut terminal,
                        cli,
                        options: &options,
                        db: &db,
                        tag_metadata_map: &mut tag_metadata_map,
                        tag_metadata_formatter: &mut tag_metadata_formatter,
                        image_runtime: &mut image_runtime,
                        max_visible,
                    },
                    event,
                    &mut input,
                )
                .await?;

                needs_redraw = outcome.needs_redraw;
                if matches!(outcome.control, super::events::LoopControl::Exit) {
                    return Ok(());
                }
                if image_runtime.needs_stdio_picker_detection_for_selection(&ui) {
                    input.shutdown().await;
                    if image_runtime.detect_stdio_picker_for_selection(&mut ui) {
                        options.set_graphics_adapter(image_runtime.detected_adapter());
                        needs_redraw = true;
                    }
                    input = options.input_config().init_async();
                }
            }
        }
    }
}
