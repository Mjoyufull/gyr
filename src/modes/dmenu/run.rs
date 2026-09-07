//! Dmenu compatibility mode

use crate::cli::Opts;
use crate::ui::{DmenuUI, InputEvent as Event};
use eyre::{Result, WrapErr};

use ratatui::backend::CrosstermBackend;
use ratatui::widgets::ListState;
use scopeguard::defer;
use std::cell::Cell;
use std::io;

use super::events::{LoopOutcome, handle_key_event, handle_mouse_event};
use super::options::DmenuOptions;
use super::preview::PreviewRuntime;
use super::render::draw_frame;

/// Run dmenu mode
pub async fn run(cli: &Opts) -> Result<()> {
    use ratatui::Terminal;
    use ratatui::backend::CrosstermBackend;

    // Check if stdin is piped (unless prompt-only mode)
    if !cli.dmenu_prompt_only && !super::parse::is_stdin_piped() {
        return Err(eyre::eyre!("dmenu mode requires input from stdin"));
    }

    // Read stdin lines
    let lines = if cli.dmenu_prompt_only {
        vec![] // No input in prompt-only mode
    } else if cli.dmenu_null_separated {
        super::parse::read_stdin_null_separated().wrap_err("Failed to read from stdin")?
    } else {
        super::parse::read_stdin_lines().wrap_err("Failed to read from stdin")?
    };

    // Exit immediately if no input and exit_if_empty is set
    if cli.dmenu_exit_if_empty && lines.is_empty() {
        return Ok(());
    }

    // Also check if lines only contain empty strings
    if cli.dmenu_exit_if_empty && lines.iter().all(|l| l.trim().is_empty()) {
        return Ok(());
    }

    // Parse items
    let items = super::parse::parse_stdin_to_items(
        lines,
        &cli.dmenu_delimiter,
        cli.dmenu_with_nth.as_ref(),
    );

    let options = DmenuOptions::from_cli(cli);
    crate::ui::terminal::setup_terminal(options.disable_mouse)?;
    let terminal_active = Cell::new(true);
    defer! {
        if terminal_active.get() {
            let _ = crate::ui::terminal::shutdown_terminal(options.disable_mouse);
        }
    }

    let backend = CrosstermBackend::new(io::stderr());
    let mut terminal = Terminal::new(backend).wrap_err("Failed to start crossterm terminal")?;
    terminal.hide_cursor().wrap_err("Failed to hide cursor")?;
    terminal.clear().wrap_err("Failed to clear terminal")?;

    let mut input = options.input_config().init_async();
    let mut ui = build_ui(cli, items, options.highlight_color);
    let mut list_state = ListState::default();
    let mut preview = PreviewRuntime::new(
        options.preview_command.clone(),
        options.graphics_adapter,
        !options.password_mode,
    );
    preview.request_if_changed(&ui);
    let mut needs_redraw = true;

    let outcome = loop {
        if needs_redraw {
            sync_update_mode(options.term_is_foot, true);
            let frame_result = (|| -> Result<()> {
                for _ in 0..2 {
                    if preview.needs_terminal_clear() {
                        terminal.clear()?;
                        preview.finish_draw();
                    }
                    let mut render_result = Ok(());
                    terminal.draw(|frame| {
                        render_result =
                            draw_frame(frame, &mut ui, &mut list_state, &options, &mut preview);
                    })?;
                    render_result?;
                    if !preview.needs_terminal_clear() {
                        break;
                    }
                }
                Ok(())
            })();
            sync_update_mode(options.term_is_foot, false);
            frame_result?;
            preview.finish_draw();
            needs_redraw = false;
        }

        tokio::select! {
            Some(result) = preview.next_result() => {
                preview.apply_result(result);
                needs_redraw = true;
            }
            maybe_event = input.next() => {
                let Some(event) = maybe_event else {
                    break LoopOutcome::Exit;
                };
                let event_outcome = match event {
                    Event::Input(key) => {
                        needs_redraw = true;
                        handle_key_event(&mut ui, key, &options, terminal.size()?.height)
                    }
                    Event::Mouse(mouse_event) => {
                        needs_redraw = true;
                        handle_mouse_event(
                            &mut ui,
                            mouse_event,
                            &options,
                            terminal.size()?.height,
                        )
                    }
                    Event::Render => {
                        needs_redraw = true;
                        LoopOutcome::Continue
                    }
                    Event::Tick => LoopOutcome::Continue,
                };

                match event_outcome {
                    LoopOutcome::Continue => preview.request_if_changed(&ui),
                    LoopOutcome::Exit => break LoopOutcome::Exit,
                    LoopOutcome::Print(output) => break LoopOutcome::Print(output),
                }
            }
        }
    };

    prepare_terminal_for_output(&mut terminal)?;
    crate::ui::terminal::shutdown_terminal(options.disable_mouse)
        .wrap_err("Failed to restore dmenu terminal state")?;
    terminal_active.set(false);

    if let LoopOutcome::Print(output) = outcome {
        println!("{output}");
    }
    Ok(())
}

fn build_ui<'a>(
    cli: &Opts,
    items: Vec<crate::common::Item>,
    highlight_color: ratatui::style::Color,
) -> DmenuUI<'a> {
    let mut ui = DmenuUI::new(
        items,
        cli.dmenu_wrap_long_lines,
        cli.dmenu_show_line_numbers,
    );
    ui.set_match_mode(cli.match_mode);
    ui.set_match_nth(cli.dmenu_match_nth.clone());

    if let Some(ref search) = cli.search_string {
        ui.query = search.clone();
    }

    ui.filter();

    if let Some(ref select_str) = cli.dmenu_select {
        let select_lower = select_str.to_lowercase();
        for (idx, item) in ui.shown.iter().enumerate() {
            if item.display_text.to_lowercase().contains(&select_lower) {
                ui.selected = Some(idx);
                break;
            }
        }
    } else if let Some(select_idx) = cli.dmenu_select_index
        && select_idx < ui.shown.len()
    {
        ui.selected = Some(select_idx);
    }

    if !ui.shown.is_empty() && ui.selected.is_none() {
        ui.selected = Some(0);
    }

    ui.info(highlight_color);
    ui
}

fn prepare_terminal_for_output(
    terminal: &mut ratatui::Terminal<CrosstermBackend<io::Stderr>>,
) -> Result<()> {
    terminal.show_cursor().wrap_err("Failed to show cursor")?;
    Ok(())
}

fn sync_update_mode(term_is_foot: bool, enable: bool) {
    if !term_is_foot {
        return;
    }

    let sequence = if enable {
        b"\x1b[?2026h"
    } else {
        b"\x1b[?2026l"
    };
    let mut stderr = std::io::stderr();
    let _ = std::io::Write::write_all(&mut stderr, sequence);
    let _ = std::io::Write::flush(&mut stderr);
}
